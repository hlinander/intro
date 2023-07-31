// #![feature(new_uninit)]

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use std::collections::{HashMap, HashSet};
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

const black_of_space: egui::Color32 = egui::Color32::from_rgb(0x00, 0x00, 0x00);
const white_of_spaceship: egui::Color32 = egui::Color32::from_rgb(0xFF, 0xFF, 0xFF);
const red_of_hal: egui::Color32 = egui::Color32::from_rgb(0xD9, 0x00, 0x00);
const blue_of_earth: egui::Color32 = egui::Color32::from_rgb(0x1E, 0x90, 0xFF);
const yellow_of_spacesuit: egui::Color32 = egui::Color32::from_rgb(0xFF, 0xD7, 0x00);
const stargate_orange: egui::Color32 = egui::Color32::from_rgb(0xFF, 0x45, 0x00);
const stargate_yellow: egui::Color32 = egui::Color32::from_rgb(0xFF, 0xD7, 0x00);
const stargate_red: egui::Color32 = egui::Color32::from_rgb(0xFF, 0x00, 0x00);

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

struct GraphState {
    selected_input_port: Option<Port>,
    selected_output_port: Option<Port>,
    selected_connection: Option<Edge>,
    selected_nodes: Vec<usize>,
}

struct SynthGui2 {
    shared_graph: SharedGraph,
    graph_state: GraphState,
}

impl SynthGui2 {
    fn new(shared_graph: SharedGraph) -> Self {
        Self {
            shared_graph,
            graph_state: GraphState {
                selected_input_port: None,
                selected_output_port: None,
                selected_connection: None,
                selected_nodes: Vec::new(),
            },
        }
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

fn draw_bezier(ui: &mut egui::Ui, src_pos: egui::Pos2, dst_pos: egui::Pos2, color: egui::Color32) {
    let connection_stroke = egui::Stroke {
        width: 2.0,
        color, //egui::Color32::from_rgba_premultiplied(255, 0, 0, 128),
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
        let mut graph = self.shared_graph.lock().unwrap();
        if ctx.input(|i| i.key_pressed(egui::Key::R)) {
            ctx.set_pixels_per_point(3.0);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::D)) {
            if let Some(edge) = self.graph_state.selected_connection.clone() {
                graph.disconnect_edge(edge.clone());
                self.graph_state.selected_nodes = vec![edge.from.node, edge.to.node];
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F)) {
            self.graph_state.selected_nodes.iter().for_each(|node_idx| {
                graph.remove(*node_idx);
            })
        }
        egui::gui_zoom::zoom_with_keyboard_shortcuts(ctx, _frame.info().native_pixels_per_point);
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut node_rects: HashMap<usize, egui::Rect> = HashMap::new();
            let mut node_inputs_pos: HashMap<(usize, usize), egui::Pos2> = HashMap::new();
            let mut node_outputs_pos: HashMap<(usize, usize), egui::Pos2> = HashMap::new();
            // graph.sort();
            ctx.request_repaint_after(Duration::from_millis(1000 / 60));
            ui.columns(2, |cols| {
                render_new_node_menu(&mut cols[0], &mut graph, &mut self.graph_state);
                egui::ScrollArea::vertical().show(&mut cols[1], |ui| {
                    ui.vertical(|ui| {
                        for node_idx in graph.node_order().clone() {
                            render_node(
                                ui,
                                &mut graph,
                                &mut self.graph_state,
                                &node_idx,
                                &mut node_inputs_pos,
                                &mut node_outputs_pos,
                                &mut node_rects,
                            );
                        }
                    })
                });
            })
        });
    }
}

fn render_new_node_menu(
    ui: &mut egui::Ui,
    mut graph: &mut std::sync::MutexGuard<'_, Graph>,
    graph_state: &mut GraphState,
) {
    let node_types: Vec<Box<dyn Node>> = vec![Box::new(Add::default()), Box::new(Bias::default())];

    ui.vertical(|ui| {
        for node in node_types {
            let res = ui.button(node.typetag_name());
            if res.clicked() {
                graph.add(node.copy());
            }
        }
    });
}

