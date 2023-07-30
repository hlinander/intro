use crate::graph::*;
use serde::{Deserialize, Serialize};

// use lolmacros::Wiretap;

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Group {}

#[typetag::serde]
impl Node for Group {
    fn copy(&self) -> Box<dyn Node> {
        let c = (*self).clone();
        Box::new(c)
    }
    fn inputs(&self) -> Vec<InputId> {
        Vec::new()
    }
    fn outputs(&self) -> Vec<OutputId> {
        Vec::new()
    }

    // Set input at index idx to value val
    fn set(&mut self, _idx: usize, _val: f32) {}

    // Get value of output index idx
    fn get(&self, _idx: usize) -> f32 {
        0.0
    }
    fn get_input_mut(&mut self, idx: usize) -> &mut f32 {
        panic!();
    }

    fn step(&mut self, _sample_rate: f32) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn name() -> &'static str {
        "Group"
    }
}
