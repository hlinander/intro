use std::sync::OnceLock;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet, VecDeque},
};

use crate::graph::{self as lolgraph, SharedGraph, UnconnectedInput, UnconnectedOutput};
use crate::knob::Knob;
use eframe::egui::{self};
use eframe::epaint::{Color32, Pos2, Vec2};
use egui::epaint;
use egui::plot::{Line, Plot, PlotPoints};
use egui::Key;
use egui_node_graph::*;
use itertools::Itertools;
use slotmap::SlotMap;
use std::time::Duration;
use std::time::Instant;
// use tracing::info;

use std::fs::File;
use std::io::Write;

use glob::glob;

use serde_with::serde_as;

/// The NodeData holds a custom data struct inside each node. It's useful to
/// store additional information that doesn't live in parameters. For this
/// example, the node data stores the template (i.e. the "type") of the node.
//#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
#[serde_as]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct GuiNodeData {
    template: GuiNodeTemplate,
    #[serde(default)]
    show_details: bool,
    lol_node_idx: Option<usize>,
    buff: Option<VecDeque<f32>>,
    graph_id: Option<GraphId>,
    sub_graph_id: Option<GraphId>,
    #[serde_as(as = "Option<Vec<(_, _)>>")]
    sub_graph_inputs: Option<HashMap<InputId, lolgraph::UnconnectedInput>>,
    #[serde_as(as = "Option<Vec<(_, _)>>")]
    sub_graph_outputs: Option<HashMap<OutputId, lolgraph::UnconnectedOutput>>,
}

pub struct NodeTypeInfo {
    type_id: core::any::TypeId,
    type_name: &'static str,
    node_name: &'static str,
    id: GuiNodeTemplate,
    default_fn: fn() -> Box<dyn lolgraph::Node>,
    can_create_in_ui: bool,
    node_drawer: Option<fn(&mut dyn lolgraph::Node) -> &mut dyn NodeDrawer>,
}

type GuiGraph = Graph<GuiNodeData, GuiConnectionDataType, GuiValueType>;
type GuiEditorState = GraphEditorState<
    GuiNodeData,
    GuiConnectionDataType,
    GuiValueType,
    GuiNodeTemplate,
    GuiGraphState,
>;

pub struct GuiGraphState {
    lol_graph: lolgraph::SharedGraph,
    keys_to_voice: HashMap<Key, usize>,
    out_node_idx: usize,
    save_name: String,
    last_reload_time: Option<Instant>,
    patch_files: Vec<String>,
    current_patch: Option<String>,
    voice_allocator: VoiceAllocator,
    next_graph_id: usize,
    detailed_nodes: HashSet<(GraphId, NodeId)>,
}

impl NodeTypeInfo {
    fn new<T: lolgraph::Node + Default>(id: GuiNodeTemplate) -> Self {
        Self {
            type_id: core::any::TypeId::of::<T>(),
            type_name: core::any::type_name::<T>(),
            node_name: T::name(),
            id,
            default_fn: || Box::new(<T as Default>::default()),
            can_create_in_ui: true,
            node_drawer: None,
        }
    }

    fn new_uncreatable<T: lolgraph::Node + Default>(id: GuiNodeTemplate) -> Self {
        let mut info = Self::new::<T>(id);
        info.can_create_in_ui = false;
        info
    }

    fn with_node_drawer<T: lolgraph::Node + NodeDrawer>(mut self) -> Self {
        self.node_drawer = Some(|node: &mut dyn lolgraph::Node| {
            let node: &mut T = node.as_any_mut().downcast_mut::<T>().unwrap();
            let node: &mut dyn NodeDrawer = node; // get the NodeDrawer vtable
            node
        });
        self
    }
}

pub trait NodeDrawer {
    fn draw(
        &mut self,
        ui: &mut egui::Ui,
        node_id: NodeId,
        graph: &GuiGraph,
        app_state: &GuiGraphState,
    ) -> Vec<NodeResponse<GuiResponse, GuiNodeData>>;
}

impl NodeDrawer for lolgraph::Out {
    fn draw(
        &mut self,
        ui: &mut egui::Ui,
        node_id: NodeId,
        graph: &GuiGraph,
        _app_state: &GuiGraphState,
    ) -> Vec<NodeResponse<GuiResponse, GuiNodeData>> {
        let responses = vec![];
        if let Some(ref buff) = graph.nodes[node_id].user_data.buff {
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
        responses
    }
}

impl NodeDrawer for lolgraph::Group {
    fn draw(
        &mut self,
        _ui: &mut egui::Ui,
        _node_id: NodeId,
        _graph: &GuiGraph,
        _app_state: &GuiGraphState,
    ) -> Vec<NodeResponse<GuiResponse, GuiNodeData>> {
        vec![]
    }
}

#[derive(PartialEq, Eq)]
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub enum GuiConnectionDataType {
    Scalar,
}

#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub enum GuiValueType {
    Scalar { value: f32 },
}

impl Default for GuiValueType {
    fn default() -> Self {
        Self::Scalar { value: 0.0 }
    }
}

impl GuiValueType {
    fn value(self) -> f32 {
        let GuiValueType::Scalar { value, .. } = self;
        value
    }
}

/// NodeTemplate is a mechanism to define node templates. It's what the graph
/// will display in the "new node" popup. The user code needs to tell the
/// library how to convert a NodeTemplate into a Node.
#[derive(Clone, Copy, PartialEq, Debug)]
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub enum GuiNodeTemplate {
    GuiSineOsc,
    GuiSawOsc,
    GuiAudioOut,
    GuiReverb,
    GuiAdd,
    GuiScale,
    GuiBias,
    GuiEnvelope,
    GuiLowpass,
    GuiHighpass,
    GuiKey,
    GuiVoiceKey,
    GuiGroup,
}

/// The response type is used to encode side-effects produced when drawing a
/// node in the graph. Most side-effects (creating new nodes, deleting existing
/// nodes, handling connections...) are already handled by the library, but this
/// mechanism allows creating additional side effects from user code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuiResponse {
    EnableDetails(NodeId),
    DisableDetails(NodeId), //SetActiveNode(NodeId),
                            //ConnectEventEnded
                            //ClearActiveNode,
}

// =========== Then, you need to implement some traits ============

// A trait for the data types, to tell the library how to display them
impl DataTypeTrait<GuiGraphState> for GuiConnectionDataType {
    fn data_type_color(&self, _user_state: &mut GuiGraphState) -> egui::Color32 {
        match self {
            GuiConnectionDataType::Scalar => egui::Color32::from_rgb(38, 109, 211),
        }
    }

    fn name(&self) -> Cow<'_, str> {
        match self {
            GuiConnectionDataType::Scalar => Cow::Borrowed("scalar"),
        }
    }
}

// fn add_inputs_and_outputs<T: Default + Node>(graph: MyGraph, name: &str,) {
//     T::default().outputs().iter().for_each(|&(_, name)| output_scalar(graph, name));
//     T::default().inputs().iter().for_each(|&(_, name)| input_scalar(graph, name));
// }
// A trait for the node kinds, which tells the library how to build new nodes
// from the templates in the node finder
impl NodeTemplateTrait for GuiNodeTemplate {
    type NodeData = GuiNodeData;
    type DataType = GuiConnectionDataType;
    type ValueType = GuiValueType;
    type UserState = GuiGraphState;
    type CategoryType = &'static str;

    fn node_finder_label(&self, _user_state: &mut Self::UserState) -> Cow<'_, str> {
        let registry = registry();
        let type_info = registry.expect_get(*self);
        Cow::Borrowed(type_info.node_name)
    }

    fn node_graph_label(&self, user_state: &mut Self::UserState) -> String {
        // It's okay to delegate this to node_finder_label if you don't want to
        // show different names in the node finder and the node itself.
        self.node_finder_label(user_state).into()
    }

    fn user_data(&self, _user_state: &mut Self::UserState) -> Self::NodeData {
        GuiNodeData {
            template: *self,
            show_details: true,
            lol_node_idx: None,
            buff: None,
            graph_id: None,
            sub_graph_id: None,
            sub_graph_inputs: None,
            sub_graph_outputs: None,
        }
    }

    fn build_node(
        &self,
        graph: &mut Graph<Self::NodeData, Self::DataType, Self::ValueType>,
        _user_state: &mut Self::UserState,
        node_id: NodeId,
    ) {
        // The nodes are created empty by default. This function needs to take
        // care of creating the desired inputs and outputs based on the template

        let type_info = registry().expect_get(*self);
        connect_things(type_info, node_id, graph);
    }
}

