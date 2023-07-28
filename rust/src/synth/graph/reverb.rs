use crate::graph::*;
use alloc::collections::VecDeque;

#[derive(Clone)]
pub struct Reverb {
    // input ports
    pub input: f32,

    channels: i32,
    pub delay: f32,
    pub damp: f32,

    // output ports
    pub value: f32,

    buffer: VecDeque<f32>,
}

impl Default for Reverb {
    fn default() -> Self {
        Self {
            input: 0.0,
            channels: 6,
            delay: 1.3,
            damp: 0.8,
            value: 0.0,
            buffer: VecDeque::new(),
        }
    }
}

impl Node for Reverb {
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
        "Reverb"
    }
    fn inputs(&self) -> Vec<Input> {
        Vec::from([(0, "input")])
    }
    fn outputs(&self) -> Vec<Output> {
        Vec::from([(0, "value")])
    }

    // Set input at index idx to value val
    fn set(&mut self, idx: usize, val: f32) {
        match idx {
            0 => self.input = val,
            _ => panic!("Invalid input id"),
        }
    }

    // Get value of output index idx
    fn get(&self, idx: usize) -> f32 {
        valid_idx!(self.value, idx, 1)
    }

    fn step(&mut self, sample_rate: f32) {
        let deltas: Vec<usize> = (0..self.channels)
            .map(|i| (sample_rate * self.delay / (i as f32)) as usize)
            .collect();
        /*
        for t in -max(delays)..0 {
            for d in delays {
                buff(t) += damp * buff(t - d)
            }
            [.........i........i+d1.........i+d2......]
            for d in delays {
                buff(t + d) += damp * buff(t)
            }
        }

        */
        let mut v: f32 = self.input;
        for d in deltas {
            if d < self.buffer.len() {
                v += self.damp / (self.channels as f32) * self.buffer[d];
            }
        }
        self.buffer.push_front(v);
        if self.buffer.len() > sample_rate as usize {
            self.buffer.pop_back();
        }
        self.value = v;
    }
}
