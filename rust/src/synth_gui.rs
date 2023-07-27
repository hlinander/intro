// #![feature(new_uninit)]

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
//use egui::plot::{Line, Plot, PlotPoints};
use std::sync::{Arc, Mutex};

// use slotmap::SlotMap;

use std::time::{Duration, Instant};

use sdl2::{
    audio::AudioCallback, audio::AudioDevice, audio::AudioSpec, audio::AudioSpecDesired,
    AudioSubsystem,
};

// mod graph;
// use graph::*;
mod knob;
mod synth;
use synth::*;

// mod knob;
// mod signal;

mod gui_graph;

#[derive(Clone)]
pub struct OutCallbacker {
    spec: AudioSpec,
    shared_graph: SharedGraph,
    out_idx: usize,
    last_time: Instant,
    took: Duration,
}

// type SharedGraph = Arc<Mutex<Graph>>;
// type SharedChannels = Arc<Mutex<SlotMap<ChannelId, SharedGraph>>>;

pub fn create_shared_graph(
    audio_subsystem: &mut AudioSubsystem,
) -> (SharedGraph, AudioDevice<OutCallbacker>, usize) {
    let retn: Out = Default::default();
    let shared_graph = Arc::new(Mutex::new(Graph::new()));
    let out_idx = shared_graph.lock().unwrap().add(Box::new(retn));
    let desired_spec = AudioSpecDesired {
        freq: Some(44_100),
        channels: Some(1),   // mono
        samples: Some(1000), // default sample size
    };

    let device = audio_subsystem
        .open_playback(None, &desired_spec, |spec| OutCallbacker {
            spec,
            shared_graph: shared_graph.clone(),
            out_idx,
            last_time: Instant::now(),
            took: Duration::new(0, 0),
        })
        .unwrap();

    device.resume();
    (shared_graph, device, out_idx)
}

impl AudioCallback for OutCallbacker {
    type Channel = f32;

    fn callback(&mut self, sdl_out: &mut [f32]) {
        let _now = Instant::now();

        for i in 0..sdl_out.len() {
            let mut graph = self.shared_graph.lock().unwrap();
            let output = graph.step(self.spec.freq as f32);

            sdl_out[i] = 0.5 * output;
        }

        self.took = self.last_time.elapsed();
        self.last_time = Instant::now();

        if self.took.as_secs_f32() > 3.0 {
            println!("Warning very slow Graph::step()");
        }
    }
}

fn create_graph(
    audio_subsystem: &mut AudioSubsystem,
) -> (SharedGraph, AudioDevice<OutCallbacker>, usize) {
    let (shared_graph, device, out_node_idx) = create_shared_graph(audio_subsystem);

    (shared_graph, device, out_node_idx)
}

fn main() {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    // tracing_subscriber::fmt::init();

    let sdl_context = sdl2::init().unwrap();
    let mut audio_subsystem = sdl_context.audio().unwrap();

    let (shared_graph, _device, out_idx) = create_graph(&mut audio_subsystem);

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(320.0, 240.0)),
        ..Default::default()
    };

    eframe::run_native(
        "My egui App",
        options,
        Box::new(move |cc| Box::new(gui_graph::NodeGraphExample::new(cc, shared_graph, out_idx))),
    );
}

struct Synth {
    shared_graph: Arc<Mutex<Graph>>,
    // age: i32,
    freq: i32,
}

impl eframe::App for Synth {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_pixels_per_point(2.0);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("My egui Application");
            ui.add(egui::Slider::new(&mut self.freq, 0..=10000).text("age"));
            if ui.button("Click each year").clicked() {
                self.freq += 1;
            }

            {
                let mut x = self.shared_graph.lock().unwrap();
                let sine_osc: &mut SawOsc = x.get_by_type_mut::<SawOsc>().unwrap();
                sine_osc.freq = self.freq as f32;
            }
        });
    }
}
