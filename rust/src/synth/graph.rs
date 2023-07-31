use serde::{Deserialize, Serialize};
use slotmap::SlotMap;
use std::any::Any;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Instant;

// use crate::signal;
//use std::fs::OpenOptions;

pub mod add;
pub mod bias;
pub mod envelope;
pub mod group;
pub mod hp;
pub mod key;
pub mod lp;
pub mod out;
pub mod reverb;
pub mod saw_osc;
pub mod scale;
pub mod sine_osc;
pub mod voice_key;

pub use add::*;
pub use bias::*;
pub use envelope::*;
pub use group::*;
pub use hp::*;
pub use key::*;
pub use lp::*;
pub use out::*;
pub use reverb::*;
pub use saw_osc::*;
pub use scale::*;
pub use sine_osc::*;
pub use voice_key::*;

slotmap::new_key_type! { pub struct ChannelId; }
slotmap::new_key_type! { pub struct NodeKey; }
// pub struct SharedGraph2<'a> {
//     g: Arc<RwLock<Graph>>,
// }

// impl<'a> Deref for SharedGraph2<'a> {
//     type Target = RwLockReadGuard<'a, Graph>;

//     fn deref(&self) -> &Self::Target {
//         let read_result = self.g.read().unwrap()
//         &read_result
//     }
// }

// impl DerefMut for SharedGraph2 {

// }

pub type SharedGraph = Arc<Mutex<Graph>>;
pub type SharedChannels = Arc<Mutex<SlotMap<ChannelId, SharedGraph>>>;

// Should be NodeInput
// pub type Input = (usize, &'static str);
// Should be NodeOutput
// pub type Output = (usize, &'static str);

#[derive(Clone)]
pub struct InputId {
    pub port: usize,
    pub name: &'static str,
}

impl From<(usize, &'static str)> for InputId {
    fn from(value: (usize, &'static str)) -> Self {
        Self {
            port: value.0,
            name: value.1,
        }
    }
}

#[derive(Clone)]
pub struct OutputId {
    pub port: usize,
    pub name: &'static str,
}

impl From<(usize, &'static str)> for OutputId {
    fn from(value: (usize, &'static str)) -> Self {
        Self {
            port: value.0,
            name: value.1,
        }
    }
}

#[typetag::serde(tag = "type")]
pub trait Node: Send + Any + 'static {
    fn type_name(&self) -> &'static str {
        core::any::type_name::<Self>()
    }
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn outputs(&self) -> Vec<OutputId>;
    fn inputs(&self) -> Vec<InputId>;
    fn read_input(&self, _idx: usize) -> f32 {
        0.0
    }
    fn set(&mut self, idx: usize, val: f32);
    fn get(&self, idx: usize) -> f32;
    fn get_input_mut(&mut self, idx: usize) -> &mut f32;
    fn step(&mut self, sample_rate: f32);
    fn name() -> &'static str
    where
        Self: Sized;
    fn copy(&self) -> Box<dyn Node>;

    fn buff(&self) -> Option<&VecDeque<f32>> {
        None
        //VecDeque::from(vec![0.0; 1000])
    }

    // Get output value for parameter n1
    fn get_by_name(&mut self, n1: &str) -> f32 {
        self.inputs()
            .iter()
            .filter(|port_id| n1 == port_id.name)
            .map(|port_id| self.get(port_id.port))
            .last()
            .unwrap()
    }

    // Set input for paramter n1 to value val
    fn set_by_name(&mut self, n1: &str, val: f32) {
        self.inputs()
            .iter()
            .filter(|port_id| n1 == port_id.name)
            .for_each(|port_id| self.set(port_id.port, val));
    }
}

macro_rules! valid_idx {
    ($ex:expr, $idx:expr, $max:expr) => {{
        match $idx {
            x if x < $max => $ex,
            _ => panic!("Unsupported idx"),
        }
    }};
}
pub(crate) use valid_idx;

// #[derive(Clone)]
// pub struct OutCallbacker {
//     spec: AudioSpec,
//     shared_graph: SharedGraph,
//     out_idx: usize,
//     last_time: Instant,
//     took: Duration,
// }

// pub fn create_shared_graph(
//     audio_subsystem: &mut AudioSubsystem,
// ) -> (SharedGraph, AudioDevice<OutCallbacker>, usize) {
//     let retn: Out = Default::default();
//     let shared_graph = Arc::new(Mutex::new(Graph::new()));
//     let out_idx = shared_graph.lock().unwrap().add(Box::new(retn));
//     let desired_spec = AudioSpecDesired {
//         freq: Some(44_100),
//         channels: Some(1),   // mono
//         samples: Some(1000), // default sample size
//     };

