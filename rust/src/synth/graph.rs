use serde::{Deserialize, Serialize};
use slotmap::SlotMap;
use std::any::{Any, TypeId};
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
pub type Input = (usize, &'static str);
// Should be NodeOutput
pub type Output = (usize, &'static str);

// Should be GraphInputPort
pub type NodeInput = (usize, usize);
// Should be GraphOutputPort
pub type NodeOutput = (usize, usize);

#[typetag::serde(tag = "type")]
pub trait Node: Send + Any + 'static {
    fn type_name(&self) -> &'static str {
        core::any::type_name::<Self>()
    }
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn outputs(&self) -> Vec<Output>;
    fn inputs(&self) -> Vec<Input>;
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
            .filter(|&&(_id, n2)| n1 == n2)
            .map(|&(idx, _)| self.get(idx))
            .last()
            .unwrap()
    }

    // Set input for paramter n1 to value val
    fn set_by_name(&mut self, n1: &str, val: f32) {
        self.inputs()
            .iter()
            .filter(|&&(_id, n2)| n1 == n2)
            .for_each(|&(idx, _)| self.set(idx, val));
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

type Edge = (NodeOutput, NodeInput);
type NodeId = usize;

#[derive(Serialize, Deserialize)]
pub struct Graph {
    nodes: Vec<RefCell<Box<dyn Node>>>,
    edges: Vec<Edge>,

    node_order: Vec<NodeId>,
    node_outputs: HashMap<NodeId, Vec<Edge>>,
    node_inputs: HashMap<NodeId, Vec<Edge>>,

    pub volume: f32,
    pub steps: u64,

    #[serde(with = "serde_millis")]
    pub ctime: Instant,
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq, Clone)]
pub struct UnconnectedInput {
    pub node_idx: NodeId,
    pub port_idx: usize,
    pub name: String,
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq, Clone)]
pub struct UnconnectedOutput {
    pub node_idx: NodeId,
    pub port_idx: usize,
    pub name: String,
}

impl Graph {
    pub fn print(&self) {
        for (node_idx, node) in self.nodes.iter().enumerate() {
            println!("{}: {}", node_idx, node.borrow().type_name());
        }
    }
    pub fn add(&mut self, node: Box<dyn Node>) -> usize {
        println!("Adding {}", node.type_name());
        self.nodes.push(RefCell::new(node));
        self.nodes.len() - 1
    }

