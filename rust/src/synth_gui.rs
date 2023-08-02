// #![feature(new_uninit)]

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use egui::plot::{Line, Plot, PlotPoints};
use egui::Color32;
use egui_extras::*;
use std::any::Any;
use std::cell::OnceCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
//use egui::plot::{Line, Plot, PlotPoints};
use glob::*;
use ron::*;
use std::sync::{Arc, Mutex, OnceLock};

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

// mod gui_graph;

// const black_of_space: egui::Color32 = egui::Color32::from_rgb(0x00, 0x00, 0x00);
// const white_of_spaceship: egui::Color32 = egui::Color32::from_rgb(0xFF, 0xFF, 0xFF);
// const red_of_hal: egui::Color32 = egui::Color32::from_rgb(0xD9, 0x00, 0x00);
// const blue_of_earth: egui::Color32 = egui::Color32::from_rgb(0x1E, 0x90, 0xFF);
// const yellow_of_spacesuit: egui::Color32 = egui::Color32::from_rgb(0xFF, 0xD7, 0x00);
// const stargate_orange: egui::Color32 = egui::Color32::from_rgb(0xFF, 0x45, 0x00);
// const stargate_yellow: egui::Color32 = egui::Color32::from_rgb(0xFF, 0xD7, 0x00);
// const stargate_red: egui::Color32 = egui::Color32::from_rgb(0xFF, 0x00, 0x00);
fn palette() -> &'static HashMap<String, Color32> {
    static PALETTE: OnceLock<HashMap<String, Color32>> = OnceLock::new();
    PALETTE.get_or_init(|| {
        let mut m = HashMap::new();
        m.insert(String::from("Add"), Color32::from_rgb(0, 0, 0)); // Deep Black for Space
        m.insert(String::from("Bias"), Color32::from_rgb(105, 105, 105)); // Dark White for Spacesuits and Monolith
        m.insert(String::from("Scale"), Color32::from_rgb(139, 0, 0)); // Dark Red for HAL's Eye
        m.insert(String::from("SineOsc"), Color32::from_rgb(0, 0, 139)); // Dark Blue for Earth
        m.insert(String::from("SawOsc"), Color32::from_rgb(139, 69, 0)); // Dark Orange for Jupiter
        m.insert(String::from("Reverb"), Color32::from_rgb(35, 35, 35)); // Very Dark Gray for Discovery One
        m.insert(String::from("Lowpass"), Color32::from_rgb(85, 85, 85)); // Dark Silver Gray for Moon Lander
        m.insert(String::from("Sequencer"), Color32::from_rgb(64, 64, 64)); // Dark Medium Gray for Spaceships
        m.insert(String::from("type9"), Color32::from_rgb(85, 85, 85)); // Dark Light Gray for Interior Spaceship Walls
        m.insert(String::from("type10"), Color32::from_rgb(110, 110, 110)); // Dark Very Light Gray for Space Stations
        m
    })
}

#[derive(Clone)]
pub struct OutCallbacker {
    spec: AudioSpec,
    shared_graph: SharedGraph,
    // out_key: NodeKey,
    last_time: Instant,
    took: Duration,
}

// type SharedGraph = Arc<Mutex<Graph>>;
// type SharedChannels = Arc<Mutex<SlotMap<ChannelId, SharedGraph>>>;

pub fn create_shared_graph(
    audio_subsystem: &mut AudioSubsystem,
) -> (SharedGraph, AudioDevice<OutCallbacker>) {
    let shared_graph = Arc::new(Mutex::new(Graph::new()));
    // let out_key = shared_graph.lock().unwrap().add(Box::new(retn));
    let desired_spec = AudioSpecDesired {
        freq: Some(44_100),
        channels: Some(1),   // mono
        samples: Some(1000), // default sample size
    };

    let device = audio_subsystem
        .open_playback(None, &desired_spec, |spec| OutCallbacker {
            spec,
            shared_graph: shared_graph.clone(),
            // out_key,
            last_time: Instant::now(),
            took: Duration::new(0, 0),
        })
        .unwrap();

    device.resume();
    (shared_graph, device)
}

impl AudioCallback for OutCallbacker {
    type Channel = f32;

