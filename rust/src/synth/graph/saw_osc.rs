use crate::graph::*;
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct SawOsc {
    // input ports
    pub freq: f32,

    // internal
    pub phase: f32,

    // output ports
    pub value: f32,
}

#[typetag::serde]
impl Node for SawOsc {
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
        "Saw Oscillator"
    }
    fn inputs(&self) -> Vec<Input> {
        vec![(0, "freq")]
    }
    fn outputs(&self) -> Vec<Output> {
        vec![(0, "V")]
    }

    // Set input at index idx to value val
    fn set(&mut self, idx: usize, val: f32) {
        valid_idx!(self.freq = val, idx, 1);
    }

    fn get_input_mut(&mut self, idx: usize) -> &mut f32 {
        valid_idx!(&mut self.freq, idx, 1)
    }

    // Get value of output index idx
    fn get(&self, idx: usize) -> f32 {
        valid_idx!(self.value, idx, 1)
    }

    fn step(&mut self, sample_rate: f32) {
        self.phase += (self.freq * 1000.0) / sample_rate;
        self.phase = self.phase % 1.0;
        self.value = 0.5 * (self.phase - 0.5);
    }
}