    pub fn get_node(&self, node_idx: NodeId) -> impl core::ops::Deref<Target = Box<dyn Node>> + '_ {
        self.nodes[node_idx].borrow()
    }

    pub fn get_node_mut(
        &self,
        node_idx: NodeId,
    ) -> impl core::ops::DerefMut<Target = Box<dyn Node>> + '_ {
        self.nodes[node_idx].borrow_mut()
    }

    pub fn has_node(&self, node_idx: NodeId) -> bool {
        node_idx < self.nodes.len()
    }

    /// Connects two nodes with an edge from `input` `(id, port)` to `output` `(id, port)`
    pub fn format_edge(&self, edge_idx: usize) -> String {
        let (s, e) = self.edges[edge_idx];
        self.format_edge_pair(s, e)
    }
    pub fn format_edge_pair(&self, s: NodeOutput, e: NodeInput) -> String {
        let output_node_type = self.nodes[s.0].borrow().type_name();
        let outputs = self.nodes[s.0].borrow().outputs();
        let output_port_name = outputs[s.1].1;
        let input_node_type = self.nodes[e.0].borrow().type_name();
        let inputs = self.nodes[e.0].borrow().inputs();
        let input_port_name = inputs[e.1].1;
        format!(
            "[{}] {} -> {} [{}]",
            output_node_type, output_port_name, input_port_name, input_node_type
        )
    }

    pub fn connect(&mut self, start: NodeOutput, end: NodeInput) {
        self.edges.push((start, end));
        println!("Connected {}", self.format_edge_pair(start, end));
        self.sort();
    }

    pub fn disconnect(&mut self, start: NodeOutput, end: NodeInput) {
        if let Some(idx) = self.edges.iter().position(|&(s, e)| s == start && e == end) {
            println!("Disconnecting {}", self.format_edge(idx));
            self.edges.remove(idx);
        } else {
            println!("Couldn't find the edge!");
        }
        self.sort();
    }

    pub fn disconnect_input_port(&mut self, input: NodeInput) {
        if let Some(idx) = self.edges.iter().position(|&(_i, o)| o == input) {
            println!("Disconnecting {}", self.format_edge(idx));
            self.edges.remove(idx);
        }
        self.sort();
    }

    pub fn remove(&mut self, node_idx: NodeId) {
        println!("Removing {}", self.nodes[node_idx].borrow().type_name());
        self.edges
            .retain(|&((i1, _), (i2, _))| !(i1 == node_idx || i2 == node_idx));
        self.sort();
        // NOTE: should we free box here?
    }

    pub fn clear(&mut self) {
        self.edges.clear();
        _ = self.nodes.split_off(1);
        self.sort();
    }

    pub fn new() -> Self {
        let g = Graph {
            nodes: vec![],
            edges: vec![],
            node_order: vec![],
            node_outputs: HashMap::new(),
            node_inputs: HashMap::new(),
            volume: 1.0,
            steps: 0,
            ctime: Instant::now(),
        };
        g
    }

    pub fn copy(&self) -> Self {
        Graph {
            nodes: self
                .nodes
                .iter()
                .map(|node| RefCell::new(node.borrow().copy()))
                .collect(),
            edges: self.edges.clone(),
            node_order: self.node_order.clone(),
            node_outputs: self.node_outputs.clone(),
            node_inputs: self.node_inputs.clone(),
            volume: self.volume,
            steps: self.steps,
            ctime: Instant::now(),
        }
    }

    pub fn get_by_type_mut<T: Node>(&mut self) -> Option<core::cell::RefMut<'_, T>> {
        for n in &mut self.nodes {
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

    pub fn new_unconnected_output(&self, output: NodeOutput) -> UnconnectedOutput {
        UnconnectedOutput {
            node_idx: output.0,
            port_idx: output.1,
            name: self.nodes[output.0].borrow().outputs()[output.1]
                .1
                .to_string(),
        }
    }

    pub fn new_unconnected_input(&self, input: NodeInput) -> UnconnectedInput {
        UnconnectedInput {
            node_idx: input.0,
            port_idx: input.1,
            name: self.nodes[input.0].borrow().inputs()[input.1].1.to_string(),
        }
    }

    pub fn get_unconnected_outputs_for_node(&self, node_idx: NodeId) -> Vec<UnconnectedOutput> {
        self.nodes[node_idx]
            .borrow()
            .outputs()
            .iter()
            .filter(|(port_idx, _name)| {
                self.edges
                    .iter()
                    .filter(
                        move |((from_idx, from_port_idx), (_to_idx, _to_port_idx))| {
                            (*from_idx == node_idx) & (from_port_idx == port_idx)
                        },
                    )
                    .count()
                    == 0
            })
            .map(|(port_idx, name)| UnconnectedOutput {
                node_idx,
                port_idx: *port_idx,
                name: name.to_string(),
            })
            .collect()
    }

    pub fn get_unconnected_inputs_for_node(&self, node_idx: NodeId) -> Vec<UnconnectedInput> {
        self.nodes[node_idx]
            .borrow()
            .inputs()
            .iter()
            .filter(|(port_idx, _name)| {
                self.edges
                    .iter()
                    .filter(
                        move |((_from_idx, _from_port_idx), (to_idx, _to_port_idx))| {
                            (*to_idx == node_idx) & (_to_port_idx == port_idx)
                        },
                    )
                    .count()
                    == 0
            })
            .map(|(port_idx, name)| UnconnectedInput {
                node_idx,
                port_idx: *port_idx,
                name: name.to_string(),
            })
            .collect()
    }

    pub fn get_unconnected_inputs(&self) -> Vec<UnconnectedInput> {
        let unconnected_inputs: Vec<_> = self
            .nodes
            .iter()
            .enumerate()
            .map(|(node_idx, _)| self.get_unconnected_inputs_for_node(node_idx))
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
            .iter()
            .enumerate()
            .filter(|(node_idx, _)| {
                self.edges
                    .iter()
                    .filter(|((_from_idx, _from_port_idx), (to_idx, _to_port_idx))| {
                        *to_idx == *node_idx
                    })
                    .count()
                    == 0
            })
            .map(|(node_idx, _)| node_idx)
            .collect();
        let edges_clone = self.edges.clone();
        let connected_opt: HashMap<_, _> = edges_clone
            .into_iter()
            .map(|(output, input)| (input, output))
            .collect();

        // Traverse until leaf output nodes
        let mut updated_connections: HashSet<NodeInput> = HashSet::new();
        while node_idxs.len() > 0 {
            let mut next_nodes: Vec<usize> = Vec::new();
            for node_idx in node_idxs {
                let node_inputs = self.nodes[node_idx].borrow().inputs();
                let connected_inputs: Vec<_> = node_inputs
                    .iter()
                    .filter(|(port_idx, _)| connected_opt.contains_key(&(node_idx, *port_idx)))
                    .collect();
                if connected_inputs
                    .iter()
                    .map(|(input_port_idx, _)| {
                        let input = (node_idx, *input_port_idx);
                        updated_connections.contains(&input)
                    })
                    .all(|x| x)
                {
                    // Get edges connecting outputs of node_idx to other nodes
                    let connections: Vec<_> = self
                        .edges
                        .clone()
                        .into_iter()
                        .filter(move |((edge_out_idx, _), _)| *edge_out_idx == node_idx)
                        .collect();

                    let connections_inputs: Vec<_> = self
                        .edges
                        .clone()
                        .into_iter()
                        .filter(move |((_, _), (edge_in_idx, _))| *edge_in_idx == node_idx)
                        .collect();

                    if !new_node_order.contains(&node_idx) {
                        new_node_order.push(node_idx.clone());
                        new_output_edges.insert(node_idx.clone(), connections.clone());
                        new_input_edges.insert(node_idx.clone(), connections_inputs.clone());
                    }

                    for ((_edge_out_idx, _port_out_idx), (edge_in_idx, port_in_idx)) in connections
                    {
                        // Add the downstream node to the traversal list
                        if !next_nodes.contains(&edge_in_idx) {
                            next_nodes.push(edge_in_idx);
                        }
                        updated_connections.insert((edge_in_idx, port_in_idx));
                    }
                }
            }
            node_idxs = next_nodes;
        }
        self.node_order = new_node_order;
        self.node_outputs = new_output_edges;
        self.node_inputs = new_input_edges;
    }

    pub fn node_order(&self) -> &Vec<usize> {
        &self.node_order
    }

    pub fn node_outputs(&self) -> &HashMap<usize, Vec<((usize, usize), (usize, usize))>> {
        &self.node_outputs
    }
    pub fn node_inputs(&self) -> &HashMap<usize, Vec<((usize, usize), (usize, usize))>> {
        &self.node_inputs
    }

    pub fn step(&mut self, sample_rate: f32) -> f32 {
        let mut retn: f32 = 0.0;
        for node_idx in &self.node_order {
            self.nodes[*node_idx].borrow_mut().step(sample_rate);

            for ((_edge_out_idx, port_out_idx), (edge_in_idx, port_in_idx)) in
                &self.node_outputs[node_idx]
            {
                // Set the input of edge_in to the output value of edge_out
                let out_val = self.nodes[*node_idx].borrow().get(*port_out_idx);
                self.nodes[*edge_in_idx]
                    .borrow_mut()
                    .set(*port_in_idx, out_val);
            }
        }

        if self.nodes.len() > 0 {
            retn = self.nodes[0].borrow().get(0);
        }

        retn
    }
}