    fn callback(&mut self, sdl_out: &mut [f32]) {
        let _now = Instant::now();

        let mut graph = self.shared_graph.lock().unwrap();
        let out_key = graph.output_node.unwrap();
        for i in 0..sdl_out.len() {
            graph.step(self.spec.freq as f32);
            let output = graph.get_node(out_key).get(0);
            sdl_out[i] = 0.5 * output;
        }

        self.took = self.last_time.elapsed();
        self.last_time = Instant::now();

        if self.took.as_secs_f32() > 3.0 {
            println!("Warning very slow Graph::step()");
        }
    }
}

fn create_graph(audio_subsystem: &mut AudioSubsystem) -> (SharedGraph, AudioDevice<OutCallbacker>) {
    let (shared_graph, device) = create_shared_graph(audio_subsystem);

    (shared_graph, device)
}

pub struct GraphState {
    selected_input_port: Option<Port>,
    selected_output_port: Option<Port>,
    selected_connection: Option<Edge>,
    selected_nodes: Vec<NodeKey>,
    save_name: String,
    current_patch: Option<String>,
    last_reload_time: Option<Instant>,
    patch_files: Vec<String>,
}

struct SynthGui2 {
    shared_graph: SharedGraph,
    graph_state: GraphState,
}

impl GraphState {
    fn update_patches(&mut self) {
        self.patch_files = glob("*.patch")
            .unwrap()
            .filter_map(Result::ok)
            .filter_map(|path| {
                path.file_name()
                    .map(|name| name.to_string_lossy().to_string())
            })
            .collect();
        self.last_reload_time = Some(Instant::now());
    }
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
                save_name: "".to_string(),
                last_reload_time: None,
                patch_files: Vec::new(),
                current_patch: None,
            },
        }
    }
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
            ctx.set_pixels_per_point(2.0);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::D)) {
            if let Some(edge) = self.graph_state.selected_connection.clone() {
                graph.disconnect_edge(edge.clone());
                self.graph_state.selected_input_port = None;
                self.graph_state.selected_connection = None;
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
            let mut node_rects: HashMap<NodeKey, egui::Rect> = HashMap::new();
            let mut node_inputs_pos: HashMap<Port, egui::Pos2> = HashMap::new();
            let mut node_outputs_pos: HashMap<Port, egui::Pos2> = HashMap::new();
            // graph.sort();

            render_patch_menu(ctx, ui, &mut graph, &mut self.graph_state);

            ctx.request_repaint_after(Duration::from_millis(1000 / 60));
            ui.columns(2, |cols| {
                render_new_node_menu(&mut cols[0], &mut graph, &mut self.graph_state);
                egui::ScrollArea::both().show(&mut cols[1], |ui| {
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

fn render_patch_menu(
    ctx: &egui::Context,
    _ui: &mut egui::Ui,
    graph: &mut std::sync::MutexGuard<'_, Graph>,
    graph_state: &mut GraphState,
) {
    egui::Window::new("Patches").show(ctx, |ui| {
        {
            let _response =
                ui.add(egui::TextEdit::singleline(&mut graph_state.save_name).desired_width(100.0));
            if ui.add(egui::Button::new("save")).clicked() {
                let graph_copy = graph.copy();
                let serialized =
                    ron::ser::to_string_pretty(&graph_copy, ron::ser::PrettyConfig::default())
                        .unwrap();
                let name = format!("{}.patch", graph_state.save_name);
                let mut file = File::create(&name).unwrap();
                file.write_all(serialized.as_bytes()).unwrap();
                graph_state.current_patch = Some(name);
                graph_state.last_reload_time = None;
            }
            if let Some(last_reload_time) = graph_state.last_reload_time {
                if last_reload_time.elapsed().as_secs() > 2 {
                    graph_state.update_patches();
                }
            } else {
                graph_state.update_patches();
                graph_state.last_reload_time = Some(Instant::now());
            }
        }
        for file in graph_state.patch_files.clone() {
            if file.contains(&graph_state.save_name) || graph_state.save_name.is_empty() {
                let patch_name = file.split(".patch").nth(0).unwrap();
                let color = match &graph_state.current_patch {
                    Some(current_patch) => match &file {
                        path if path == current_patch => egui::Color32::from_rgb(0, 128, 0),
                        _ => Default::default(),
                    },
                    _ => Default::default(),
                };
                if ui.add(egui::Button::new(patch_name).fill(color)).clicked() {
                    let file_contents = std::fs::read_to_string(&file).unwrap();
                    let result_graph: Result<Graph, _> = ron::from_str(&file_contents);
                    match result_graph {
                        Ok(loaded_graph) => {
                            **graph = loaded_graph;
                            if graph.output_node.is_none() {
                                let mut out = None;
                                if let Some((out_key, _)) = graph.get_by_type_mut::<Out>() {
                                    out = Some(out_key);
                                }
                                graph.output_node = out;
                            }
                            graph.sort();
                            graph_state.current_patch = Some(file.clone());
                            graph_state.save_name = patch_name[..].to_string();
                        }
                        Err(error) => println!("{:?}", error),
                    }
                }
            }
        }
    });
}

fn draw_sequencer(
    ui: &mut egui::Ui,
    node_key: NodeKey,
    graph: &mut std::sync::MutexGuard<'_, Graph>,
    graph_state: &mut GraphState,
) {
    let mut node = graph.get_node_mut(node_key);
    let v: &mut dyn Any = node.as_any_mut();

    let sequencer: &mut Sequencer = v.downcast_mut::<Sequencer>().unwrap();
    ui.push_id(node_key, |ui| {
        let mut s = ui.style().as_ref().clone();
        s.spacing.item_spacing = egui::Vec2::ZERO;
        ui.set_style(s);
        egui_extras::TableBuilder::new(ui)
            // Allocate columns for all sequence beats and 2 extra for sequence length control
            .columns(Column::auto(), sequencer.sequence.len() + 2)
            .body(|mut body| {
                body.rows(5.0, 12, |inv_row_idx, mut row| {
                    let row_idx = 11 - inv_row_idx;
                    for (idx, note) in sequencer.sequence.iter_mut().enumerate() {
                        row.col(|ui| {
                            let (response, painter) = ui
                                .allocate_painter(egui::Vec2::new(10.0, 5.0), egui::Sense::click());
                            let stroke_color = if row_idx == note.octave as usize {
                                egui::Color32::RED
                            } else {
                                egui::Color32::DARK_GRAY
                            };
                            painter.rect_stroke(
                                response.rect,
                                0.0,
                                egui::Stroke::new(1.0, stroke_color),
                            );
                            if row_idx == note.pitch as usize {
                                let color = if note.active {
                                    egui::Color32::GREEN
                                } else {
                                    egui::Color32::GRAY.gamma_multiply(0.5)
                                };
                                painter.rect_filled(
                                    egui::Rect::from_center_size(
                                        response.rect.center(),
                                        response.rect.size() * 0.8,
                                    ),
                                    0.0,
                                    color,
                                );
                                if note.active && idx == sequencer.beat {
                                    ui.painter().add(
                                        egui::Frame::none()
                                            .shadow(egui::epaint::Shadow {
                                                extrusion: 3.0,
                                                color,
                                            })
                                            .rounding(response.rect.width() * 0.5)
                                            .paint(egui::Rect::from_center_size(
                                                response.rect.center(),
                                                response.rect.size() * 0.8,
                                            )),
                                    );
                                }
                            } else if idx == sequencer.beat {
                                painter.rect_filled(
                                    response.rect,
                                    0.0,
                                    egui::Color32::DARK_GREEN.gamma_multiply(0.3),
                                );
                            }
                            if response.clicked() {
                                if row_idx == note.pitch as usize {
                                    note.active = !note.active;
                                } else {
                                    note.active = true;
                                }
                                note.pitch = row_idx as u8;
                            }
                            if response.hovered() {
                                ui.input(|input| {
                                    if input.scroll_delta.y > 0.0 {
                                        note.octave += 1;
                                    } else if input.scroll_delta.y < 0.0 {
                                        note.octave -= 1;
                                    }
                                });
                            }

                            // let beat_button = ui.add(
                            // egui::Button::new("") //format!("{}", pitch))
                            // .small()
                            // .fill(fill_color)
                            // .stroke(stroke),
                            // );
                            // if beat_button.clicked() {
                            // *beat = !beat.clone();
                            // }
                            // if beat_button.hovered() {
                            //     ui.input(|input| {
                            //         if input.scroll_delta.y > 0.0 {
                            //             *pitch += 1;
                            //         } else if input.scroll_delta.y < 0.0 {
                            //             *pitch -= 1;
                            //         }
                            //     });
                            // }
                        });
                    }
                    // row.col(|ui| {
                    //     if ui.button("-").clicked() {
                    //         sequencer.sequence.pop();
                    //     }
                    // });
                    // row.col(|ui| {
                    //     if ui.button("+").clicked() {
                    //         sequencer.sequence.push((false, 0));
                    //     }
                    // });
                })
            });
    });
    // egui::
    // TableBuilder
    // for beat in sequencer.sequence {}
}

fn draw_out(
    ui: &mut egui::Ui,
    node_key: NodeKey,
    graph: &mut std::sync::MutexGuard<'_, Graph>,
    graph_state: &mut GraphState,
) {
    if let Some(ref buff) = graph.get_node(node_key).buff() {
        let points: PlotPoints = buff
            .iter()
            .enumerate()
            .map(|(sample, v)| [sample as f64, *v as f64])
            .collect();

        let line = Line::new(points);
        Plot::new("my_plot")
            .height(30.0)
            .width(100.0)
            .show(ui, |plot_ui| plot_ui.line(line));
    }
}

fn draw_subgraph(
    ui: &mut egui::Ui,
    node_key: NodeKey,
    graph: &mut std::sync::MutexGuard<'_, Graph>,
    graph_state: &mut GraphState,
) {
    let mut to_load_filename: Option<String> = None;
    let combo = egui::ComboBox::new(node_key, "Select patch").show_ui(ui, |ui| {
        for patch_filename in graph_state.patch_files.clone() {
            let mut v: i32 = 0;
            if ui
                .selectable_value(&mut v, 1, patch_filename.split(".patch").nth(0).unwrap())
                .clicked()
            {
                to_load_filename = Some(patch_filename);
            }
        }
    });

    if let Some(filename) = to_load_filename {
        graph.disconnect_node(node_key);
        let mut node = graph.get_node_mut(node_key);
        let v: &mut dyn Any = node.as_any_mut();

        let subgraph: &mut Subgraph = v.downcast_mut::<Subgraph>().unwrap();
        subgraph.load(filename);
    }
    // if ui.button("load").clicked() {
    // subgraph.load("bladesmall.patch".to_string());
    // }
}

fn draw_scale(
    ui: &mut egui::Ui,
    node_key: NodeKey,
    graph: &mut std::sync::MutexGuard<'_, Graph>,
    graph_state: &mut GraphState,
) {
    let mut node = graph.get_node_mut(node_key);
    let v: &mut dyn Any = node.as_any_mut();

    let scale: &mut Scale = v.downcast_mut::<Scale>().unwrap();

    // ui.horizontal(|ui| {
    //     ui.checkbox(&mut scale.fractional, "frac");
    //     if scale.fractional {
    //         // ui.set_max_size([100.0, 20.0].into());
    //         let mut nom: String = scale.nominator.to_string();
    //         let mut den: String = scale.denominator.to_string();
    //         ui.add_sized(
    //             egui::Vec2::from([30.0, 20.0]),
    //             egui::TextEdit::singleline(&mut nom),
    //         );
    //         // ui.text_edit_singleline(&mut nom);
    //         ui.label("/");
    //         ui.add_sized(
    //             egui::Vec2::from([30.0, 20.0]),
    //             egui::TextEdit::singleline(&mut den),
    //         );
    //         scale.nominator = nom.parse::<i32>().unwrap_or(1);
    //         scale.denominator = den.parse::<u32>().unwrap_or(1);
    //     }
    // });
    // // ui.text_edit_singleline(&mut nom)
    // if ui.button("load").clicked() {
    // subgraph.load("bladesmall.patch".to_string());
    // }
}

fn render_node_custom(
    ui: &mut egui::Ui,
    graph: &mut std::sync::MutexGuard<'_, Graph>,
    graph_state: &mut GraphState,
    node_key: NodeKey,
) {
    let type_name = graph.get_node(node_key).typetag_name().clone();
    match type_name {
        "Out" => {
            draw_out(ui, node_key, graph, graph_state);
        }
        "Sequencer" => {
            draw_sequencer(ui, node_key, graph, graph_state);
        }
        "Subgraph" => {
            draw_subgraph(ui, node_key, graph, graph_state);
        }
        "Scale" => {
            draw_scale(ui, node_key, graph, graph_state);
        }
        _ => {}
    }
}

static NODE_TYPES: OnceLock<Mutex<Vec<Box<dyn Node>>>> = OnceLock::new();

fn render_new_node_menu(
    ui: &mut egui::Ui,
    graph: &mut std::sync::MutexGuard<'_, Graph>,
    _graph_state: &mut GraphState,
) {
    let node_types = NODE_TYPES
        .get_or_init(|| {
            Mutex::new(vec![
                Box::new(Add::default()),
                Box::new(SineOsc::default()),
                Box::new(SawOsc::default()),
                Box::new(Scale::default()),
                Box::new(Bias::default()),
                Box::new(Reverb::default()),
                Box::new(Lowpass::default()),
                Box::new(Envelope::default()),
                Box::new(Sequencer::default()),
                Box::new(Subgraph::default()),
                Box::new(PhaseGen::default()),
            ])
        })
        .lock()
        .unwrap();

    ui.vertical(|ui| {
        for node in &*node_types {
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
    node_idx: &NodeKey,
    node_inputs_pos: &mut HashMap<Port, eframe::epaint::Pos2>,
    node_outputs_pos: &mut HashMap<Port, eframe::epaint::Pos2>,
    node_rects: &mut HashMap<NodeKey, eframe::epaint::Rect>,
) {
    // let frame = egui::Frame {
    // inner_margin: egui::Margin::same(5.0),
    // rounding: egui::Rounding::same(4.0),
    // stroke: egui::Stroke::new(1.0, egui::Color32::DARK_GRAY),
    // fill: ..Default::default(),
    // };
    let r = //frame.show(ui, |ui| {
        ui.group(|ui| {
        ui.vertical(|ui| {
            render_node_custom(ui, &mut graph, graph_state, *node_idx);
            render_core_node(
                ui,
                graph,
                node_idx,
                graph_state,
                node_inputs_pos,
                node_outputs_pos,
            );
        });
    });
    if r.response.interact(egui::Sense::click()).clicked() {
        graph_state.selected_nodes = vec![*node_idx];
    }
    node_rects.insert(*node_idx, r.response.rect);
}

fn render_core_node(
    ui: &mut egui::Ui,
    mut graph: &mut std::sync::MutexGuard<'_, Graph>,
    node_idx: &NodeKey,
    graph_state: &mut GraphState,
    node_inputs_pos: &mut HashMap<Port, eframe::epaint::Pos2>,
    node_outputs_pos: &mut HashMap<Port, eframe::epaint::Pos2>,
) {
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
            if node_outputs_pos.contains_key(&edge.from) && node_inputs_pos.contains_key(&edge.to) {
                let color = if graph_state.selected_connection == Some(edge.clone()) {
                    egui::Color32::from_rgba_premultiplied(0, 255, 0, 128)
                } else {
                    egui::Color32::from_rgba_premultiplied(255, 0, 0, 128)
                };
                draw_bezier(
                    ui,
                    node_outputs_pos[&edge.from],
                    node_inputs_pos[&edge.to],
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
}

fn render_output_port(
    ui: &mut egui::Ui,
    node_outputs_pos: &mut HashMap<Port, eframe::epaint::Pos2>,
    node_idx: &NodeKey,
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
        Knob::new(&mut v, ui.auto_id_with(node_idx))
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
    node_outputs_pos.insert(
        Port {
            node: *node_idx,
            port: output_idx,
            kind: PortKind::Output,
        },
        res.rect.center(),
    );
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
    node_idx: &NodeKey,
    input_idx: usize,
    ui: &mut egui::Ui,
    node_inputs_pos: &mut HashMap<Port, eframe::epaint::Pos2>,
) {
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
    let mut val = graph.get_node_mut(*node_idx).get_input(input_idx);
    let res = ui.add(
        // Knob::new(graph.get_node_mut(*node_idx).get_input_mut(input_idx))
        Knob::new(&mut val, ui.auto_id_with(node_idx))
            .speed(10.0 / 300.0)
            .color(egui::Color32::RED)
            .clamp_range(0.0..=2.0) // .with_id(_node_id),
            .selected(selected),
    );
    graph.get_node_mut(*node_idx).set(input_idx, val);
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
    node_inputs_pos.insert(
        Port {
            node: *node_idx,
            port: input_idx,
            kind: PortKind::Input,
        },
        res.rect.center(),
    );
    knob::draw_knob_text(
        ui,
        graph.get_node(*node_idx).inputs()[input_idx].name,
        egui::Color32::GRAY,
        6.0,
        res.rect,
    )
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

    let (shared_graph, _device) = create_graph(&mut audio_subsystem);

    // let file_contents = std::fs::read_to_string("synth3.patch").unwrap();
    // let graph: Graph = serde_json::from_str(&file_contents).unwrap();
    // *shared_graph.lock().unwrap() = graph;

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(320.0, 240.0)),
        ..Default::default()
    };

    eframe::run_native(
        "synthotron",
        options,
        Box::new(move |_cc| Box::new(SynthGui2::new(shared_graph))),
    )
    .unwrap();
}
