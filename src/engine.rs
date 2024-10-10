use crate::module::{Module, ModuleId};
use std::collections::HashMap;
use petgraph::graph::{DiGraph, NodeIndex, EdgeIndex};
use petgraph::algo::toposort;
use crossbeam::queue::ArrayQueue;
use std::sync::Arc;
use parking_lot::Mutex;

pub struct AudioEngine {
    sample_rate: f64,
    buffer_size: usize,
    modules: HashMap<ModuleId, Arc<Mutex<dyn Module>>>,
    module_graph: DiGraph<ModuleId, (usize, usize)>, // (source_output, dest_input)
    processing_order: Vec<NodeIndex>,
    audio_input_queue: Arc<ArrayQueue<Vec<f32>>>,
    audio_output_queue: Arc<ArrayQueue<Vec<f32>>>,
}

impl AudioEngine {
    pub fn new(sample_rate: f64, buffer_size: usize) -> Self {
        AudioEngine {
            sample_rate,
            buffer_size,
            modules: HashMap::new(),
            module_graph: DiGraph::new(),
            processing_order: Vec::new(),
            audio_input_queue: Arc::new(ArrayQueue::new(32)),
            audio_output_queue: Arc::new(ArrayQueue::new(32)),
        }
    }

    pub fn add_module(&mut self, module: Arc<Mutex<dyn Module>>) -> ModuleId {
        let id = module.lock().id();
        let node_index = self.module_graph.add_node(id);
        self.modules.insert(id, module);
        self.update_processing_order();
        id
    }

    pub fn connect_modules(&mut self, source: ModuleId, source_output: usize, destination: ModuleId, dest_input: usize) {
        if let (Some(src_index), Some(dest_index)) = (
            self.module_graph.node_indices().find(|&n| self.module_graph[n] == source),
            self.module_graph.node_indices().find(|&n| self.module_graph[n] == destination),
        ) {
            self.module_graph.add_edge(src_index, dest_index, (source_output, dest_input));
            self.update_processing_order();
        }
    }

    pub fn get_module(&self, id: ModuleId) -> Option<&Arc<Mutex<dyn Module>>> {
        self.modules.get(&id)
    }

    fn update_processing_order(&mut self) {
        match toposort(&self.module_graph, None) {
            Ok(order) => {
                self.processing_order = order;
            }
            Err(_) => {
                // Handle cycle in graph
                log::error!("Cycle detected in module graph");
            }
        }
    }

    pub fn process(&mut self) -> Vec<f32> {
        let mut module_outputs: HashMap<ModuleId, Vec<f32>> = HashMap::new();

        for &node_index in &self.processing_order {
            let module_id = self.module_graph[node_index];
            let mut inputs = vec![vec![0.0; self.buffer_size]; self.modules[&module_id].lock().base.inputs.len()];

            // Collect inputs from connected modules
            for edge in self.module_graph.edges_directed(node_index, petgraph::Direction::Incoming) {
                let (source_output, dest_input) = *edge.weight();
                let source_id = self.module_graph[edge.source()];
                if let Some(source_output) = module_outputs.get(&source_id) {
                    inputs[dest_input] = source_output.clone();
                }
            }

            // Process the module
            let mut outputs = vec![0.0; self.buffer_size];
            if let Some(module) = self.modules.get(&module_id) {
                module.lock().process(&inputs.concat(), &mut outputs);
            }

            // Store the module's output
            module_outputs.insert(module_id, outputs);
        }

        // Return the output of the last module in the processing order
        if let Some(&last_module_id) = self.processing_order.last().map(|&n| &self.module_graph[n]) {
            module_outputs.get(&last_module_id).cloned().unwrap_or_else(|| vec![0.0; self.buffer_size])
        } else {
            vec![0.0; self.buffer_size]
        }
    }
}
