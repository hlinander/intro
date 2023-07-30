use crate::graph::*;
use serde::{Deserialize, Serialize};
//use std::collections::VecDeque;

#[derive(Clone, Serialize, Deserialize)]
pub struct Highpass {
    // input ports
    pub input: f32,
    pub prev: f32,
    pub prev_out: f32,

    pub cutoff: f32,

    // output ports
    pub value: f32,
    //buffer: VecDeque<f32>
}

// void lp(float* buffer, size_t size, float cutoff) {
//     // LP
//     float dt = 1.0 / PCM_SAMPLE_RATE;
//     float RC = 1.0 / (2 * 3.14159 * cutoff);
//     float alpha = dt / (dt + RC);
//     std::vector<float> internal(size);
//     internal[0] = alpha * buffer[0];
//     for (int i = 1; i < size; i += 1) {
//         internal[i] = alpha * buffer[i] + (1. - alpha) * internal[i - 1];
//     }
//     std::copy(internal.begin(), internal.end(), buffer);
// }
//
// void hp(float* buffer, size_t size, float cutoff) {
//     // HP
//     std::vector<float> internal(size);
//     float dt = 1.0 / PCM_SAMPLE_RATE;
//     float RCH = 1.0 / (2 * 3.14159 * cutoff);
//     float alpha = RCH / (dt + RCH);
//     internal[0] = buffer[0];
//     for (int i = 1; i < size; i += 1) {
//         internal[i] = alpha * internal[i - 1] + alpha * (buffer[i] - buffer[i - 1]);
//     }
//     std::copy(internal.begin(), internal.end(), buffer);
// }

impl Default for Highpass {
    fn default() -> Self {
        Self {
            input: 0.0,
            prev: 0.0,
            prev_out: 0.0,
            cutoff: 10.0,
            value: 0.0,
            //buffer: VecDeque::new()
        }
    }
}

#[typetag::serde]
impl Node for Highpass {
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
        "Highpass"
    }
    fn inputs(&self) -> Vec<Input> {
        vec![(0, "input"), (1, "cutoff")]
    }
    fn outputs(&self) -> Vec<Output> {
        vec![(0, "value")]
    }

    // Set input at index idx to value val
    fn set(&mut self, idx: usize, val: f32) {
        match idx {
            0 => self.input = val,
            1 => self.cutoff = val,
            _ => panic!("Invalid input id"),
        }
    }

    fn get_input_mut(&mut self, idx: usize) -> &mut f32 {
        match idx {
            0 => &mut self.input,
            1 => &mut self.cutoff,
            _ => panic!("Invalid input id"),
        }
    }

    // Get value of output index idx
    fn get(&self, idx: usize) -> f32 {
        valid_idx!(self.value, idx, 1)
    }

    fn step(&mut self, sample_rate: f32) {
        /*
        // void hp(float* buffer, size_t size, float cutoff) {
        //     // HP
        //     std::vector<float> internal(size);
        //     float dt = 1.0 / PCM_SAMPLE_RATE;
        //     float RCH = 1.0 / (2 * 3.14159 * cutoff);
        //     float alpha = RCH / (dt + RCH);
        //     internal[0] = buffer[0];
        //     for (int i = 1; i < size; i += 1) {
        //         internal[i] = alpha * internal[i - 1] + alpha * (buffer[i] - buffer[i - 1]);
        //     }
        //     std::copy(internal.begin(), internal.end(), buffer);
        // }
         */
        let dt = 1.0 / sample_rate;
        let rch = 1.0 / (2.0 * 3.14159 * self.cutoff * 1000.0);
        let alpha = rch / (dt + rch);
        let v = alpha * self.prev_out + alpha * (self.input - self.prev);
        //if self.buffer.len() > 1 {
        //    v = alpha * self.buffer[1] + alpha * (self.input - self._prev);
        //}
        //self.buffer.push_front(v);
        //if self.buffer.len() > sample_rate as usize {
        //    self.buffer.pop_back();
        //}
        self.value = v;
        self.prev = self.input;
        self.prev_out = v;
    }
}

// std::vector<int32_t> deltas(6);
// for (int i = 0; i < 6; ++i) {
//     deltas[i] = PCM_NUM_CHANNELS * time_to_sample(delay) / float(i + 1);
// }
// int delta = time_to_sample(delay);
// damp = damp / 6.0;
// for (int i = 0; i < size; i += PCM_NUM_CHANNELS) {
//     for (int j = 0; j < 6; ++j) {
//         int32_t delta = deltas[j];
//         buffer[i + delta] += damp * buffer[i];
//         delta -= delta * 0.05;
//         buffer[i + delta + 1] += damp * buffer[i + 1];
//     }
// }
