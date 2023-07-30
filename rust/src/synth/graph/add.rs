use crate::graph::*;
use serde::{Deserialize, Serialize};

// use lolmacros::Wiretap;

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Add {
    // input ports
    pub i1: f32,
    pub i2: f32,
    pub i3: f32,
    pub i4: f32,

    // output ports
    pub value: f32,
}

#[typetag::serde]
impl Node for Add {
    fn copy(&self) -> Box<dyn Node> {
        let c = (*self).clone();
        Box::new(c)
    }
    fn inputs(&self) -> Vec<InputId> {
        [(0, "i1"), (1, "i2"), (2, "i3"), (3, "i4")]
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
            0 => self.i1 = val,
            1 => self.i2 = val,
            2 => self.i3 = val,
            3 => self.i4 = val,
            _ => panic!("Invalid input id"),
        }
    }

    // Get value of output index idx
    fn get(&self, idx: usize) -> f32 {
        valid_idx!(self.value, idx, 1)
    }

    fn get_input_mut(&mut self, idx: usize) -> &mut f32 {
        match idx {
            0 => &mut self.i1,
            1 => &mut self.i2,
            2 => &mut self.i3,
            3 => &mut self.i4,
            _ => panic!("Invalid input id"),
        }
    }

    fn step(&mut self, _sample_rate: f32) {
        self.value = self.i1 + self.i2 + self.i3 + self.i4;
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn name() -> &'static str {
        "Add"
    }
}
