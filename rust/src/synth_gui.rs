// #![feature(new_uninit)]

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use std::collections::HashMap;
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
use knob::*;
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

struct SynthGui2 {
    shared_graph: SharedGraph,
}

impl SynthGui2 {
    fn new(shared_graph: SharedGraph) -> Self {
        Self { shared_graph }
    }
}

fn draw_circle(ui: &mut egui::Ui, color: egui::Color32) -> egui::Rect {
    let mut pos: egui::Pos2 = egui::Pos2::ZERO;
    // ui.allocate_ui_with_layout(
    // egui::Vec2::new(20.0, 20.0),
    // egui::Layout::top_down(egui::Align::Center),
    // |ui| {
    let (rect, response) =
        ui.allocate_exact_size(egui::Vec2::new(20.0, 20.0), egui::Sense::click_and_drag());
    ui.painter().circle(
        rect.center(),
        rect.width() / 2.0,
        color,
        egui::Stroke::from((2.0, egui::Color32::BLACK)),
    );
    rect
}

fn draw_bezier(ui: &mut egui::Ui, src_pos: egui::Pos2, dst_pos: egui::Pos2) {
    let connection_stroke = egui::Stroke {
        width: 2.0,
        color: egui::Color32::from_rgba_premultiplied(255, 0, 0, 128),
    };

    let control_scale = ((dst_pos.x - src_pos.x) / 2.0).max(30.0);
    let y_dist = 0.5 * (dst_pos.y - src_pos.y - 20.0);
    let src_control = src_pos + egui::Vec2::Y * control_scale + egui::Vec2::X * y_dist;
    let dst_control = dst_pos - egui::Vec2::Y * control_scale + egui::Vec2::X * y_dist;

    let bezier = egui::epaint::CubicBezierShape::from_points_stroke(
        [src_pos, src_control, dst_control, dst_pos],
        false,
        egui::Color32::TRANSPARENT,
        connection_stroke,
    );
    ui.painter().add(bezier);
}

impl eframe::App for SynthGui2 {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ctx.set_pixels_per_point(4.0);
        egui::gui_zoom::zoom_with_keyboard_shortcuts(ctx, _frame.info().native_pixels_per_point);
        egui::CentralPanel::default().show(ctx, |ui| {
            let mouse_pos = ctx.input(|i| i.pointer.hover_pos().unwrap_or(egui::Pos2::ZERO));
            let mut graph = self.shared_graph.lock().unwrap();
            let mut node_rects: HashMap<usize, egui::Rect> = HashMap::new();
            let mut node_inputs_pos: HashMap<(usize, usize), egui::Pos2> = HashMap::new();
            let mut node_outputs_pos: HashMap<(usize, usize), egui::Pos2> = HashMap::new();
            graph.sort();
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    for node_idx in graph.node_order() {
                        render_node(
                            ui,
                            &graph,
                            node_idx,
                            &mut node_inputs_pos,
                            &mut node_outputs_pos,
                            &mut node_rects,
                        );
                    }
                })
            });
        });
    }
}

fn render_node(
    ui: &mut egui::Ui,
    graph: &std::sync::MutexGuard<'_, Graph>,
    node_idx: &usize,
    node_inputs_pos: &mut HashMap<(usize, usize), eframe::epaint::Pos2>,
    node_outputs_pos: &mut HashMap<(usize, usize), eframe::epaint::Pos2>,
    node_rects: &mut HashMap<usize, eframe::epaint::Rect>,
) {
    let r = ui.group(|ui| {
        let layout =
            egui::Layout::left_to_right(egui::Align::Center).with_cross_align(egui::Align::Center);
        ui.allocate_ui_with_layout(egui::Vec2::ZERO, layout, |ui| {
            ui.label(graph.get_node(*node_idx).typetag_name());

            // Render node input ports
            let node_inputs = graph.get_node(*node_idx).inputs().clone();
            for (input_idx, _input) in node_inputs.iter().enumerate() {
                render_input_port(graph, node_idx, input_idx, ui, node_inputs_pos);
            }

            // Render cables to the input ports of this node
            for ((_edge_out_idx, port_out_idx), (edge_in_idx, port_in_idx)) in
                &graph.node_inputs()[node_idx]
            {
                draw_bezier(
                    ui,
                    node_outputs_pos[&(*_edge_out_idx, *port_out_idx)],
                    node_inputs_pos[&(*edge_in_idx, *port_in_idx)],
                );
            }

            // Render node output ports
            let node_outputs = graph.get_node(*node_idx).outputs().clone();
            for (output_idx, _output) in node_outputs.iter().enumerate() {
                render_output_port(ui, node_outputs_pos, node_idx, output_idx, graph);
            }
        });
    });
    node_rects.insert(*node_idx, r.response.rect);
}

fn render_output_port(
    ui: &mut egui::Ui,
    node_outputs_pos: &mut HashMap<(usize, usize), eframe::epaint::Pos2>,
    node_idx: &usize,
    output_idx: usize,
    graph: &std::sync::MutexGuard<'_, Graph>,
) {
    let rect = draw_circle(ui, egui::Color32::GOLD);
    node_outputs_pos.insert((*node_idx, output_idx), rect.center());
    knob::draw_knob_text(
        ui,
        graph.get_node(*node_idx).outputs()[output_idx].1,
        egui::Color32::GRAY,
        rect,
    );
}

fn render_input_port(
    graph: &std::sync::MutexGuard<'_, Graph>,
    node_idx: &usize,
    input_idx: usize,
    ui: &mut egui::Ui,
    node_inputs_pos: &mut HashMap<(usize, usize), eframe::epaint::Pos2>,
) {
    if graph.node_inputs()[node_idx]
        .iter()
        .filter(|(input, out)| out.1 == input_idx)
        .count()
        == 0
    {
        let res = ui.add(
            Knob::new(graph.get_node_mut(*node_idx).get_input_mut(input_idx))
                .speed(10.0 / 300.0)
                .clamp_range(0.0..=10.0), // .with_id(_node_id),
        );
        node_inputs_pos.insert((*node_idx, input_idx), res.rect.center());
        knob::draw_knob_text(
            ui,
            graph.get_node(*node_idx).inputs()[input_idx].1,
            egui::Color32::GRAY,
            res.rect,
        );
    } else {
        let rect = draw_circle(ui, egui::Color32::DARK_GREEN);
        node_inputs_pos.insert((*node_idx, input_idx), rect.center());
        knob::draw_knob_text(
            ui,
            graph.get_node(*node_idx).inputs()[input_idx].1,
            egui::Color32::GRAY,
            rect,
        );
    }
}

fn main() {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    // tracing_subscriber::fmt::init();

    let sdl_context = sdl2::init().unwrap();
    let mut audio_subsystem = sdl_context.audio().unwrap();

    let (shared_graph, _device, out_idx) = create_graph(&mut audio_subsystem);

    let file_contents = std::fs::read_to_string("synth2.patch").unwrap();
    let graph: Graph = serde_json::from_str(&file_contents).unwrap();
    *shared_graph.lock().unwrap() = graph;

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(320.0, 240.0)),
        ..Default::default()
    };

    eframe::run_native(
        "My egui App",
        options,
        // Box::new(move |cc| Box::new(gui_graph::NodeGraphExample::new(cc, shared_graph, out_idx))),
        Box::new(move |cc| Box::new(SynthGui2::new(shared_graph))),
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
                let mut sine_osc = x.get_by_type_mut::<SawOsc>().unwrap();
                sine_osc.freq = self.freq as f32;
            }
        });
    }
}
