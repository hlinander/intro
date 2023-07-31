// #![no_std]
// use core::panic::PanicInfo;
use std::sync::mpsc;
use std::thread;
// use std::time::Duration;
mod audio;
mod gl;
mod synth;
use crate::synth::*;

pub fn main() {
    let (audio_control, audio_control_recv) = mpsc::channel();
    let (audio_status_send, audio_status) = mpsc::channel();

    let (video_control, video_control_recv) = mpsc::channel();

    let _audio_thread_handle = thread::spawn(move || {
        audio::audio_system(audio_control_recv, audio_status_send).unwrap();
    });

    let _video_thread_handle = thread::spawn(move || {
        gl::gl_system(video_control_recv);
    });

    audio_control.send(audio::AudioControl::Start).unwrap();
    let mut video_started = false;

    for status in audio_status {
        // println!("{}", status.ticks);
        if status.ticks > 0.0 && !video_started {
            video_control.send(gl::VideoControl::Start).unwrap();
            video_started = true;
        }
    }
    println!("exiting");
}