fn render_node(
    ui: &mut egui::Ui,
    mut graph: &mut std::sync::MutexGuard<'_, Graph>,
    graph_state: &mut GraphState,
    node_idx: &usize,
    node_inputs_pos: &mut HashMap<(usize, usize), eframe::epaint::Pos2>,
    node_outputs_pos: &mut HashMap<(usize, usize), eframe::epaint::Pos2>,
    node_rects: &mut HashMap<usize, eframe::epaint::Rect>,
) {
    let r = ui.group(|ui| {
        let layout =
            egui::Layout::left_to_right(egui::Align::Center).with_cross_align(egui::Align::Center);
        ui.allocate_ui_with_layout(egui::Vec2::ZERO, layout, |ui| {
            let name = graph.get_node(*node_idx).typetag_name();
            let text = if graph_state.selected_nodes.contains(node_idx) {
                egui::RichText::new(name).underline()
            } else {
                egui::RichText::new(name)
            };
            ui.label(text);

            // Render node input ports
            let node_inputs = graph.get_node(*node_idx).inputs().clone();
            for (input_idx, _input) in node_inputs.iter().enumerate() {
                render_input_port(
                    &mut graph,
                    graph_state,
                    node_idx,
                    input_idx,
                    ui,
                    node_inputs_pos,
                );
            }

            // Render cables to the input ports of this node
            // for ((_edge_out_idx, port_out_idx), (edge_in_idx, port_in_idx)) in
            for edge in &graph.node_inputs()[node_idx] {
                let from_spec = (edge.from.node, edge.from.port);
                let to_spec = (edge.to.node, edge.to.port);
                if node_outputs_pos.contains_key(&from_spec)
                    && node_inputs_pos.contains_key(&to_spec)
                {
                    let color = if graph_state.selected_connection == Some(edge.clone()) {
                        egui::Color32::from_rgba_premultiplied(0, 255, 0, 128)
                    } else {
                        egui::Color32::from_rgba_premultiplied(255, 0, 0, 128)
                    };
                    draw_bezier(
                        ui,
                        node_outputs_pos[&from_spec],
                        node_inputs_pos[&to_spec],
                        color,
                    );
                }
            }

            // Render node output ports
            let node_outputs = graph.get_node(*node_idx).outputs().clone();
            for (output_idx, _output) in node_outputs.iter().enumerate() {
                render_output_port(
                    ui,
                    node_outputs_pos,
                    node_idx,
                    output_idx,
                    graph,
                    graph_state,
                );
            }
        });
    });
    if r.response.interact(egui::Sense::click()).clicked() {
        graph_state.selected_nodes = vec![*node_idx];
    }
    node_rects.insert(*node_idx, r.response.rect);
}

fn render_output_port(
    ui: &mut egui::Ui,
    node_outputs_pos: &mut HashMap<(usize, usize), eframe::epaint::Pos2>,
    node_idx: &usize,
    output_idx: usize,
    graph: &mut std::sync::MutexGuard<'_, Graph>,
    graph_state: &mut GraphState,
) {
    // let rect = draw_circle(ui, egui::Color32::GOLD);
    let mut v = graph.get_node_mut(*node_idx).get(output_idx);

    let port = Port {
        node: *node_idx,
        port: output_idx,
        kind: PortKind::Output,
    };

    let selected = if let Some(sel_port) = graph_state.selected_output_port.clone() {
        sel_port == port
    } else {
        false
    };

    let res = ui.add(
        Knob::new(&mut v)
            .color(egui::Color32::GREEN)
            .speed(10.0 / 300.0)
            .clamp_range(0.0..=2.0)
            .selected(selected), // .with_id(_node_id),
    );
    if res.clicked() {
        if let Some(sel_port) = graph_state.selected_output_port.clone() {
            if sel_port != port {
                graph_state.selected_output_port = Some(port);
            } else {
                graph_state.selected_output_port = None;
            }
        } else {
            graph_state.selected_output_port = Some(port);
        }
        maybe_create_connection(graph_state, graph);
        println!("{:?}", graph_state.selected_output_port);
    }
    node_outputs_pos.insert((*node_idx, output_idx), res.rect.center());
    knob::draw_knob_text(
        ui,
        graph.get_node(*node_idx).outputs()[output_idx].name,
        egui::Color32::GRAY,
        6.0,
        res.rect,
    );
}

