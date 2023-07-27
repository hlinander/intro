use crate::graph::*;
use interp1d::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Envelope {
    // input ports
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
    pub trigger: f32,

    pub input: f32,

    // internal
    pub phase: f32,
    pub release_phase: f32,
    pub old_trigger: f32,

    // output ports
    pub value: f32,
}

impl Default for Envelope {
    fn default() -> Self {
        Self {
            attack: 0.1,
            decay: 0.1,
            release: 0.5,
            sustain: 0.3,
            phase: 0.0,
            release_phase: 0.0,
            input: 0.0,
            value: 0.0,
            trigger: 0.0,
            old_trigger: 0.0,
        }
    }
}

impl Envelope {
    fn env(&self) -> f32 {
        let t1: f32 = 0.0;
        let t2: f32 = self.attack;
        let t3: f32 = t2 + self.decay;
        let t4: f32 = t3 + 1.0;
        //let t5: f32 = t4 + self.release;
        //let x: Vec<f32> = vec![t1, t2, t3, t4, t5];
        //let y: Vec<f32> = vec![0.0, 1.0, self.sustain, self.sustain, 0.0];
        let x: Vec<f32> = vec![t1, t2, t3, t4];
        let y: Vec<f32> = vec![0.0, 1.0, self.sustain, self.sustain];

        let interpolator = Interp1d::new_sorted(x, y).unwrap();

        let xr: Vec<f32> = vec![0.0, self.release, self.release + 1.0];
        let yr: Vec<f32> = vec![1.0, 0.0, 0.0];

        let release_interpolator = Interp1d::new_sorted(xr, yr).unwrap();
        let release = release_interpolator.interpolate(self.release_phase);
        interpolator.interpolate(self.phase) * release
    }
}

#[typetag::serde]
impl Node for Envelope {
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
        "Envelope"
    }
    fn inputs(&self) -> Vec<Input> {
        vec![
            (0, "input"),
            (1, "attack"),
            (2, "decay"),
            (3, "sustain"),
            (4, "release"),
            (5, "trigger"),
        ]
    }
    fn outputs(&self) -> Vec<Output> {
        vec![(0, "V"), (1, "env")]
    }

    fn read_input(&self, idx: usize) -> f32 {
        match idx {
            0 => self.input,
            1 => self.attack,
            2 => self.decay,
            3 => self.sustain,
            4 => self.release,
            5 => self.trigger,
            _ => panic!("Invalid idx"),
        }
    }

    // Set input at index idx to value val
    fn set(&mut self, idx: usize, val: f32) {
        match idx {
            0 => self.input = val,
            1 => self.attack = val,
            2 => self.decay = val,
            3 => self.sustain = val,
            4 => self.release = val,
            5 => self.trigger = val,
            _ => panic!("Invalid idx"),
        }
    }

    // Get value of output index idx
    fn get(&self, idx: usize) -> f32 {
        match idx {
            0 => self.input * self.env(),
            1 => self.env(),
            _ => panic!("unknown output"),
        }
    }

    fn step(&mut self, sample_rate: f32) {
        //self.value = f32::sin(2.0 * consts::PI * self.phase);
        if self.trigger - self.old_trigger > 0.0 {
            self.phase = 0.0;
            self.release_phase = 0.0;
        }
        if self.trigger > 0.0 {
            self.phase += 1.0 / sample_rate;
        } else {
            self.release_phase += 1.0 / sample_rate;
        }
        self.old_trigger = self.trigger;
        // self.phase = self.phase % (2.0 * (self.attack + self.decay + 1.0 + self.release));
    }
}