//     let device = audio_subsystem
//         .open_playback(None, &desired_spec, |spec| OutCallbacker {
//             spec,
//             shared_graph: shared_graph.clone(),
//             out_idx,
//             last_time: Instant::now(),
//             took: Duration::new(0, 0),
//         })
//         .unwrap();

//     device.resume();
//     (shared_graph, device, out_idx)
// }

// impl AudioCallback for OutCallbacker {
//     type Channel = f32;

//     fn callback(&mut self, sdl_out: &mut [f32]) {
//         let now = Instant::now();

//         for i in 0..sdl_out.len() {
//             let mut graph = self.shared_graph.lock().unwrap();
//             let output = graph.step(self.spec.freq as f32);

//             sdl_out[i] = 0.5 * signal::compress::compress(output);
//         }

//         self.took = self.last_time.elapsed();
//         self.last_time = Instant::now();

//         if self.took.as_secs_f32() > 3.0 {
//             println!("Warning very slow Graph::step()");
//         }
//     }
// }

#[derive(Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize, Debug))]
pub enum PortKind {
    Input,
    Output,
}

#[derive(Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize, Debug))]
pub struct Port {
    pub node: NodeKey,
    pub port: usize,
    pub kind: PortKind,
}

#[derive(Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Edge {
    pub from: Port,
    pub to: Port,
}

// type Edge = (NodeOutput, NodeInput);
type NodeId = usize;

#[derive(Serialize, Deserialize)]
pub struct Graph {
    nodes: SlotMap<NodeKey, RefCell<Box<dyn Node>>>,
    edges: Vec<Edge>,

    node_order: Vec<NodeKey>,
    node_outputs: HashMap<NodeKey, Vec<Edge>>,
    node_inputs: HashMap<NodeKey, Vec<Edge>>,

    output_node: Option<NodeKey>,

    pub volume: f32,
    pub steps: u64,

    #[serde(with = "serde_millis")]
    pub ctime: Instant,
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq, Clone)]
pub struct UnconnectedInput {
    pub node_key: NodeKey,
    pub port_idx: usize,
    pub name: String,
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq, Clone)]
pub struct UnconnectedOutput {
    pub node_key: NodeKey,
    pub port_idx: usize,
    pub name: String,
}

impl Graph {
    pub fn print(&self) {
        for (node_idx, node) in self.nodes.values().enumerate() {
            println!("{}: {}", node_idx, node.borrow().type_name());
        }
    }
    pub fn add(&mut self, node: Box<dyn Node>) -> NodeKey {
        println!("Adding {}", node.type_name());
        let key = self.nodes.insert(RefCell::new(node));
        self.sort();
        key
        // self.nodes.len() - 1
    }

