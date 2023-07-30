use crate::graph::*;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Debug)]
#[repr(u8)]
#[allow(dead_code, non_camel_case_types)]
enum VoiceKeyIO {
    cv0 = 0,
    cv1,
    cv2,
    cv3,
    // cv4,
    // cv5,
    // cv6,
    // cv7,
    gate0,
    gate1,
    gate2,
    gate3,
    // gate4,
    // gate5,
    // gate6,
    // gate7,
    MAX,
}

pub const N_VOICES: usize = 4;

static VOICE_KEY_NAMES: OnceLock<[&'static str; VoiceKeyIO::MAX as usize]> = OnceLock::new();
fn voice_key_name(key: VoiceKeyIO) -> &'static str {
    VOICE_KEY_NAMES.get_or_init(|| {
        let mut outputs = Vec::new();
        for i in 0..VoiceKeyIO::MAX as u8 {
            let io_num = unsafe { core::mem::transmute::<u8, VoiceKeyIO>(i) };
            let str = format!("{:?}", io_num);
            let leaked_str: &'static str = Box::leak(str.into_boxed_str());
            outputs.push(leaked_str);
        }
        outputs.try_into().unwrap()
    })[key as usize]
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct VoiceKey {
    // output ports
    pub pitch: [f32; N_VOICES],
    pub trigger: [f32; N_VOICES],
    pub count: usize,
}

#[typetag::serde]
impl Node for VoiceKey {
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
        "VoiceKey"
    }
    fn inputs(&self) -> Vec<Input> {
        //out_names
        vec![]
    }
    fn outputs(&self) -> Vec<Output> {
        // output_names!("test", 0, 5)
        //VOICE_KEY_OUTPUTS.to_vec()
        let mut outputs = Vec::new();
        for i in 0..VoiceKeyIO::MAX as u8 {
            let io_num = unsafe { core::mem::transmute::<u8, VoiceKeyIO>(i) };
            outputs.push((i as usize, voice_key_name(io_num)));
        }
        outputs
    }

    // Set input at index idx to value val
    fn set(&mut self, _idx: usize, _val: f32) {}

    fn get_input_mut(&mut self, idx: usize) -> &mut f32 {
        panic!()
    }

    // Get value of output index idx
    fn get(&self, idx: usize) -> f32 {
        if idx < N_VOICES {
            self.pitch[idx]
        } else if idx < 2 * N_VOICES {
            self.trigger[idx - N_VOICES]
        } else {
            panic!("VoiceKey::get Index out of range");
        }
    }

    fn step(&mut self, _sample_rate: f32) {}

    fn buff(&self) -> Option<&VecDeque<f32>> {
        None
        //Some(&self.buff)
    }
}
