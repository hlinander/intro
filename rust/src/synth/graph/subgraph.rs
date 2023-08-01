use crate::graph::*;
use serde::{Deserialize, Serialize};

// use lolmacros::Wiretap;

#[derive(Serialize, Deserialize)]
pub struct Subgraph {
    pub subgraph: Graph,
    pub inputs: Vec<UnconnectedInput>,
    pub outputs: Vec<UnconnectedOutput>,
    // output ports
}

impl Default for Subgraph {
    fn default() -> Self {
        Subgraph::new()
    }
}

impl Subgraph {
    fn new() -> Self {
        let mut sg = Self {
            subgraph: Graph::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
        };
        // sg.inputs = sg.subgraph.get_unconnected_inputs();
        // sg.outputs = sg.subgraph.get_unconnected_outputs();
        sg
    }
    pub fn load(&mut self, filename: String) {
        let file_contents = std::fs::read_to_string(filename).unwrap();
        let graph: Graph = ron::from_str(&file_contents).unwrap();
        self.subgraph = graph;
        self.inputs = self.subgraph.get_unconnected_inputs();
        self.outputs = self.subgraph.get_unconnected_outputs();
    }
}

#[typetag::serde]
impl Node for Subgraph {
    fn copy(&self) -> Box<dyn Node> {
        let c = Subgraph {
            subgraph: self.subgraph.copy(),
            inputs: self.inputs.clone(),
            outputs: self.outputs.clone(),
        };
        Box::new(c)
    }
    fn inputs(&self) -> Vec<InputId> {
        self.inputs
            .iter()
            .map(|ui| InputId {
                port: ui.port_idx,
                name: &ui.name.as_str(),
            })
            .collect()
    }
    fn outputs(&self) -> Vec<OutputId> {
        self.outputs
            .iter()
            .map(|ui| OutputId {
                port: ui.port_idx,
                name: &ui.name.as_str(),
            })
            .collect()
    }

    // Set input at index idx to value val
    fn set(&mut self, idx: usize, val: f32) {
        if idx < self.inputs.len() {
            let sinput = &self.inputs[idx];
            self.subgraph
                .get_node_mut(sinput.node_key)
                .set(sinput.port_idx, val);
        } else {
            panic!();
        }
    }

    // Get value of output index idx
    fn get(&self, idx: usize) -> f32 {
        if idx < self.outputs.len() {
            let soutput = &self.outputs[idx];
            self.subgraph
                .get_node_mut(soutput.node_key)
                .get(soutput.port_idx)
        } else {
            panic!();
        }
    }

    fn get_input_mut(&mut self, idx: usize) -> &mut f32 {
        if idx < self.inputs.len() {
            let sinput = &self.inputs[idx];
            self.subgraph
                .get_node_mut(sinput.node_key)
                .get_input_mut(sinput.port_idx);
            // &mut self.value
            panic!()
        } else {
            panic!()
        }
    }

    fn get_input(&mut self, idx: usize) -> f32 {
        if idx < self.inputs.len() {
            let sinput = &self.inputs[idx];
            self.subgraph
                .get_node_mut(sinput.node_key)
                .get_input(sinput.port_idx)
        } else {
            panic!()
        }
    }

    fn step(&mut self, sample_rate: f32) {
        self.subgraph.step(sample_rate);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn name() -> &'static str {
        "Subgraph"
    }
}