impl GuiNodeTemplate {
    fn build_empty_node(
        &self,
        _graph: &mut GuiGraph,
        _user_state: &mut GuiGraphState,
        _node_id: NodeId,
    ) {
    }
}

pub fn connect_things(
    type_info: &NodeTypeInfo,
    node_id: NodeId,
    g: &mut Graph<GuiNodeData, GuiConnectionDataType, GuiValueType>,
) {
    let default = (type_info.default_fn)();
    default.outputs().iter().for_each(|&(_, n)| {
        g.add_output_param(node_id, n.to_string(), GuiConnectionDataType::Scalar);
    });
    let _name = type_info.node_name;
    default.inputs().iter().for_each(|&(idx, n)| {
        g.add_input_param(
            node_id,
            n.to_string(),
            GuiConnectionDataType::Scalar,
            GuiValueType::Scalar {
                value: default.read_input(idx),
            },
            InputParamKind::ConnectionOrConstant,
            true,
        );
    });
}

pub struct AllMyNodeTemplates;
impl NodeTemplateIter for AllMyNodeTemplates {
    type Item = GuiNodeTemplate;

    fn all_kinds(&self) -> Vec<Self::Item> {
        // This function must return a list of node kinds, which the node finder
        // will use to display it to the user. Crates like strum can reduce the
        // boilerplate in enumerating all variants of an enum.
        let mut kinds = Vec::new();
        for node_type in &registry().node_types {
            if node_type.can_create_in_ui {
                kinds.push(node_type.id);
            }
        }
        kinds
    }
}

impl WidgetValueTrait for GuiValueType {
    type Response = GuiResponse;
    type UserState = GuiGraphState;
    type NodeData = GuiNodeData;
    fn value_widget(
        &mut self,
        param_name: &str,
        _node_id: NodeId,
        ui: &mut egui::Ui,
        _user_state: &mut GuiGraphState,
        _node_data: &GuiNodeData,
    ) -> Vec<GuiResponse> {
        // This trait is used to tell the library which UI to display for the
        // inline parameter widgets.
        if _node_data.show_details {
            match self {
                GuiValueType::Scalar { value } => {
                    ui.horizontal(|ui| {
                        ui.label(param_name);
                        ui.add(
                            Knob::new(value)
                                .speed(10.0 / 300.0)
                                .clamp_range(0.0..=10.0)
                                .with_id(_node_id),
                        );
                    });
                }
            }
        }
        // This allows you to return your responses from the inline widgets.
        Vec::new()
    }
}

impl UserResponseTrait for GuiResponse {}
impl NodeDataTrait for GuiNodeData {
    type Response = GuiResponse;
    type UserState = GuiGraphState;
    type DataType = GuiConnectionDataType;
    type ValueType = GuiValueType;

    // This method will be called when drawing each node. This allows adding
    // extra ui elements inside the nodes. In this case, we create an "active"
    // button which introduces the concept of having an active node in the
    // graph. This is done entirely from user code with no modifications to the
    // node graph library.
    fn bottom_ui(
        &self,
        ui: &mut egui::Ui,
        node_id: NodeId,
        graph: &Graph<GuiNodeData, GuiConnectionDataType, GuiValueType>,
        app_state: &mut Self::UserState,
    ) -> Vec<NodeResponse<GuiResponse, GuiNodeData>>
    where
        GuiResponse: UserResponseTrait,
    {
        let gui_node = &graph[node_id];
        let mut lol_graph = app_state.lol_graph.lock().unwrap();
        let lol_idx = gui_node.user_data.lol_node_idx.unwrap();
        let mut responses: Vec<NodeResponse<GuiResponse, GuiNodeData>> = vec![];
        //ui.add(egui::Checkbox::new(&mut gui_node.user_data.show_details, "details"));
        let is_active = gui_node.user_data.show_details;

        if !is_active {
            if ui.button("👁 Detailed").clicked() {
                responses.push(NodeResponse::User(GuiResponse::EnableDetails(node_id)));
            }
        } else {
            let button =
                egui::Button::new(egui::RichText::new("👁 No details").color(egui::Color32::BLACK))
                    .fill(egui::Color32::GOLD);
            if ui.add(button).clicked() {
                responses.push(NodeResponse::User(GuiResponse::DisableDetails(node_id)));
            }
        }

        if lol_graph.has_node(lol_idx) {
            let mut lol_node = lol_graph.get_node_mut(lol_idx);

            let node_type_info = registry().expect_get(gui_node.user_data.template);
            if let Some(drawer) = node_type_info.node_drawer {
                let drawer = drawer(&mut **lol_node);
                let mut drawer_respondes = drawer.draw(ui, node_id, graph, app_state);
                responses.append(&mut drawer_respondes);
            }
        }

        responses
    }
}

pub struct NodeRegistry {
    pub node_types: Vec<NodeTypeInfo>,
}
impl NodeRegistry {
    pub fn expect_get(&self, id: GuiNodeTemplate) -> &NodeTypeInfo {
        self.get(id)
            .unwrap_or_else(|| panic!("failed to find node type info for {:?}", id))
    }
    pub fn get(&self, id: GuiNodeTemplate) -> Option<&NodeTypeInfo> {
        for node_type in &self.node_types {
            if node_type.id == id {
                return Some(node_type);
            }
        }
        None
    }
}

// replace with once_cell for safe code
static REGISTRY: OnceLock<NodeRegistry> = OnceLock::new();

pub fn registry<'a>() -> &'a NodeRegistry {
    REGISTRY.get_or_init(|| NodeRegistry::default())
}

impl Default for NodeRegistry {
    fn default() -> Self {
        use GuiNodeTemplate::*;
        Self {
            node_types: vec![
                NodeTypeInfo::new::<lolgraph::Add>(GuiAdd),
                NodeTypeInfo::new::<lolgraph::Bias>(GuiBias),
                NodeTypeInfo::new::<lolgraph::Envelope>(GuiEnvelope),
                NodeTypeInfo::new::<lolgraph::Highpass>(GuiHighpass),
                NodeTypeInfo::new::<lolgraph::Key>(GuiKey),
                NodeTypeInfo::new::<lolgraph::VoiceKey>(GuiVoiceKey),
                NodeTypeInfo::new::<lolgraph::Lowpass>(GuiLowpass),
                NodeTypeInfo::new_uncreatable::<lolgraph::Out>(GuiAudioOut)
                    .with_node_drawer::<lolgraph::Out>(),
                NodeTypeInfo::new::<lolgraph::Reverb>(GuiReverb),
                NodeTypeInfo::new::<lolgraph::SawOsc>(GuiSawOsc),
                NodeTypeInfo::new::<lolgraph::Scale>(GuiScale),
                NodeTypeInfo::new::<lolgraph::SineOsc>(GuiSineOsc),
                NodeTypeInfo::new::<lolgraph::Group>(GuiGroup)
                    .with_node_drawer::<lolgraph::Group>(),
            ],
        }
    }
}

/// The graph 'global' state. This state struct is passed around to the node and
/// parameter drawing callbacks. The contents of this struct are entirely up to
/// the user. For this example, we use it to keep track of the 'active' node.

type VoiceId = usize;
struct VoiceAllocator {
    voices: Vec<VoiceId>,
    next_voice: usize,
}

impl VoiceAllocator {
    pub fn new(n_voices: usize) -> Self {
        Self {
            voices: (0..n_voices).collect(),
            next_voice: 0,
        }
    }

    pub fn get_voice(&mut self) -> usize {
        let ret = self.next_voice;
        self.next_voice = (self.next_voice + 1) % self.voices.len();
        ret
    }
}

