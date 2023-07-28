use crate::graph::*;

#[derive(Default, Clone)]
pub struct Scale {
    // input ports
    pub input: f32,
    pub scale: f32,

    // output ports
    pub value: f32,
}

impl Node for Scale {
    fn copy(&self) -> Box<dyn Node> {
        let c = (*self).clone();
        Box::new(c)
    }
    fn name() -> &'static str
    where
        Self: Sized,
    {
        "Scale"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn inputs(&self) -> Vec<Input> {
        Vec::from([(0, "input"), (1, "scale")])
    }
    fn outputs(&self) -> Vec<Output> {
        Vec::from([(0, "value")])
    }

    // Set input at index idx to value val
    fn set(&mut self, idx: usize, val: f32) {
        match idx {
            0 => self.input = val,
            1 => self.scale = val,
            _ => panic!("Invalid input id"),
        }
    }

    // Get value of output index idx
    fn get(&self, idx: usize) -> f32 {
        valid_idx!(self.value, idx, 1)
    }

    fn step(&mut self, _sample_rate: f32) {
        self.value = self.input * self.scale
    }
}
