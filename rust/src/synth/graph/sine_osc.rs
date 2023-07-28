use crate::graph::*;
use core::f32::consts;

#[derive(Default, Clone)]
pub struct SineOsc {
    // input ports
    pub freq: f32,

    // internal
    pub phase: f32,

    // output ports
    pub value: f32,
}

impl Node for SineOsc {
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
        "Sine oscillator"
    }
    fn inputs(&self) -> Vec<Input> {
        Vec::from([(0, "freq")])
    }
    fn outputs(&self) -> Vec<Output> {
        Vec::from([(0, "V")])
    }

    // Set input at index idx to value val
    fn set(&mut self, idx: usize, val: f32) {
        valid_idx!(self.freq = val, idx, 1);
    }

    // Get value of output index idx
    fn get(&self, idx: usize) -> f32 {
        valid_idx!(self.value, idx, 1)
    }

    fn step(&mut self, sample_rate: f32) {
        self.value = f32::sin(2.0 * consts::PI * self.phase);
        self.phase += (self.freq * 1000.0) / sample_rate;
        self.phase = self.phase % 1.0;
    }
}