//type GraphId = usize;
slotmap::new_key_type! { pub struct GraphId; }
pub struct NodeGraphExample {
    // The `GraphEditorState` is the top-level object. You "register" all your
    // custom types by specifying it as its generic parameters.
    gui_state: SlotMap<GraphId, GuiEditorState>,
    graph_stack: Vec<GraphId>,
    mother_graph: GraphId,
    //active_graph_stack: Vec<GraphId>,
    //gui_states: Vec<GuiEditorState>,
    app_state: GuiGraphState,
}

pub fn delete_node(
    gui_state: &mut GuiEditorState,
    lol_graph_opt: Option<&SharedGraph>,
    node_id: &NodeId,
) {
    let node = gui_state.graph.nodes.get(*node_id).unwrap();
    if let Some(lol_graph) = lol_graph_opt {
        let lol_node_idx = node.user_data.lol_node_idx.unwrap();
        lol_graph.lock().unwrap().remove(lol_node_idx);
    }
    gui_state.graph.remove_node(*node_id);
    gui_state.node_positions.remove(*node_id);
    // Make sure to not leave references to old nodes hanging
    gui_state.selected_nodes.retain(|id| *id != *node_id);
    gui_state.node_order.retain(|id| *id != *node_id);
}

#[cfg(feature = "persistence")]
const PERSISTENCE_KEY: &str = "egui_node_graph";

impl NodeGraphExample {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        shared_graph: lolgraph::SharedGraph,
        out_idx: usize,
    ) -> Self {
        let sg = shared_graph.clone();

        // {
        // let mut foo = &state.graph;
        let mut user_state = GuiGraphState {
            lol_graph: sg,
            out_node_idx: out_idx,
            keys_to_voice: HashMap::new(),
            save_name: Default::default(),
            last_reload_time: None,
            patch_files: Vec::new(),
            current_patch: None,
            voice_allocator: VoiceAllocator::new(lolgraph::voice_key::N_VOICES),
            next_graph_id: 1,
            detailed_nodes: HashSet::new(),
        };
        let (state, mother_graph) = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, PERSISTENCE_KEY))
            .unwrap_or({
                let mut state: GraphEditorState<
                    GuiNodeData,
                    GuiConnectionDataType,
                    GuiValueType,
                    GuiNodeTemplate,
                    GuiGraphState,
                > = Default::default();
                state
                    .pan_zoom
                    .adjust_zoom(2.0, Vec2::new(0.0, 0.0), 0.0, 3.0);
                let a = GuiNodeTemplate::GuiAudioOut;
                let node_id = state.graph.add_node(
                    "Output".to_string(),
                    GuiNodeData {
                        template: GuiNodeTemplate::GuiAudioOut,
                        show_details: true,
                        lol_node_idx: Some(out_idx),
                        buff: None,
                        graph_id: None,
                        sub_graph_id: None,
                        sub_graph_inputs: None,
                        sub_graph_outputs: None,
                    },
                    |graph, node_id| {
                        GuiNodeTemplate::build_node(&a, graph, &mut user_state, node_id)
                    },
                );
                state.node_order.push(node_id);
                state
                    .node_positions
                    .insert(node_id, epaint::Pos2::new(100.0, 100.0));
                // info!("Loaded from serialized state");
                let mut graphs: SlotMap<GraphId, GuiEditorState> = SlotMap::default();
                let mother_graph = graphs.insert(state);
                (graphs, mother_graph)
            });
        // }

        let mut ret = Self {
            gui_state: state,
            app_state: user_state,
            mother_graph,
            graph_stack: vec![mother_graph],
        };
        ret.mirror_graph_new();
        ret
    }
}
fn create_node(
    template: GuiNodeTemplate,
    node: NodeId,
    node_registry: &NodeRegistry,
    state: &mut GuiEditorState,
    lol_graph: &mut lolgraph::SharedGraph,
) {
    let mut new_node = None;
    for node_type in &node_registry.node_types {
        if node_type.id == template && node_type.can_create_in_ui {
            new_node = Some((node_type.default_fn)());
            break;
        }
    }

    let new_node =
        new_node.unwrap_or_else(|| panic!("failed to create node of type {:?}", template));
    let instance_idx = lol_graph.lock().unwrap().add(new_node);
    state
        .graph
        .nodes
        .get_mut(node)
        .unwrap()
        .user_data
        .lol_node_idx = Some(instance_idx);
}

fn mirror_node(app_state: &mut GuiGraphState, graph_editor: &mut GuiEditorState, node_id: NodeId) {
    let template = graph_editor.graph.nodes[node_id].user_data.template.clone();
    if let GuiNodeTemplate::GuiAudioOut = template {
        graph_editor.graph.nodes[node_id].user_data.lol_node_idx = Some(app_state.out_node_idx)
    } else {
        create_node(
            template,
            node_id,
            registry(),
            graph_editor,
            &mut app_state.lol_graph,
        );
    }
}

impl NodeGraphExample {
    fn mirror_nodes(&mut self) {
        let node_order = self
            .gui_state
            .iter()
            .map(|(graph_id, graph)| {
                graph.graph.nodes.iter().map(move |(node_id, node)| {
                    (graph_id, node_id, node.user_data.lol_node_idx.unwrap())
                })
            })
            .flatten()
            .sorted_by_key(|(_, _, lol_node_idx)| *lol_node_idx);
        for (graph_id, node_id, _) in node_order {
            let graph_editor_state = &mut self.gui_state.get_mut(graph_id).unwrap();
            mirror_node(&mut self.app_state, graph_editor_state, node_id);
        }
    }

    fn mirror_connections(&mut self, graph: GraphId) {
        let graph_state = self.gui_state.get(graph).unwrap();
        let input_keys: Vec<_> = graph_state.graph.connections.keys().collect();
        let mut inputs: Vec<InputId> = vec![];
        let mut to_be_connected: Vec<(InputId, OutputId)> = Vec::new();
        for input in input_keys {
            let output = graph_state.graph.connections[input];
            to_be_connected.push((input, output));
            if !inputs.contains(&input) {
                inputs.push(input);
            } else {
                println!("Connection already present");
            }
        }
        to_be_connected.iter().for_each(|&(input, output)| {
            self.connect_nodes(graph, input, output);
        });
    }

    fn mirror_graph_new(&mut self) {
        self.mirror_nodes();
        self.app_state.lol_graph.lock().unwrap().print();
        let graph_ids = self.gui_state.keys().collect_vec();
        for graph_id in graph_ids {
            self.mirror_connections(graph_id)
        }
    }

    fn get_lol_input_idxs(&self, graph: GraphId, input: InputId) -> (usize, usize) {
        let input_node = self
            .gui_state
            .get(graph)
            .unwrap()
            .graph
            .inputs
            .get(input)
            .unwrap()
            .node;
        // id of port in eguinodegraph::inputs
        let gui_graph = &self.gui_state.get(graph).unwrap().graph;
        let _node = &gui_graph.nodes[input_node];

        if let Some(input_map) = &gui_graph.nodes[input_node].user_data.sub_graph_inputs {
            println!("{:?}", gui_graph.nodes[input_node].user_data.template);
            let unconnected_port = input_map.get(&input).unwrap();
            return (unconnected_port.node_idx, unconnected_port.port_idx);
        }

        let lol_input_node_idx = gui_graph.nodes[input_node].user_data.lol_node_idx.unwrap();

        let lol_node_inputs = self
            .app_state
            .lol_graph
            .lock()
            .unwrap()
            .get_node(lol_input_node_idx)
            .inputs();

        let (input_name, _) = gui_graph.nodes[input_node]
            .inputs
            .iter()
            .filter(|&&(_, id)| id == input)
            .last()
            .unwrap();

        let (lol_input_port_id, _) = lol_node_inputs
            .iter()
            .filter(|&&(_, lol_input_name)| lol_input_name == input_name)
            .last()
            .unwrap();
        (lol_input_node_idx, *lol_input_port_id)
    }

