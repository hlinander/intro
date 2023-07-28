// Copyright The pipewire-rs Contributors.
// SPDX-License-Identifier: MIT

//! This file is a rustic interpretation of the the [PipeWire Tutorial 4][tut]
//!
//! tut: https://docs.pipewire.org/page_tutorial4.html

use crate::synth::*;
// use std::sync::mpsc;
// use std::time::Duration;

use alloc::vec::Vec;
use pipewire as pw;
use pw::prelude::*;
use pw::{properties, spa};
use spa::param::ParamType;
use spa::pod::builder::{SpaPodBuild, SpaPodBuilder};

pub const DEFAULT_RATE: u32 = 44100;
pub const DEFAULT_CHANNELS: u32 = 2;
pub const CHAN_SIZE: usize = core::mem::size_of::<i16>();

pub enum AudioControl {
    Start,
    Stop,
}

pub struct AudioStatus {
    pub ticks: f64,
}

pub fn audio_system(// control: mpsc::Receiver<AudioControl>,
    // status: mpsc::Sender<AudioStatus>,
) -> Result<(), pw::Error> {
    pw::init();
    let mainloop = pw::MainLoop::new()?;

    // FIXME: fs
    // let file_contents = std::fs::read_to_string("synth.patch").unwrap();
    let graph: Graph = Graph::new(); //serde_json::from_str(&file_contents).unwrap();

    let stream = pw::stream::Stream::<Graph>::with_user_data(
        &mainloop,
        "audio-src",
        properties! {
            *pw::keys::MEDIA_TYPE => "Audio",
            *pw::keys::MEDIA_ROLE => "Music",
            *pw::keys::MEDIA_CATEGORY => "Playback",
        },
        graph,
    )
    .process(|stream, graph| match stream.dequeue_buffer() {
        None => {} //println!("No buffer received"),
        Some(mut buffer) => {
            let datas = buffer.datas_mut();
            let stride = CHAN_SIZE * DEFAULT_CHANNELS as usize;
            let data = &mut datas[0];
            let n_frames = if let Some(slice) = data.data() {
                let n_frames = slice.len() / stride;
                // println!("writing {}", n_frames);
                for i in 0..n_frames {
                    let val = (graph.step(DEFAULT_RATE as f32) * 16767.0) as i16;
                    // let val = (f64::sin(*acc) * DEFAULT_VOLUME * 16767.0) as i16;
                    for c in 0..DEFAULT_CHANNELS {
                        let start = i * stride + (c as usize * CHAN_SIZE);
                        let end = start + CHAN_SIZE;
                        let chan = &mut slice[start..end];
                        chan.copy_from_slice(&i16::to_le_bytes(val));
                    }
                }
                n_frames
            } else {
                0
            };
            let chunk = data.chunk_mut();
            *chunk.offset_mut() = 0;
            *chunk.stride_mut() = stride as _;
            *chunk.size_mut() = (stride * n_frames) as _;
        }
    })
    .create()?;

    let mut buffer_vec = Vec::<u8>::with_capacity(1024);
    let mut builder = SpaPodBuilder::with_buffer(&mut buffer_vec);

    let mut audio_info = spa::param::audio::AudioInfoRaw::new();
    audio_info.set_format(spa::param::audio::AudioFormat::S16_LE);
    audio_info.set_rate(DEFAULT_RATE);
    audio_info.set_channels(DEFAULT_CHANNELS);
    let obj = audio_info.build_pod(&mut builder, ParamType::EnumFormat);
    let mut params = [obj];

    stream.connect(
        spa::Direction::Output,
        None,
        pw::stream::StreamFlags::AUTOCONNECT
            | pw::stream::StreamFlags::MAP_BUFFERS
            | pw::stream::StreamFlags::RT_PROCESS,
        &mut params,
    )?;

    // let status_timer = mainloop.add_timer(move |_| {
    // let ticks = stream.get_time();
    // status.send(AudioStatus { ticks });
    // });
    // status_timer.update_timer(
    // Some(Duration::from_millis(30)),
    // Some(Duration::from_millis(30)),
    // );

    // for audio_control in control {
    // match audio_control {
    // AudioControl::Start => {
    mainloop.run();
    // }
    // _ => {}
    // }
    // }

    unsafe { pw::deinit() };

    Ok(())
}
