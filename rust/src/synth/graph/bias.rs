use crate::graph::*;
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Bias {
    // input ports
    pub input: f32,
    pub shift: f32,

    // output ports
    pub value: f32,
}

#[typetag::serde]
impl Node for Bias {
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
        "Bias"
    }
    fn inputs(&self) -> Vec<Input> {
        vec![(0, "input"), (1, "shift")]
    }
    fn outputs(&self) -> Vec<Output> {
        vec![(0, "value")]
    }

    // Set input at index idx to value val
    fn set(&mut self, idx: usize, val: f32) {
        match idx {
            0 => self.input = val,
            1 => self.shift = val,
            _ => panic!("Invalid input id"),
        }
    }

    // Get value of output index idx
    fn get(&self, idx: usize) -> f32 {
        valid_idx!(self.value, idx, 1)
    }

    fn step(&mut self, _sample_rate: f32) {
        self.value = self.input + self.shift
    }
}