    fn get_lol_output_idxs(&self, graph: GraphId, output: OutputId) -> (usize, usize) {
        let output_node = self
            .gui_state
            .get(graph)
            .unwrap()
            .graph
            .outputs
            .get(output)
            .unwrap()
            .node;
        // id of port in eguinodegraph::outputs
        let gui_graph = &self.gui_state.get(graph).unwrap().graph;

        if let Some(output_map) = &gui_graph.nodes[output_node].user_data.sub_graph_outputs {
            let unconnected_port = output_map.get(&output).unwrap();
            return (unconnected_port.node_idx, unconnected_port.port_idx);
        }

        let lol_output_node_idx = gui_graph.nodes[output_node].user_data.lol_node_idx.unwrap();

        let lol_node_outputs = self
            .app_state
            .lol_graph
            .lock()
            .unwrap()
            .get_node(lol_output_node_idx)
            .outputs();

        let (output_name, _) = gui_graph.nodes[output_node]
            .outputs
            .iter()
            .filter(|&&(_, id)| id == output)
            .last()
            .unwrap();

        let (lol_output_port_id, _) = lol_node_outputs
            .iter()
            .filter(|&&(_, lol_output_name)| lol_output_name == output_name)
            .last()
            .unwrap();
        (lol_output_node_idx, *lol_output_port_id)
    }

    fn disconnect_nodes(&mut self, graph: GraphId, input: InputId, output: OutputId) {
        let (lol_input_node_idx, lol_input_port_id) = self.get_lol_input_idxs(graph, input);
        let (lol_output_node_idx, lol_output_port_id) = self.get_lol_output_idxs(graph, output);

        self.app_state.lol_graph.lock().unwrap().disconnect(
            (lol_output_node_idx, lol_output_port_id),
            (lol_input_node_idx, lol_input_port_id),
        );
    }

    fn connect_nodes(&mut self, graph: GraphId, input: InputId, output: OutputId) {
        let (lol_input_node_idx, lol_input_port_id) = self.get_lol_input_idxs(graph, input);
        let (lol_output_node_idx, lol_output_port_id) = self.get_lol_output_idxs(graph, output);
        self.app_state
            .lol_graph
            .lock()
            .unwrap()
            .disconnect_input_port((lol_input_node_idx, lol_input_port_id));
        self.app_state.lol_graph.lock().unwrap().connect(
            (lol_output_node_idx, lol_output_port_id),
            (lol_input_node_idx, lol_input_port_id),
        );
    }
}

