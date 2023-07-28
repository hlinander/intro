use crate::graph::*;

#[derive(Default, Clone)]
pub struct Key {
    // output ports
    pub pitch: f32,
    pub trigger: f32,
    pub buff: VecDeque<f32>,
}

impl Node for Key {
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
        "Key"
    }
    fn inputs(&self) -> Vec<Input> {
        Vec::from([])
    }
    fn outputs(&self) -> Vec<Output> {
        Vec::from([(0, "pitch"), (1, "trigger")])
    }

    // Set input at index idx to value val
    fn set(&mut self, _idx: usize, _val: f32) {}

    // Get value of output index idx
    fn get(&self, idx: usize) -> f32 {
        match idx {
            0 => self.pitch,
            1 => self.trigger,
            _ => panic!("Invalid output id"),
        }
    }

    fn step(&mut self, _sample_rate: f32) {
        if self.buff.is_empty() {
            self.buff.push_front(self.trigger);
        } else {
            self.buff[0] = self.trigger;
        }
    }

    fn buff(&self) -> Option<&VecDeque<f32>> {
        None
        //Some(&self.buff)
    }
}
