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
    fn inputs(&self) -> Vec<InputId> {
        vec![(0, "input"), (1, "shift")]
            .into_iter()
            .map(|t| t.into())
            .collect()
    }
    fn outputs(&self) -> Vec<OutputId> {
        vec![(0, "value")].into_iter().map(|t| t.into()).collect()
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

    fn get_input_mut(&mut self, idx: usize) -> &mut f32 {
        match idx {
            0 => &mut self.input,
            1 => &mut self.shift,
            _ => panic!("Invalid input id"),
        }
    }

    fn step(&mut self, _sample_rate: f32) {
        self.value = self.input + self.shift
    }
}