impl GuiGraphState {
    fn update_patches(&mut self) {
        self.patch_files = glob("*.gui_patch")
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

pub fn duplicate_node_to_graph(
    app_state: &mut GuiGraphState,
    gui_state: &SlotMap<GraphId, GuiEditorState>,
    source_graph_id: GraphId,
    source_node_id: NodeId,
    target_graph: &mut GuiEditorState,
) -> (
    NodeId,
    HashMap<InputId, InputId>,
    HashMap<OutputId, OutputId>,
) {
    let gui_node = gui_state
        .get(source_graph_id)
        .unwrap()
        .graph
        .nodes
        .get(source_node_id)
        .unwrap();
    let a = gui_node.user_data.template;
    let lol_node_idx = gui_node.user_data.lol_node_idx;
    let label = gui_state
        .get(source_graph_id)
        .unwrap()
        .graph
        .nodes
        .get(source_node_id)
        .unwrap()
        .label
        .clone();
    let node_id = target_graph.graph.add_node(
        label,
        GuiNodeData {
            template: a,
            show_details: true,
            lol_node_idx,
            buff: None,
            graph_id: None,
            sub_graph_id: gui_node.user_data.sub_graph_id,
            sub_graph_inputs: gui_node.user_data.sub_graph_inputs.clone(),
            sub_graph_outputs: gui_node.user_data.sub_graph_outputs.clone(),
        },
        |graph, node_id| GuiNodeTemplate::build_empty_node(&a, graph, app_state, node_id),
    );
    let mut input_map: HashMap<InputId, InputId> = HashMap::new();
    let mut output_map: HashMap<OutputId, OutputId> = HashMap::new();
    gui_node.inputs.iter().for_each(|(name, input_id)| {
        let input = gui_state
            .get(source_graph_id)
            .unwrap()
            .graph
            .inputs
            .get(*input_id)
            .unwrap();
        let new_input_id = target_graph.graph.add_input_param(
            node_id,
            name.clone(),
            GuiConnectionDataType::Scalar,
            input.value,
            input.kind,
            input.shown_inline,
        );
        input_map.insert(*input_id, new_input_id);
    });
    gui_node.outputs.iter().for_each(|(name, output_id)| {
        let _output = gui_state
            .get(source_graph_id)
            .unwrap()
            .graph
            .outputs
            .get(*output_id)
            .unwrap();
        let new_output_id = target_graph.graph.add_output_param(
            node_id,
            name.clone(),
            GuiConnectionDataType::Scalar,
        );
        output_map.insert(*output_id, new_output_id);
    });
    //node_id_map.insert(*sel_node_idx, node_id);
    target_graph.node_order.push(node_id);
    target_graph.node_positions.insert(
        node_id,
        gui_state.get(source_graph_id).unwrap().node_positions[source_node_id],
    );
    (node_id, input_map, output_map)
}

impl NodeGraphExample {
    pub fn get_unconnected_ports_from_selection(
        &self,
    ) -> (
        Vec<lolgraph::UnconnectedInput>,
        Vec<lolgraph::UnconnectedOutput>,
    ) {
        let nodes = &self
            .gui_state
            .get(self.mother_graph)
            .unwrap()
            .selected_nodes;
        self.get_unconnected_ports(self.mother_graph, nodes)
    }

    fn get_unconnected_ports(
        &self,
        graph_id: GraphId,
        nodes: &Vec<NodeId>,
    ) -> (Vec<UnconnectedInput>, Vec<UnconnectedOutput>) {
        let unconnected_inputs = self.get_unconnected_inputs(nodes, graph_id);
        let unconnected_outputs = self.get_unconnected_outputs(nodes, graph_id);
        (unconnected_inputs, unconnected_outputs)
    }

    fn get_unconnected_outputs(
        &self,
        nodes: &Vec<NodeId>,
        graph_id: GraphId,
    ) -> Vec<UnconnectedOutput> {
        let unconnected_outputs: Vec<_> = nodes
            .iter()
            .map(|sel_id| {
                let node = self
                    .gui_state
                    .get(graph_id)
                    .unwrap()
                    .graph
                    .nodes
                    .get(*sel_id)
                    .unwrap();
                match node.user_data.template {
                    GuiNodeTemplate::GuiGroup => {
                        let subgraph_id = node.user_data.sub_graph_id.unwrap();
                        let subgraph = self.gui_state.get(subgraph_id).unwrap();
                        self.get_unconnected_outputs(
                            &subgraph.graph.iter_nodes().collect_vec(),
                            subgraph_id,
                        )
                    }
                    _ => {
                        let lol_node_idx = node.user_data.lol_node_idx.unwrap();
                        let unconnected_ports = self
                            .app_state
                            .lol_graph
                            .lock()
                            .unwrap()
                            .get_unconnected_outputs_for_node(lol_node_idx);
                        unconnected_ports
                    }
                }
            })
            .flatten()
            .collect();
        unconnected_outputs
    }

    fn get_unconnected_inputs(
        &self,
        nodes: &Vec<NodeId>,
        graph_id: GraphId,
    ) -> Vec<UnconnectedInput> {
        let unconnected_inputs: Vec<_> = nodes
            .iter()
            .map(|sel_id| {
                let node = self
                    .gui_state
                    .get(graph_id)
                    .unwrap()
                    .graph
                    .nodes
                    .get(*sel_id)
                    .unwrap();
                match node.user_data.template {
                    GuiNodeTemplate::GuiGroup => {
                        let subgraph_id = node.user_data.sub_graph_id.unwrap();
                        let subgraph = self.gui_state.get(subgraph_id).unwrap();
                        self.get_unconnected_inputs(
                            &subgraph.graph.iter_nodes().collect_vec(),
                            subgraph_id,
                        )
                    }
                    _ => {
                        let lol_node_idx = node.user_data.lol_node_idx.unwrap();
                        let unconnected_ports = self
                            .app_state
                            .lol_graph
                            .lock()
                            .unwrap()
                            .get_unconnected_inputs_for_node(lol_node_idx);
                        unconnected_ports
                    }
                }
            })
            .flatten()
            .collect();
        unconnected_inputs
    }

    pub fn move_selected_nodes_to_subgraph(
        &mut self,
    ) -> (
        GraphEditorState<
            GuiNodeData,
            GuiConnectionDataType,
            GuiValueType,
            GuiNodeTemplate,
            GuiGraphState,
        >,
        Vec<(NodeId, String, f32)>,
        Pos2,
    ) {
        let mut subgraph_editor_state = GuiEditorState::new(0.2);
        //let sub_graph = &mut subgraph_editor_state.graph;
        let mut node_id_map: HashMap<NodeId, NodeId> = HashMap::new();

        let p = self
            .gui_state
            .get(self.mother_graph)
            .unwrap()
            .node_positions
            .get(
                *self
                    .gui_state
                    .get(self.mother_graph)
                    .unwrap()
                    .selected_nodes
                    .iter()
                    .nth(0)
                    .unwrap(),
            )
            .unwrap();

        let min_pos: Pos2 = self
            .gui_state
            .get(self.mother_graph)
            .unwrap()
            .node_positions
            .iter()
            .filter(|(id, _)| {
                self.gui_state
                    .get(self.mother_graph)
                    .unwrap()
                    .selected_nodes
                    .contains(id)
            })
            .fold(*p, |acc, (_, v)| acc.min(*v));

        let mut full_input_map = HashMap::new();
        let mut full_output_map = HashMap::new();
        for sel_node_idx in &self
            .gui_state
            .get(self.mother_graph)
            .unwrap()
            .selected_nodes
        {
            let source_graph_id = self.mother_graph.clone();
            let source_node_id = sel_node_idx.clone();
            let (new_node_id, input_map, output_map) = duplicate_node_to_graph(
                &mut self.app_state,
                &self.gui_state,
                source_graph_id,
                source_node_id,
                &mut subgraph_editor_state,
            );
            full_input_map.extend(input_map);
            full_output_map.extend(output_map);
            node_id_map.insert(*sel_node_idx, new_node_id);
        }
        let new_id_name_val = Self::extract_input_values(
            self.gui_state.get(self.mother_graph).unwrap(),
            &node_id_map,
        );
        {
            let gui_state = self.gui_state.get(self.mother_graph).unwrap();
            let graph = &self.gui_state.get(self.mother_graph).unwrap().graph;
            for (input_id, output_id) in
                graph.iter_connections().filter(|&(input_id, output_id)| {
                    let input_node_id = graph.get_input(input_id).node;
                    let output_node_id = graph.get_output(output_id).node;
                    gui_state.selected_nodes.contains(&input_node_id)
                        & gui_state.selected_nodes.contains(&output_node_id)
                })
            {
                let subgraph_output = full_output_map[&output_id];
                let subgraph_input = full_input_map[&input_id];
                subgraph_editor_state
                    .graph
                    .add_connection(subgraph_output, subgraph_input)
                //duplicate_connection(
                //    graph,
                //    input_id,
                //    output_id,
                //    &mut subgraph_editor_state,
                //    &node_id_map,
                //);
            }
        }

        for node_id in &self
            .gui_state
            .get(self.mother_graph)
            .unwrap()
            .selected_nodes
            .clone()
        {
            delete_node(
                &mut self.gui_state.get_mut(self.mother_graph).unwrap(),
                None,
                node_id,
            );
        }
        (subgraph_editor_state, new_id_name_val, min_pos)
    }

    pub fn disconnect_selected(
        &mut self,
    ) -> (
        Vec<(OutputId, UnconnectedInput)>,
        Vec<(UnconnectedOutput, InputId)>,
    ) {
        let mut connections_to_be_deleted: Vec<(InputId, OutputId)> = Vec::new();
        let graph = &self.gui_state.get(self.mother_graph).unwrap().graph;
        let gui_state = self.gui_state.get(self.mother_graph).unwrap();
        let mut outside_to_inside: Vec<(OutputId, UnconnectedInput)> = Vec::new();
        let mut inside_to_outside: Vec<(UnconnectedOutput, InputId)> = Vec::new();
        for (input_id, output_id) in graph.iter_connections() {
            let input_node_id = graph.get_input(input_id).node;
            let output_node_id = graph.get_output(output_id).node;
            if gui_state.selected_nodes.contains(&input_node_id)
                & gui_state.selected_nodes.contains(&output_node_id)
            {
            } else if gui_state.selected_nodes.contains(&input_node_id) {
                connections_to_be_deleted.push((input_id, output_id));
                let lol_input = self.get_lol_input_idxs(self.mother_graph, input_id);
                outside_to_inside.push((
                    output_id,
                    self.app_state
                        .lol_graph
                        .lock()
                        .unwrap()
                        .new_unconnected_input(lol_input),
                ));
            } else if gui_state.selected_nodes.contains(&output_node_id) {
                connections_to_be_deleted.push((input_id, output_id));
                let lol_output = self.get_lol_output_idxs(self.mother_graph, output_id);
                inside_to_outside.push((
                    self.app_state
                        .lol_graph
                        .lock()
                        .unwrap()
                        .new_unconnected_output(lol_output),
                    input_id,
                ));
            }
        }
        for (input_id, output_id) in connections_to_be_deleted {
            self.disconnect_nodes(self.mother_graph, input_id, output_id);
        }
        (outside_to_inside, inside_to_outside)
    }

    fn duplicate_selected_group(&mut self) {
        let node_clone = self
            .gui_state
            .get_mut(self.mother_graph)
            .unwrap()
            .graph
            .add_node(
                "Copy Group".to_string(),
                GuiNodeData {
                    template: GuiNodeTemplate::GuiGroup,
                    show_details: true,
                    lol_node_idx: None,
                    buff: None,
                    graph_id: Some(self.mother_graph),
                    sub_graph_id: None,
                    sub_graph_inputs: None,
                    sub_graph_outputs: None,
                },
                |graph, node_id| {
                    GuiNodeTemplate::build_node(
                        &GuiNodeTemplate::GuiGroup,
                        graph,
                        &mut self.app_state,
                        node_id,
                    )
                },
            );
        let mut node_id_map: HashMap<NodeId, NodeId> = HashMap::new();
        let graph_state = self.gui_state.get(self.mother_graph).unwrap();
        let sel_idx = graph_state.selected_nodes.iter().nth(0).unwrap().clone();
        let node = graph_state.graph.nodes.get(sel_idx).unwrap();
        if node.user_data.template == GuiNodeTemplate::GuiGroup {
            let mut remaining_subgraphs: Vec<_> = vec![node.user_data.sub_graph_id.unwrap()];
            while let Some(sub_graph_id) = remaining_subgraphs.pop() {
                let mut new_sub_graph = GuiEditorState::new(1.0);
                for node_id in self.gui_state.get(sub_graph_id).unwrap().graph.iter_nodes() {
                    let (new_node_id, _input_map, _output_map) = duplicate_node_to_graph(
                        &mut self.app_state,
                        &self.gui_state,
                        sub_graph_id,
                        node_id,
                        &mut new_sub_graph,
                    );
                    node_id_map.insert(node_id, new_node_id);
                    mirror_node(&mut self.app_state, &mut new_sub_graph, node_id);
                }
                let subgraph = &self.gui_state.get(sub_graph_id).unwrap().graph;
                for (input_id, output_id) in subgraph.iter_connections() {
                    duplicate_connection(
                        subgraph,
                        input_id,
                        output_id,
                        &mut new_sub_graph,
                        &node_id_map,
                    )
                }
                let new_sub_graph_id = self.gui_state.insert(new_sub_graph);
                self.mirror_connections(new_sub_graph_id);
                let nodes = self
                    .gui_state
                    .get(new_sub_graph_id)
                    .unwrap()
                    .graph
                    .iter_nodes()
                    .collect_vec();

                let (unconnected_inputs, unconnected_outputs) =
                    self.get_unconnected_ports(new_sub_graph_id, &nodes);
                let (
                    sub_graph_inputs,
                    _sub_graph_reverse_inputs,
                    sub_graph_outputs,
                    _sub_graph_reverse_outputs,
                ) = self.assign_subgraph_port_maps(
                    self.mother_graph,
                    unconnected_inputs,
                    node_clone,
                    unconnected_outputs,
                );
                let subgraph = self.gui_state.get(sub_graph_id).unwrap();
                let new_id_val = Self::extract_input_values(subgraph, &node_id_map);
                self.replicate_input_values(new_id_val, new_sub_graph_id);

                Self::get_mut_node(&mut self.gui_state, self.mother_graph, node_clone)
                    .user_data
                    .sub_graph_id = Some(new_sub_graph_id);
                self.set_subgraph_maps_for_node(node_clone, sub_graph_inputs, sub_graph_outputs);

                let mg = self.gui_state.get_mut(self.mother_graph).unwrap();
                mg.node_order.push(node_clone);
                let old_pos = mg.node_positions.get(sel_idx).unwrap();
                mg.node_positions
                    .insert(node_clone, *old_pos + Vec2::new(10.0, 10.0));
                create_node(
                    GuiNodeTemplate::GuiGroup,
                    node_clone,
                    registry(),
                    &mut self.gui_state.get_mut(self.mother_graph).unwrap(),
                    &mut self.app_state.lol_graph,
                );
            }
        }
    }

    fn replicate_input_values(
        &mut self,
        new_id_val: Vec<(NodeId, String, f32)>,
        //subgraph_id: GraphId,
        new_subgraph_id: GraphId,
    ) {
        //let subgraph = self.gui_state.get(subgraph_id).unwrap();
        //let new_id_val = Self::extract_input_values(subgraph, node_id_map);
        new_id_val.iter().for_each(|(node_id, name, value)| {
            let subgraph = self.gui_state.get_mut(new_subgraph_id).unwrap();
            let node = subgraph.graph.nodes.get(*node_id).unwrap();
            let new_input_id = node
                .inputs
                .iter()
                .find(|(new_name, _input_id)| new_name == name)
                .unwrap()
                .1;
            self.gui_state
                .get_mut(new_subgraph_id)
                .unwrap()
                .graph
                .inputs
                .get_mut(new_input_id)
                .unwrap()
                .value = GuiValueType::Scalar { value: *value };
        });
    }

    fn extract_input_values(
        subgraph: &GuiEditorState,
        node_id_map: &HashMap<NodeId, NodeId>,
    ) -> Vec<(NodeId, String, f32)> {
        let new_id_val: Vec<(NodeId, String, f32)> = subgraph
            .graph
            .inputs
            .iter()
            .filter(|(_input_id, param)| node_id_map.contains_key(&param.node))
            .map(|(input_id, param)| {
                let n = subgraph.graph.nodes.get(param.node).unwrap();
                let new_node_id = node_id_map.get(&param.node).unwrap();
                let (name, _) = n
                    .inputs
                    .iter()
                    .find(|(_name, named_input_id)| *named_input_id == input_id)
                    .unwrap();
                (new_node_id.clone(), name.clone(), param.value().value())
            })
            .collect();
        new_id_val
    }

    fn try_get_mut_node(
        gui_state: &mut SlotMap<GraphId, GuiEditorState>,
        graph_id: GraphId,
        node_clone: NodeId,
    ) -> Option<&mut Node<GuiNodeData>> {
        gui_state.get_mut(graph_id)?.graph.nodes.get_mut(node_clone)
    }

    fn get_mut_node(
        gui_state: &mut SlotMap<GraphId, GuiEditorState>,
        graph_id: GraphId,
        node_clone: NodeId,
    ) -> &mut Node<GuiNodeData> {
        gui_state
            .get_mut(graph_id)
            .unwrap()
            .graph
            .nodes
            .get_mut(node_clone)
            .unwrap()
        // .user_data
        // .sub_graph_id = Some(new_sub_graph_id)
    }

    pub fn group_selected(&mut self) {
        let (outside_to_inside, inside_to_outside) = self.disconnect_selected();

        // Get unconnected inputs and outputs from selected nodes
        let (unconnected_inputs, unconnected_outputs) = self.get_unconnected_ports_from_selection();

        // Create subgraph
        let (subgraph_editor_state, new_id_name_val, min_pos) =
            self.move_selected_nodes_to_subgraph();

        // Create GuiEditorState
        let sub_graph_id = self.gui_state.insert(subgraph_editor_state);

        self.replicate_input_values(new_id_name_val, sub_graph_id);
        let group_node_id = self
            .gui_state
            .get_mut(self.mother_graph)
            .unwrap()
            .graph
            .add_node(
                "Group".to_string(),
                GuiNodeData {
                    template: GuiNodeTemplate::GuiGroup,
                    show_details: true,
                    lol_node_idx: None,
                    buff: None,
                    graph_id: Some(self.mother_graph),
                    sub_graph_id: Some(sub_graph_id),
                    sub_graph_inputs: None,
                    sub_graph_outputs: None,
                },
                |graph, node_id| {
                    GuiNodeTemplate::build_node(
                        &GuiNodeTemplate::GuiGroup,
                        graph,
                        &mut self.app_state,
                        node_id,
                    )
                },
            );

        let (
            sub_graph_inputs,
            sub_graph_reverse_inputs,
            sub_graph_outputs,
            sub_graph_reverse_outputs,
        ) = self.assign_subgraph_port_maps(
            self.mother_graph,
            unconnected_inputs,
            group_node_id,
            unconnected_outputs,
        );

        self.set_subgraph_maps_for_node(group_node_id, sub_graph_inputs, sub_graph_outputs);

        self.app_state.next_graph_id += 1;
        self.gui_state
            .get_mut(self.mother_graph)
            .unwrap()
            .node_order
            .push(group_node_id);
        self.gui_state
            .get_mut(self.mother_graph)
            .unwrap()
            .node_positions
            .insert(group_node_id, min_pos);
        create_node(
            GuiNodeTemplate::GuiGroup,
            group_node_id,
            registry(),
            &mut self.gui_state.get_mut(self.mother_graph).unwrap(),
            &mut self.app_state.lol_graph,
        );

        // Reconnect subgraph to outside graph if relevant
        for (output_id, unconnected_input) in &outside_to_inside {
            let inside_id = sub_graph_reverse_inputs[unconnected_input];
            self.connect_nodes(self.mother_graph, inside_id, *output_id);
            self.gui_state
                .get_mut(self.mother_graph)
                .unwrap()
                .graph
                .add_connection(*output_id, inside_id);
        }
        for (unconnected_output, input_id) in &inside_to_outside {
            let output_id = sub_graph_reverse_outputs[unconnected_output];
            self.connect_nodes(self.mother_graph, *input_id, output_id);
            self.gui_state
                .get_mut(self.mother_graph)
                .unwrap()
                .graph
                .add_connection(output_id, *input_id);
        }
        self.app_state.lol_graph.lock().unwrap().print();
    }

    fn set_subgraph_maps_for_node(
        &mut self,
        group_node_id: NodeId,
        sub_graph_inputs: HashMap<InputId, UnconnectedInput>,
        sub_graph_outputs: HashMap<OutputId, UnconnectedOutput>,
    ) {
        self.gui_state
            .get_mut(self.mother_graph)
            .unwrap()
            .graph
            .nodes
            .get_mut(group_node_id)
            .unwrap()
            .user_data
            .sub_graph_inputs = Some(sub_graph_inputs);
        self.gui_state
            .get_mut(self.mother_graph)
            .unwrap()
            .graph
            .nodes
            .get_mut(group_node_id)
            .unwrap()
            .user_data
            .sub_graph_outputs = Some(sub_graph_outputs);
    }

    fn assign_subgraph_port_maps(
        &mut self,
        graph_id: GraphId,
        unconnected_inputs: Vec<UnconnectedInput>,
        group_node_id: NodeId,
        unconnected_outputs: Vec<UnconnectedOutput>,
    ) -> (
        HashMap<InputId, UnconnectedInput>,
        HashMap<UnconnectedInput, InputId>,
        HashMap<OutputId, UnconnectedOutput>,
        HashMap<UnconnectedOutput, OutputId>,
    ) {
        let mut sub_graph_inputs: HashMap<InputId, lolgraph::UnconnectedInput> = HashMap::new();
        let mut sub_graph_reverse_inputs: HashMap<lolgraph::UnconnectedInput, InputId> =
            HashMap::new();
        unconnected_inputs
            .into_iter()
            .for_each(|unconnected_input| {
                let input_id = self
                    .gui_state
                    .get_mut(graph_id)
                    .unwrap()
                    .graph
                    .add_input_param(
                        group_node_id,
                        unconnected_input.name.clone(),
                        GuiConnectionDataType::Scalar,
                        GuiValueType::Scalar { value: 0.0 }, // TODO: Update to fix group duplication input values
                        InputParamKind::ConnectionOrConstant,
                        true,
                    );
                sub_graph_inputs.insert(input_id, unconnected_input.clone());
                sub_graph_reverse_inputs.insert(unconnected_input, input_id);
            });

        let mut sub_graph_outputs = HashMap::new();
        let mut sub_graph_reverse_outputs = HashMap::new();
        unconnected_outputs
            .into_iter()
            .for_each(|unconnected_output| {
                let output_id = self
                    .gui_state
                    .get_mut(graph_id)
                    .unwrap()
                    .graph
                    .add_output_param(
                        group_node_id,
                        unconnected_output.name.clone(),
                        GuiConnectionDataType::Scalar,
                    );
                sub_graph_outputs.insert(output_id, unconnected_output.clone());
                sub_graph_reverse_outputs.insert(unconnected_output, output_id);
            });
        (
            sub_graph_inputs,
            sub_graph_reverse_inputs,
            sub_graph_outputs,
            sub_graph_reverse_outputs,
        )
    }
}

fn duplicate_connection(
    graph: &Graph<GuiNodeData, GuiConnectionDataType, GuiValueType>,
    input_id: InputId,
    output_id: OutputId,
    subgraph_editor_state: &mut GraphEditorState<
        GuiNodeData,
        GuiConnectionDataType,
        GuiValueType,
        GuiNodeTemplate,
        GuiGraphState,
    >,
    node_id_map: &HashMap<NodeId, NodeId>,
) {
    let input_node_id = graph.get_input(input_id).node;
    let output_node_id = graph.get_output(output_id).node;
    // Get name of input port in parent graph
    let outside_input_name = get_input_name(graph, input_node_id, input_id);

    // Get input port id in the subgraph
    let inside_input_id = get_subgraph_input_id(
        &*subgraph_editor_state,
        node_id_map,
        input_node_id,
        outside_input_name,
    );

    // Get name of output port in parent graph
    let outside_output_name = get_output_name(graph, output_node_id, output_id);

    // Get output port id in the subgraph
    let inside_output_id = get_subgraph_output_id(
        &*subgraph_editor_state,
        node_id_map,
        output_node_id,
        outside_output_name,
    );

    subgraph_editor_state
        .graph
        .add_connection(inside_output_id, inside_input_id);
}

fn get_subgraph_output_id(
    subgraph_editor_state: &GraphEditorState<
        GuiNodeData,
        GuiConnectionDataType,
        GuiValueType,
        GuiNodeTemplate,
        GuiGraphState,
    >,
    node_id_map: &HashMap<NodeId, NodeId>,
    output_node_id: NodeId,
    outside_output_name: &String,
) -> OutputId {
    let inside_output_id = subgraph_editor_state
        .graph
        .nodes
        .get(node_id_map[&output_node_id])
        .unwrap()
        .get_output(outside_output_name)
        .unwrap();
    inside_output_id
}

fn get_output_name(
    graph: &Graph<GuiNodeData, GuiConnectionDataType, GuiValueType>,
    output_node_id: NodeId,
    output_id: OutputId,
) -> &String {
    let outside_output_name = graph
        .nodes
        .get(output_node_id)
        .unwrap()
        .outputs
        .iter()
        .find_map(|(name, id)| if *id == output_id { Some(name) } else { None })
        .unwrap();
    outside_output_name
}

fn get_subgraph_input_id(
    subgraph_editor_state: &GraphEditorState<
        GuiNodeData,
        GuiConnectionDataType,
        GuiValueType,
        GuiNodeTemplate,
        GuiGraphState,
    >,
    node_id_map: &HashMap<NodeId, NodeId>,
    input_node_id: NodeId,
    outside_input_name: &String,
) -> InputId {
    let inside_input_id = subgraph_editor_state
        .graph
        .nodes
        .get(node_id_map[&input_node_id])
        .unwrap()
        .get_input(outside_input_name)
        .unwrap();
    inside_input_id
}

fn get_input_name(
    graph: &Graph<GuiNodeData, GuiConnectionDataType, GuiValueType>,
    input_node_id: NodeId,
    input_id: InputId,
) -> &String {
    let outside_input_name = graph
        .nodes
        .get(input_node_id)
        .unwrap()
        .inputs
        .iter()
        .find_map(|(name, id)| if *id == input_id { Some(name) } else { None })
        .unwrap();
    outside_input_name
}

impl eframe::App for NodeGraphExample {
    #[cfg(feature = "persistence")]
    /// If the persistence function is enabled,
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        println!("Serializing graph");
        eframe::set_value(storage, PERSISTENCE_KEY, &self.gui_state);
    }
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_pixels_per_point(2.0);
        ctx.request_repaint_after(Duration::from_secs_f64(1.0 / 30.0));
        let _patch_window = egui::Window::new("Patches").show(ctx, |ui| {
            {
                let Self {
                    app_state: this,
                    gui_state: graph_state,
                    mother_graph: _,
                    graph_stack: _,
                } = self;
                let _response =
                    ui.add(egui::TextEdit::singleline(&mut this.save_name).desired_width(100.0));
                if ui.add(egui::Button::new("save")).clicked() {
                    let g = this.lol_graph.lock().unwrap().copy();
                    let serialized = serde_json::to_string_pretty(&g).unwrap();
                    let serialized_gui = serde_json::to_string_pretty(&graph_state).unwrap();
                    let name = format!("{}.patch", this.save_name);
                    let name_gui = format!("{}.gui_patch", this.save_name);
                    let mut file = File::create(&name).unwrap();
                    file.write_all(serialized.as_bytes()).unwrap();
                    let mut gui_file = File::create(&name_gui).unwrap();
                    gui_file.write_all(serialized_gui.as_bytes()).unwrap();
                    this.current_patch = Some(name_gui);
                    this.last_reload_time = None;
                }
                if let Some(last_reload_time) = this.last_reload_time {
                    if last_reload_time.elapsed().as_secs() > 2 {
                        this.update_patches();
                    }
                } else {
                    this.update_patches();
                    this.last_reload_time = Some(Instant::now());
                }
            }
            for file in self.app_state.patch_files.clone() {
                if file.contains(&self.app_state.save_name) || self.app_state.save_name.is_empty() {
                    let patch_name = file.split(".gui_patch").nth(0).unwrap();
                    let color = match &self.app_state.current_patch {
                        Some(current_patch) => match &file {
                            path if path == current_patch => Color32::from_rgb(0, 128, 0),
                            _ => Default::default(),
                        },
                        _ => Default::default(),
                    };
                    if ui.add(egui::Button::new(patch_name).fill(color)).clicked() {
                        let file_contents = std::fs::read_to_string(&file).unwrap();
                        match serde_json::from_str(&file_contents) {
                            Ok(state) => {
                                self.gui_state = state;
                                self.app_state.lol_graph.lock().unwrap().clear();
                                self.mirror_graph_new();
                                self.app_state.current_patch = Some(file.clone());
                                self.app_state.save_name = patch_name[..].to_string();
                                self.gui_state.values_mut().for_each(|s| {
                                    s.pan_zoom.adjust_zoom(0.2, Vec2::new(0.0, 0.0), 0.0, 3.0)
                                });
                            }
                            Err(error) => println!("{:?}", error),
                        }
                    }
                }
            }
        });
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::widgets::global_dark_light_mode_switch(ui);
            });
        });
        let graph_response = egui::CentralPanel::default()
            .show(ctx, |ui| {
                self.gui_state
                    .get_mut(self.mother_graph)
                    .unwrap()
                    .draw_graph_editor(ui, AllMyNodeTemplates, &mut self.app_state)
            })
            .inner;
        let set_pitch = |voice: usize, pitch, trigger| {
            let mut g = self.app_state.lol_graph.lock().unwrap();
            let okey = g.get_by_type_mut::<lolgraph::VoiceKey>();
            match okey {
                Some(mut voice_key) => {
                    voice_key.pitch[voice] = pitch as f32;
                    voice_key.trigger[voice] = trigger as f32;
                }
                None => (),
            }
        };
        let tone = |x: f64| 0.44 / 4.0 * 2.0_f64.powf(x / 12.0);
        let key_to_tone = HashMap::from([
            (Key::A, 0.0),
            (Key::W, 1.0),
            (Key::S, 2.0),
            (Key::E, 3.0),
            (Key::D, 4.0),
            (Key::R, 5.0),
            (Key::F, 6.0),
            (Key::T, 7.0),
            (Key::G, 8.0),
            (Key::Y, 9.0),
            (Key::H, 10.0),
            (Key::U, 11.0),
            (Key::J, 12.0),
        ]);

        let mut create_voice = |key: Key, keys_to_voice: &mut HashMap<Key, usize>| {
            let voice = self.app_state.voice_allocator.get_voice();
            println!("Allocated voice {}", voice);
            set_pitch(voice, tone(key_to_tone[&key]), 1.0);
            keys_to_voice.insert(key, voice);
        };
        let release_voice = |key: Key, keys_to_voice: &mut HashMap<Key, usize>| {
            if keys_to_voice.contains_key(&key) {
                println!("Release voice {}", keys_to_voice[&key]);
                set_pitch(keys_to_voice[&key], tone(key_to_tone[&key]), 0.0);
                keys_to_voice.remove(&key);
            }
        };
        if ctx.input(|i| i.key_down(Key::Space)) {
            for key in key_to_tone.keys() {
                if ctx.input(|i| i.key_down(*key))
                    && !self.app_state.keys_to_voice.contains_key(key)
                {
                    create_voice(*key, &mut self.app_state.keys_to_voice);
                }
            }
        }
        for key in key_to_tone.keys() {
            if ctx.input(|i| i.key_released(*key)) {
                release_voice(*key, &mut self.app_state.keys_to_voice);
            }
        }

        if ctx.input(|i| i.key_released(Key::M)) {
            self.group_selected();
        }

        if ctx.input(|i| i.key_released(Key::B)) {
            self.duplicate_selected_group();
        }

        if ctx.input(|i| i.key_released(Key::ArrowUp)) {
            if let Some(graph_id) = self.graph_stack.pop() {
                self.mother_graph = graph_id;
                return;
            }
        }

        if ctx.input(|i| i.key_released(Key::N)) {
            if self
                .gui_state
                .get(self.mother_graph)
                .unwrap()
                .selected_nodes
                .len()
                == 1
            {
                let sel_id = self
                    .gui_state
                    .get(self.mother_graph)
                    .unwrap()
                    .selected_nodes
                    .iter()
                    .nth(0)
                    .unwrap();
                let node = self
                    .gui_state
                    .get(self.mother_graph)
                    .unwrap()
                    .graph
                    .nodes
                    .get(*sel_id);
                if let Some(sub_graph_id) = node.unwrap().user_data.sub_graph_id {
                    self.graph_stack.push(self.mother_graph);
                    self.mother_graph = sub_graph_id;
                    return;
                }
            }
        }

        // update unconnected input parameters but set by gui text component
        self.gui_state.values_mut().for_each(|gui_graph| {
            for (_node_id, _node_data) in &mut gui_graph.graph.nodes {
                {
                    if let Some(lol_idx) = _node_data.user_data.lol_node_idx {
                        if let Some(buffer) = self
                            .app_state
                            .lol_graph
                            .lock()
                            .unwrap()
                            .get_node(lol_idx)
                            .buff()
                        {
                            _node_data.user_data.buff = Some(buffer.clone());
                        }
                    }
                }
                //let node = //&self.state.graph.nodes[node_id];
                for (input_name, input_id) in &_node_data.inputs {
                    let inp = &gui_graph.graph.inputs[*input_id];
                    // if inp;

                    if gui_graph.graph.connections.contains_key(*input_id) {
                        continue;
                    }

                    let val = inp.value.value();

                    if let Some(_idx) = _node_data.user_data.lol_node_idx {
                        let node_id = _node_data.user_data.lol_node_idx.unwrap();
                        self.app_state
                            .lol_graph
                            .lock()
                            .unwrap()
                            .get_node_mut(node_id)
                            .set_by_name(input_name.as_str(), val);
                    }
                }
            }
        });

        for node_response in graph_response.node_responses {
            // Here, we ignore all other graph events. But you may find
            // some use for them. For example, by playing a sound when a new
            // connection is created
            if let NodeResponse::User(user_response) = node_response {
                match user_response {
                    GuiResponse::EnableDetails(node_id) => {
                        println!("Enable");
                        self.gui_state
                            .get_mut(self.mother_graph)
                            .unwrap()
                            .graph
                            .nodes
                            .get_mut(node_id)
                            .unwrap()
                            .user_data
                            .show_details = true;
                    }
                    GuiResponse::DisableDetails(node_id) => {
                        println!("Disable");
                        self.gui_state
                            .get_mut(self.mother_graph)
                            .unwrap()
                            .graph
                            .nodes
                            .get_mut(node_id)
                            .unwrap()
                            .user_data
                            .show_details = false;
                    }
                }
            }
            match node_response {
                NodeResponse::DeleteNodeFull { node_id: _, node } => {
                    println!("DeleteNode");
                    if let Some(sub_graph_id) = node.user_data.sub_graph_id {
                        let mut sub_graph = self.gui_state.get_mut(sub_graph_id).unwrap();
                        for node_id in sub_graph.graph.iter_nodes().collect_vec() {
                            delete_node(&mut sub_graph, Some(&self.app_state.lol_graph), &node_id)
                        }
                    }
                    let lol_node_idx = node.user_data.lol_node_idx.unwrap();
                    self.app_state
                        .lol_graph
                        .lock()
                        .unwrap()
                        .remove(lol_node_idx);
                }
                NodeResponse::DisconnectEvent { output, input } => {
                    println!("DisconnectEvent");
                    if self
                        .gui_state
                        .get(self.mother_graph)
                        .unwrap()
                        .graph
                        .inputs
                        .contains_key(input)
                        && self
                            .gui_state
                            .get(self.mother_graph)
                            .unwrap()
                            .graph
                            .outputs
                            .contains_key(output)
                    {
                        self.disconnect_nodes(self.mother_graph, input, output);
                    }
                }
                // NOTE output: the output port to which the connection applues
                //       input: the receiving end of the connection
                NodeResponse::ConnectEventEnded { output, input } => {
                    println!("ConnectEventEnded");
                    self.connect_nodes(self.mother_graph, input, output);
                }
                NodeResponse::CreatedNode(node) => {
                    let Self {
                        gui_state: state,
                        app_state: GuiGraphState { lol_graph, .. },
                        ..
                    } = self;
                    let created_node = state
                        .get_mut(self.mother_graph)
                        .unwrap()
                        .graph
                        .nodes
                        .get_mut(node)
                        .unwrap();
                    created_node.user_data.graph_id = Some(self.mother_graph);

                    create_node(
                        created_node.user_data.template,
                        node,
                        registry(),
                        state.get_mut(self.mother_graph).unwrap(),
                        lol_graph,
                    );
                }
                _ => {}
            }
        }
    }
}
