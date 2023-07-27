use std::sync::mpsc;
use std::thread;
use std::time::Duration;
mod audio;
mod synth;
use crate::synth::*;

pub fn main() {
    let (audio_control, audio_control_recv) = mpsc::channel();
    let (audio_status_send, _audio_status) = mpsc::channel();

    let _audio_thread_handle = thread::spawn(move || {
        audio::audio_system(audio_control_recv, audio_status_send);
    });

    thread::sleep(Duration::from_secs_f64(3.0));

    audio_control.send(audio::AudioControl::Start);

    thread::sleep(Duration::from_secs_f64(5.0));
    println!("exiting");
}