    pub fn get_node(&self, node: NodeKey) -> impl core::ops::Deref<Target = Box<dyn Node>> + '_ {
        self.nodes[node].borrow()
    }

    pub fn get_node_mut(
        &self,
        node_key: NodeKey,
    ) -> impl core::ops::DerefMut<Target = Box<dyn Node>> + '_ {
        self.nodes[node_key].borrow_mut()
    }

    pub fn has_node(&self, node_idx: NodeId) -> bool {
        node_idx < self.nodes.len()
    }

    /// Connects two nodes with an edge from `input` `(id, port)` to `output` `(id, port)`
    pub fn format_edge(&self, edge_idx: usize) -> String {
        let e = &self.edges[edge_idx];
        self.format_edge_pair(e)
    }
    pub fn format_edge_pair(&self, edge: &Edge) -> String {
        let from_node_type = self.nodes[edge.from.node].borrow().type_name();
        let froms = self.nodes[edge.from.node].borrow().outputs();
        let from_port_name = froms[edge.from.port].name;
        let to_node_type = self.nodes[edge.to.node].borrow().type_name();
        let tos = self.nodes[edge.to.node].borrow().inputs();
        let to_port_name = tos[edge.to.port].name;
        format!(
            "[{}] {} -> {} [{}]",
            from_node_type, from_port_name, to_port_name, to_node_type
        )
    }

    pub fn connect(&mut self, from: Port, to: Port) {
        self.edges.push(Edge {
            from: from.clone(),
            to: to.clone(),
        });
        println!("Connected {}", self.format_edge_pair(&Edge { from, to }));
        self.sort();
    }

    pub fn get_edge(&self, to: Port) -> Option<Edge> {
        self.edges
            .iter()
            .filter(|edge| edge.to == to)
            .last()
            .cloned()
    }

    pub fn disconnect_edge(&mut self, edge: Edge) {
        self.disconnect(edge.from, edge.to);
    }

    pub fn disconnect(&mut self, from: Port, to: Port) {
        if let Some(idx) = self
            .edges
            .iter()
            .position(|edge| edge.from == from && edge.to == to)
        {
            println!("Disconnecting {}", self.format_edge(idx));
            self.edges.remove(idx);
        } else {
            println!("Couldn't find the edge!");
        }
        self.sort();
    }

    pub fn disconnect_input_port(&mut self, input: Port) {
        if let Some(idx) = self.edges.iter().position(|edge| edge.to == input) {
            println!("Disconnecting {}", self.format_edge(idx));
            self.edges.remove(idx);
        }
        self.sort();
    }

    pub fn remove(&mut self, node_key: NodeKey) {
        println!("Removing {}", self.nodes[node_key].borrow().type_name());
        self.edges
            .retain(|edge| !(edge.from.node == node_key || edge.to.node == node_key));
        self.nodes.remove(node_key);
        self.sort();
        // NOTE: should we free box here?
    }

    pub fn clear(&mut self) {
        self.edges.clear();
        self.nodes.clear();
        self.add(Box::new(Out::default()));
        // _ = self.nodes.split_off(1);
        self.sort();
    }

    pub fn new() -> Self {
        let g = Graph {
            nodes: SlotMap::with_key(),
            edges: vec![],
            node_order: vec![],
            node_outputs: HashMap::new(),
            node_inputs: HashMap::new(),
            output_node: None,
            volume: 1.0,
            steps: 0,
            ctime: Instant::now(),
        };
        g
    }

    pub fn copy(&self) -> Self {
        // panic!();
        let mut node_lookup: HashMap<NodeKey, NodeKey> = HashMap::new();
        let mut edge_lookup: HashMap<Edge, Edge> = HashMap::new();
        let mut new_sm: SlotMap<NodeKey, _> = SlotMap::with_key();
        self.nodes.iter().for_each(|(key, node)| {
            let new_key = new_sm.insert(RefCell::new(node.borrow().copy()));
            node_lookup.insert(key, new_key);
        });
        let mut new_edges: Vec<Edge> = Vec::new();
        self.edges.iter().for_each(|edge| {
            let new_edge = Edge {
                from: Port {
                    node: node_lookup[&edge.from.node],
                    port: edge.from.port,
                    kind: edge.from.kind.clone(),
                },
                to: Port {
                    node: node_lookup[&edge.to.node],
                    port: edge.to.port,
                    kind: edge.to.kind.clone(),
                },
            };
            new_edges.push(new_edge.clone());
            edge_lookup.insert(edge.clone(), new_edge);
        });
        let mut graph = Graph {
            nodes: new_sm,
            edges: new_edges,
            node_order: self
                .node_order
                .iter()
                .map(|node_key| node_lookup[node_key])
                .collect(),
            node_outputs: HashMap::new(),
            node_inputs: HashMap::new(),
            output_node: self.output_node.map(|node_key| node_lookup[&node_key]),
            volume: self.volume,
            steps: self.steps,
            ctime: Instant::now(),
        };
        graph.sort();
        graph
    }

    pub fn get_by_type_mut<T: Node>(&mut self) -> Option<core::cell::RefMut<'_, T>> {
        for n in &mut self.nodes.values() {
            let n = core::cell::RefMut::filter_map(n.borrow_mut(), |n| {
                let v: &mut dyn Any = n.as_any_mut();

                v.downcast_mut::<T>()
            });
            if let Ok(v) = n {
                return Some(v);
            }
        }
        None
    }

    pub fn new_unconnected_output(&self, output: Port) -> UnconnectedOutput {
        UnconnectedOutput {
            node_key: output.node,
            port_idx: output.port,
            name: self.nodes[output.node].borrow().outputs()[output.port]
                .name
                .to_string(),
        }
    }

    pub fn new_unconnected_input(&self, input: Port) -> UnconnectedInput {
        UnconnectedInput {
            node_key: input.node,
            port_idx: input.port,
            name: self.nodes[input.node].borrow().inputs()[input.port]
                .name
                .to_string(),
        }
    }

    pub fn get_unconnected_outputs_for_node(&self, node_key: NodeKey) -> Vec<UnconnectedOutput> {
        self.nodes[node_key]
            .borrow()
            .outputs()
            .iter()
            .filter(|port_id| {
                self.edges
                    .iter()
                    .filter(move |edge| {
                        (edge.from.node == node_key) && (edge.from.port == port_id.port)
                    })
                    .count()
                    == 0
            })
            .map(|port_id| UnconnectedOutput {
                node_key,
                port_idx: port_id.port,
                name: port_id.name.to_string(),
            })
            .collect()
    }

    pub fn get_unconnected_inputs_for_node(&self, node_key: NodeKey) -> Vec<UnconnectedInput> {
        self.nodes[node_key]
            .borrow()
            .inputs()
            .iter()
            .filter(|port_id| {
                self.edges
                    .iter()
                    .filter(move |edge| {
                        (edge.to.node == node_key) && (edge.to.port == port_id.port)
                    })
                    .count()
                    == 0
            })
            .map(|port_id| UnconnectedInput {
                node_key,
                port_idx: port_id.port,
                name: port_id.name.to_string(),
            })
            .collect()
    }

    pub fn get_unconnected_inputs(&self) -> Vec<UnconnectedInput> {
        let unconnected_inputs: Vec<_> = self
            .nodes
            .keys()
            .map(|node_key| self.get_unconnected_inputs_for_node(node_key))
            .flatten()
            .collect();
        unconnected_inputs
    }

    pub fn sort(&mut self) {
        let mut new_node_order = vec![];
        let mut new_output_edges = HashMap::new();
        let mut new_input_edges = HashMap::new();

        // Find nodes without connected inputs, these are the initial nodes
        let mut node_idxs: Vec<_> = self
            .nodes
            .keys()
            .filter(|node_key| {
                self.edges
                    .iter()
                    .filter(|edge| edge.to.node == *node_key)
                    .count()
                    == 0
            })
            .collect();
        let edges_clone = self.edges.clone();
        let connected_opt: HashMap<Port, Port> = edges_clone
            .into_iter()
            .map(|edge| (edge.to, edge.from))
            .collect();

        // Traverse until leaf output nodes
        let mut updated_connections: HashSet<Port> = HashSet::new();
        while node_idxs.len() > 0 {
            let mut next_nodes: Vec<NodeKey> = Vec::new();
            for node_idx in node_idxs {
                let node_inputs = self.nodes[node_idx].borrow().inputs();
                let connected_inputs: Vec<_> = node_inputs
                    .iter()
                    .filter(|port_id| {
                        connected_opt.contains_key(&Port {
                            node: node_idx,
                            port: port_id.port,
                            kind: PortKind::Input,
                        })
                    })
                    .collect();
                if connected_inputs
                    .iter()
                    .map(|port_id| {
                        let input = Port {
                            node: node_idx,
                            port: port_id.port,
                            kind: PortKind::Input,
                        };
                        updated_connections.contains(&input)
                    })
                    .all(|x| x)
                {
                    // Get edges connecting outputs of node_idx to other nodes
                    let connections: Vec<_> = self
                        .edges
                        .iter()
                        // .clone()
                        // .into_iter()
                        .filter(move |edge| edge.from.node == node_idx)
                        .cloned()
                        .collect();

                    let connections_inputs: Vec<_> = self
                        .edges
                        .iter()
                        // .clone()
                        // .into_iter()
                        .filter(move |edge| edge.to.node == node_idx)
                        .cloned()
                        .collect();

                    if !new_node_order.contains(&node_idx) {
                        new_node_order.push(node_idx.clone());
                        new_output_edges.insert(node_idx.clone(), connections.clone());
                        new_input_edges.insert(node_idx.clone(), connections_inputs.clone());
                    }

                    // for ((_edge_out_idx, _port_out_idx), (edge_in_idx, port_in_idx)) in connections
                    for edge in connections {
                        // Add the downstream node to the traversal list
                        if !next_nodes.contains(&edge.to.node) {
                            next_nodes.push(edge.to.node);
                        }
                        updated_connections.insert(Port {
                            node: edge.to.node,
                            port: edge.to.port,
                            kind: PortKind::Input,
                        });
                    }
                }
            }
            node_idxs = next_nodes;
        }
        self.node_order = new_node_order;
        self.node_outputs = new_output_edges;
        self.node_inputs = new_input_edges;
    }

    pub fn node_order(&self) -> &Vec<NodeKey> {
        &self.node_order
    }

    pub fn node_outputs(&self) -> &HashMap<NodeKey, Vec<Edge>> {
        &self.node_outputs
    }
    pub fn node_inputs(&self) -> &HashMap<NodeKey, Vec<Edge>> {
        &self.node_inputs
    }

    pub fn step(&mut self, sample_rate: f32) -> f32 {
        let mut retn: f32 = 0.0;
        for node_idx in &self.node_order {
            self.nodes[*node_idx].borrow_mut().step(sample_rate);

            // for ((_edge_out_idx, port_out_idx), (edge_in_idx, port_in_idx)) in
            for edge in &self.node_outputs[node_idx] {
                // Set the input of edge_in to the output value of edge_out
                let out_val = self.nodes[*node_idx].borrow().get(edge.from.port);
                self.nodes[edge.to.node]
                    .borrow_mut()
                    .set(edge.to.port, out_val);
            }
        }

        if let Some(output_node_key) = self.output_node {
            retn = self.nodes[output_node_key].borrow().get(0);
        }

        retn
    }
}
