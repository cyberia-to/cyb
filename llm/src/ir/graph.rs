//! Core Graph IR types — typed DAG for model representation

use std::collections::HashMap;

use super::ops::Op;
use super::dtype::DType;

/// Tensor identifier (weight name or intermediate tensor name)
pub type TensorId = String;

/// Graph IR — directed acyclic graph of operations
pub struct Graph {
    pub nodes: Vec<Node>,
    pub tensors: HashMap<TensorId, TensorMeta>,
    pub weights: HashMap<String, WeightData>,
}

/// A single computation node
pub struct Node {
    pub id: usize,
    pub op: Op,
    pub inputs: Vec<TensorId>,
    pub outputs: Vec<TensorId>,
}

/// Tensor metadata (shape + type, no data)
#[derive(Clone, Debug)]
pub struct TensorMeta {
    pub shape: Vec<usize>,
    pub dtype: DType,
}

/// Raw weight data with metadata
pub struct WeightData {
    pub data: Vec<u8>,
    pub shape: Vec<usize>,
    pub dtype: DType,
}

impl Graph {
    /// Create an empty graph
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            tensors: HashMap::new(),
            weights: HashMap::new(),
        }
    }

    /// Add a node to the graph, returns its ID
    pub fn add_node(&mut self, op: Op, inputs: Vec<TensorId>, outputs: Vec<TensorId>) -> usize {
        let id = self.nodes.len();
        self.nodes.push(Node { id, op, inputs, outputs });
        id
    }

    /// Register tensor metadata
    pub fn add_tensor(&mut self, name: TensorId, meta: TensorMeta) {
        self.tensors.insert(name, meta);
    }

    /// Add weight data
    pub fn add_weight(&mut self, name: String, data: WeightData) {
        self.weights.insert(name, data);
    }

    /// Get a weight by name
    pub fn get_weight(&self, name: &str) -> Option<&WeightData> {
        self.weights.get(name)
    }

    /// Number of nodes
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Check if graph is empty
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Log summary of the graph
    pub fn summary(&self) -> String {
        let mut op_counts: HashMap<&str, usize> = HashMap::new();
        for node in &self.nodes {
            *op_counts.entry(node.op.name()).or_insert(0) += 1;
        }
        let mut counts: Vec<_> = op_counts.into_iter().collect();
        counts.sort_by(|a, b| b.1.cmp(&a.1));

        let mut s = format!("Graph: {} nodes, {} weights\n", self.nodes.len(), self.weights.len());
        for (op, count) in counts {
            s.push_str(&format!("  {op}: {count}\n"));
        }
        s
    }
}