fn render_input_port(
    graph: &mut std::sync::MutexGuard<'_, Graph>,
    graph_state: &mut GraphState,
    node_idx: &usize,
    input_idx: usize,
    ui: &mut egui::Ui,
    node_inputs_pos: &mut HashMap<(usize, usize), eframe::epaint::Pos2>,
) {
    // if graph.node_inputs()[node_idx]
    //     .iter()
    //     .filter(|edge| edge.from.port == input_idx)
    //     .count()
    //     == 0
    // {
    let port = Port {
        node: *node_idx,
        port: input_idx,
        kind: PortKind::Input,
    };

    let selected = if let Some(sel_port) = graph_state.selected_input_port.clone() {
        sel_port == port
    } else {
        false
    };

    let res = ui.add(
        Knob::new(graph.get_node_mut(*node_idx).get_input_mut(input_idx))
            .speed(10.0 / 300.0)
            .color(egui::Color32::RED)
            .clamp_range(0.0..=2.0) // .with_id(_node_id),
            .selected(selected),
    );
    if res.clicked() {
        if let Some(sel_port) = graph_state.selected_input_port.clone() {
            if sel_port != port {
                graph_state.selected_input_port = Some(port);
            } else {
                graph_state.selected_input_port = None;
                graph_state.selected_connection = None;
            }
        } else {
            graph_state.selected_input_port = Some(port);
        }

        if let Some(input_port) = graph_state.selected_input_port.clone() {
            graph_state.selected_connection = graph.get_edge(input_port);
        }

        maybe_create_connection(graph_state, graph);
        println!("{:?}", graph_state.selected_input_port);
    }
    node_inputs_pos.insert((*node_idx, input_idx), res.rect.center());
    knob::draw_knob_text(
        ui,
        graph.get_node(*node_idx).inputs()[input_idx].name,
        egui::Color32::GRAY,
        6.0,
        res.rect,
    )
    // } else {
    //     let rect = draw_circle(ui, egui::Color32::DARK_GREEN);
    //     node_inputs_pos.insert((*node_idx, input_idx), rect.center());
    //     knob::draw_knob_text(
    //         ui,
    //         graph.get_node(*node_idx).inputs()[input_idx].1,
    //         egui::Color32::GRAY,
    //         rect,
    //     );
    // }
}

fn maybe_create_connection(
    graph_state: &mut GraphState,
    graph: &mut std::sync::MutexGuard<'_, Graph>,
) {
    if let Some(input_port) = graph_state.selected_input_port.clone() {
        if let Some(output_port) = graph_state.selected_output_port.clone() {
            graph.connect(output_port.clone(), input_port.clone());
            graph_state.selected_input_port = None;
            graph_state.selected_output_port = None;
            graph_state.selected_connection = Some(Edge {
                from: output_port.clone(),
                to: input_port.clone(),
            });
            graph_state.selected_nodes = vec![output_port.node, input_port.node];
        }
    }
}

fn main() {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    // tracing_subscriber::fmt::init();

    let sdl_context = sdl2::init().unwrap();
    let mut audio_subsystem = sdl_context.audio().unwrap();

    let (shared_graph, _device, out_idx) = create_graph(&mut audio_subsystem);

    let file_contents = std::fs::read_to_string("synth3.patch").unwrap();
    let graph: Graph = serde_json::from_str(&file_contents).unwrap();
    *shared_graph.lock().unwrap() = graph;

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(320.0, 240.0)),
        ..Default::default()
    };

    eframe::run_native(
        "synthotron",
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
