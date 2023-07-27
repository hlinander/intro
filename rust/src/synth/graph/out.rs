use crate::graph::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Out {
    pub value: f32,

    // Oscilloscope
    pub prev_value: f32,
    pub buffer: VecDeque<f32>,
    pub triggered: bool,
    pub buffer_index: usize,
}

impl Default for Out {
    fn default() -> Self {
        Self {
            value: 0.0,
            prev_value: 0.0,
            buffer: VecDeque::from(vec![0.0; 1000]),
            buffer_index: 0,
            triggered: false,
        }
    }
}

///////////////////////////////////////////////////////////////////////
// unsafe impl Send for Out {}
// unsafe impl Sync for Out {}
///////////////////////////////////////////////////////////////////////

#[typetag::serde]
impl Node for Out {
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
        "Out"
    }
    fn inputs(&self) -> Vec<Input> {
        vec![(0, "value")]
    }
    fn outputs(&self) -> Vec<Output> {
        vec![(0, "value")]
    }

    // Set input at index idx to value val
    fn set(&mut self, idx: usize, val: f32) {
        valid_idx!(self.value = val, idx, 1);
        //println!("Set {}", val);
    }

    // Get value of output index idx
    fn get(&self, idx: usize) -> f32 {
        let out = self.value; //compress(self.value);
                              //let out = self.value;
        valid_idx!(out, idx, 1)
    }

    fn step(&mut self, _sample_rate: f32) {
        if self.prev_value < 0.0 && self.value > 0.0 && !self.triggered {
            self.triggered = true;
            self.buffer_index = 0;
        }
        if self.triggered && self.buffer_index < self.buffer.len() {
            self.buffer[self.buffer_index] = self.value;
            self.buffer_index += 1;
        }
        if self.buffer_index > self.buffer.len() - 1 {
            self.buffer_index = 0;
            self.triggered = false;
        }
        self.prev_value = self.value;
    }
    fn buff(&self) -> Option<&VecDeque<f32>> {
        Some(&self.buffer)
        //VecDeque::from(vec![1.0, 2.0, 3.0])
    }
}
