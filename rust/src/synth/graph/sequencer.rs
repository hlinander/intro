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
    // pub tempo: f32,
    pub trigger_in: f32,

    // internal
    pub beat: usize,
    prev_trigger_in: f32,

    // output ports
    pub trigger: f32,
    pub pitch: f32,

    // Vec<(triggered, tone)>
    pub sequence: Vec<Note>,
}

impl Default for Sequencer {
    fn default() -> Self {
        Self {
            // tempo: 1.0,
            trigger_in: 0.0,
            prev_trigger_in: 0.0,
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
        vec![(0, "trigger_in")]
            .into_iter()
            .map(|t| t.into())
            .collect()
    }
    fn outputs(&self) -> Vec<OutputId> {
        vec![(0, "trigger"), (1, "pitch")]
            .into_iter()
            .map(|t| t.into())
            .collect()
    }

    fn read_input(&self, idx: usize) -> f32 {
        match idx {
            // 0 => self.tempo,
            0 => self.trigger_in,
            _ => panic!("Invalid idx"),
        }
    }

    // Set input at index idx to value val
    fn set(&mut self, idx: usize, val: f32) {
        match idx {
            // 0 => self.tempo = val,
            0 => self.trigger_in = val,
            _ => panic!("Invalid idx"),
        }
    }

    fn get_input_mut(&mut self, idx: usize) -> &mut f32 {
        match idx {
            // 0 => &mut self.tempo,
            0 => &mut self.trigger_in,
            _ => panic!("Invalid input id"),
        }
    }

    // Get value of output index idx
    fn get(&self, idx: usize) -> f32 {
        match idx {
            0 => self.trigger,
            1 => self.pitch,
            _ => panic!("unknown output"),
        }
    }

    fn step(&mut self, _sample_rate: f32) {
        // tempo is fraction of 200bpm
        // let bpm = self.tempo * 400.0;
        // 60 seconds per minute / bpm
        // let beat_length = 60.0 / bpm;

        // self.phase += 1.0 / sample_rate;
        // let local_phase = self.phase % (self.sequence.len() as f32 * beat_length);
        // if self.phase > self.sequence.len() as f32 * beat_length {
        // self.phase = 0.0;
        // }

        // let beat_idx = (local_phase / beat_length).floor() as usize;
        // self.beat = beat_idx;
        if self.trigger_in > 0.0 && self.prev_trigger_in <= 0.0 {
            self.beat += 1;
            if self.beat >= self.sequence.len() {
                self.beat = 0;
            }
        }
        self.prev_trigger_in = self.trigger_in;
        self.trigger = if self.sequence[self.beat].active {
            1.0
        } else {
            0.0
        };
        self.pitch = tone_to_khz(
            (self.sequence[self.beat].pitch as i32 + 12 * (self.sequence[self.beat].octave - 4) - 9)
                as f32,
        );
    }
}

pub fn tone_to_khz(x: f32) -> f32 {
    0.44 * 2.0_f32.powf(x / 12.0)
}
