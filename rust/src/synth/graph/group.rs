use crate::graph::*;

// use lolmacros::Wiretap;

#[derive(Default, Clone)]
pub struct Group {}

impl Node for Group {
    fn copy(&self) -> Box<dyn Node> {
        let c = (*self).clone();
        Box::new(c)
    }
    fn inputs(&self) -> Vec<Input> {
        Vec::from([])
    }
    fn outputs(&self) -> Vec<Output> {
        Vec::from([])
    }

    // Set input at index idx to value val
    fn set(&mut self, _idx: usize, _val: f32) {}

    // Get value of output index idx
    fn get(&self, _idx: usize) -> f32 {
        0.0
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
