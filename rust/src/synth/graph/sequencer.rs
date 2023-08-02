use crate::graph::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Note {
    pub active: bool,
    pub pitch: u8,
    pub octave: i32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Sequencer {
    // input ports
    pub tempo: f32,
    pub phase: f32,

    // internal
    pub beat: usize,

    // output ports
    pub trigger: f32,
    pub pitch: f32,

    // Vec<(triggered, tone)>
    pub sequence: Vec<Note>,
}

impl Default for Sequencer {
    fn default() -> Self {
        Self {
            tempo: 1.0,
            phase: 0.0,
            beat: 0,
            trigger: 0.0,
            pitch: 1.0,
            sequence: vec![
                Note {
                    active: false,
                    pitch: 0,
                    octave: 4
                };
                8
            ],
        }
    }
}

#[typetag::serde]
impl Node for Sequencer {
    fn copy(&self) -> Box<dyn Node> {
        let c = (*self).clone();
        Box::new(c)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn name() -> &'static str {
        "Sequencer"
    }
    fn inputs(&self) -> Vec<InputId> {
        vec![(0, "tempo"), (1, "phase")]
            .into_iter()
            .map(|t| t.into())
            .collect()
    }
    fn outputs(&self) -> Vec<OutputId> {
        vec![(0, "trigger"), (1, "pitch"), (2, "phase")]
            .into_iter()
            .map(|t| t.into())
            .collect()
    }

    fn read_input(&self, idx: usize) -> f32 {
        match idx {
            0 => self.tempo,
            1 => self.phase,
            _ => panic!("Invalid idx"),
        }
    }

    // Set input at index idx to value val
    fn set(&mut self, idx: usize, val: f32) {
        match idx {
            0 => self.tempo = val,
            1 => self.phase = val,
            _ => panic!("Invalid idx"),
        }
    }

    fn get_input_mut(&mut self, idx: usize) -> &mut f32 {
        match idx {
            0 => &mut self.tempo,
            1 => &mut self.phase,
            _ => panic!("Invalid input id"),
        }
    }

    // Get value of output index idx
    fn get(&self, idx: usize) -> f32 {
        match idx {
            0 => self.trigger,
            1 => self.pitch,
            2 => self.phase * self.tempo,
            _ => panic!("unknown output"),
        }
    }

    fn step(&mut self, _sample_rate: f32) {
        // tempo is fraction of 200bpm
        let bpm = self.tempo * 400.0;
        // 60 seconds per minute / bpm
        let beat_length = 60.0 / bpm;

        // self.phase += 1.0 / sample_rate;
        let local_phase = self.phase % (self.sequence.len() as f32 * beat_length);
        // if self.phase > self.sequence.len() as f32 * beat_length {
        // self.phase = 0.0;
        // }

        let beat_idx = (local_phase / beat_length).floor() as usize;
        self.beat = beat_idx;
        self.trigger = if self.sequence[beat_idx].active {
            1.0
        } else {
            0.0
        };
        self.pitch = tone_to_khz(
            (self.sequence[beat_idx].pitch as i32 + 12 * (self.sequence[beat_idx].octave - 4) - 9)
                as f32,
        );
    }
}

pub fn tone_to_khz(x: f32) -> f32 {
    0.44 * 2.0_f32.powf(x / 12.0)
}
