//! Core Graph IR types — typed DAG for model representation

use std::collections::HashMap;

use super::ops::Op;
use super::dtype::DType;

/// Tensor identifier (weight name or intermediate tensor name)
pub type TensorId = String;

/// Attribute key-value storage for node parameters
pub type Attrs = HashMap<String, AttrValue>;

/// Attribute value — typed parameter for ops (num_heads, eps, kernel_size, etc.)
#[derive(Clone, Debug)]
pub enum AttrValue {
    Int(i64),
    Float(f32),
    String(String),
    Ints(Vec<i64>),
    Floats(Vec<f32>),
    Bool(bool),
}

/// Backend hint — prefer a specific backend for this node
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BackendHint {
    Metal,
    Ane,
    Cuda,
    Wgpu,
    Cpu,
}

/// Shape dimension — supports both fixed and dynamic (symbolic) dimensions
#[derive(Clone, Debug, PartialEq)]
pub enum Dim {
    /// Fixed size known at graph construction time
    Fixed(usize),
    /// Dynamic dimension resolved at inference time (e.g. "batch", "seq_len")
    Dynamic(String),
}

/// Tensor shape — list of dimensions, may contain dynamic dims
pub type Shape = Vec<Dim>;

/// Tensor residency — how the tensor lives in memory
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Residency {
    /// Always in memory (weights, small constants)
    Resident,
    /// Cached across inference calls (KV cache)
    Cached,
    /// Streamed — computed and consumed, buffer reusable
    Streamed,
}

/// Buffer lifetime for memory planning
#[derive(Clone, Debug)]
pub struct BufferLifetime {
    /// Node index where buffer is first produced
    pub produced_at: usize,
    /// Node index where buffer is last consumed
    pub consumed_at: usize,
    /// Size in bytes
    pub size_bytes: usize,
}

/// Memory plan — buffer assignments and peak memory
#[derive(Clone, Debug)]
pub struct MemoryPlan {
    /// Per-tensor buffer lifetime
    pub lifetimes: HashMap<TensorId, BufferLifetime>,
    /// Peak memory usage in bytes
    pub peak_bytes: usize,
}

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
    pub attrs: Attrs,
    pub backend_hint: Option<BackendHint>,
}

/// Tensor metadata (shape + type + residency, no data)
#[derive(Clone, Debug)]
pub struct TensorMeta {
    pub shape: Shape,
    pub dtype: DType,
    pub residency: Residency,
}

/// Raw weight data with metadata
pub struct WeightData {
    pub data: Vec<u8>,
    pub shape: Vec<usize>,
    pub dtype: DType,
}

impl TensorMeta {
    /// Create a TensorMeta with fixed shape and default Streamed residency
    pub fn fixed(shape: Vec<usize>, dtype: DType) -> Self {
        Self {
            shape: shape.into_iter().map(Dim::Fixed).collect(),
            dtype,
            residency: Residency::Streamed,
        }
    }

    /// Create a TensorMeta for a weight (Resident residency)
    pub fn weight(shape: Vec<usize>, dtype: DType) -> Self {
        Self {
            shape: shape.into_iter().map(Dim::Fixed).collect(),
            dtype,
            residency: Residency::Resident,
        }
    }

    /// Get fixed shape dimensions, returning None if any dim is dynamic
    pub fn fixed_shape(&self) -> Option<Vec<usize>> {
        self.shape
            .iter()
            .map(|d| match d {
                Dim::Fixed(n) => Some(*n),
                Dim::Dynamic(_) => None,
            })
            .collect()
    }
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
        self.nodes.push(Node {
            id,
            op,
            inputs,
            outputs,
            attrs: HashMap::new(),
            backend_hint: None,
        });
        id
    }

    /// Add a node with attributes and optional backend hint
    pub fn add_node_with_attrs(
        &mut self,
        op: Op,
        inputs: Vec<TensorId>,
        outputs: Vec<TensorId>,
        attrs: Attrs,
        backend_hint: Option<BackendHint>,
    ) -> usize {
        let id = self.nodes.len();
        self.nodes.push(Node {
            id,
            op,
            inputs,
            outputs,
            attrs,
            backend_hint,
        });
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

    /// Return node indices for all stateful ops (KV cache, etc.)
    pub fn stateful_ops(&self) -> Vec<usize> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.op.is_stateful())
            .map(|(i, _)| i)
            .collect()
    }

    /// Eliminate dead nodes — remove nodes whose outputs are never consumed
    /// by any other node and are not graph-level outputs.
    /// Returns number of removed nodes.
    pub fn eliminate_dead_nodes(&mut self) -> usize {
        // Collect all tensor ids that are consumed as inputs
        let mut consumed: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for node in &self.nodes {
            for input in &node.inputs {
                consumed.insert(input.as_str());
            }
        }

        // Find nodes whose outputs are all unconsumed
        let mut alive = vec![true; self.nodes.len()];
        let mut removed = 0;

        // Iterate in reverse to propagate dead-ness
        for i in (0..self.nodes.len()).rev() {
            let all_outputs_dead = self.nodes[i]
                .outputs
                .iter()
                .all(|o| !consumed.contains(o.as_str()));

            // Don't remove stateful ops or nodes with no outputs (side effects)
            if all_outputs_dead && !self.nodes[i].op.is_stateful() && !self.nodes[i].outputs.is_empty() {
                alive[i] = false;
                removed += 1;
                // Remove this node's inputs from consumed set (they may become dead too)
                for input in &self.nodes[i].inputs {
                    // Only remove if no other alive node consumes it
                    let still_consumed = self.nodes.iter().enumerate().any(|(j, n)| {
                        j != i && alive[j] && n.inputs.contains(input)
                    });
                    if !still_consumed {
                        consumed.remove(input.as_str());
                    }
                }
            }
        }

        if removed > 0 {
            let mut new_nodes: Vec<Node> = Vec::new();
            for (i, node) in self.nodes.drain(..).enumerate() {
                if alive[i] {
                    new_nodes.push(node);
                }
            }
            // Re-index
            for (new_id, node) in new_nodes.iter_mut().enumerate() {
                node.id = new_id;
            }
            self.nodes = new_nodes;
            log::info!("Dead node elimination: removed {removed} nodes");
        }

        removed
    }

    /// Topological sort — ensure execution order respects data dependencies.
    /// Returns true if sort succeeded, false if cycle detected.
    pub fn topological_sort(&mut self) -> bool {
        let n = self.nodes.len();
        if n == 0 {
            return true;
        }

        // Build a map from tensor id -> producing node index
        let mut producer: HashMap<&str, usize> = HashMap::new();
        for (i, node) in self.nodes.iter().enumerate() {
            for output in &node.outputs {
                producer.insert(output.as_str(), i);
            }
        }

        // Build adjacency: edges[i] = set of nodes that i depends on
        let mut in_degree = vec![0usize; n];
        let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); n];

        for (i, node) in self.nodes.iter().enumerate() {
            for input in &node.inputs {
                if let Some(&prod) = producer.get(input.as_str()) {
                    if prod != i {
                        dependents[prod].push(i);
                        in_degree[i] += 1;
                    }
                }
            }
        }

        // Kahn's algorithm
        let mut queue: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
        for i in 0..n {
            if in_degree[i] == 0 {
                queue.push_back(i);
            }
        }

        let mut order: Vec<usize> = Vec::with_capacity(n);
        while let Some(idx) = queue.pop_front() {
            order.push(idx);
            for &dep in &dependents[idx] {
                in_degree[dep] -= 1;
                if in_degree[dep] == 0 {
                    queue.push_back(dep);
                }
            }
        }

        if order.len() != n {
            log::error!("Topological sort failed: cycle detected");
            return false;
        }

        // Reorder nodes
        let mut new_nodes: Vec<Option<Node>> = self.nodes.drain(..).map(Some).collect();
        for &idx in &order {
            if let Some(node) = new_nodes[idx].take() {
                self.nodes.push(node);
            }
        }
        // Re-index
        for (new_id, node) in self.nodes.iter_mut().enumerate() {
            node.id = new_id;
        }

        true
    }

    /// Plan memory — assign buffer lifetimes and compute peak memory.
    /// Only works for tensors with fully fixed shapes.
    pub fn memory_plan(&self) -> MemoryPlan {
        let mut lifetimes: HashMap<TensorId, BufferLifetime> = HashMap::new();

        // Pass 1: find produced_at for each tensor
        for (i, node) in self.nodes.iter().enumerate() {
            for output in &node.outputs {
                if let Some(meta) = self.tensors.get(output) {
                    if let Some(fixed) = meta.fixed_shape() {
                        let num_elements: usize = fixed.iter().product();
                        let size_bytes = num_elements * meta.dtype.element_size();
                        lifetimes.insert(output.clone(), BufferLifetime {
                            produced_at: i,
                            consumed_at: i, // will be updated
                            size_bytes,
                        });
                    }
                }
            }
        }

        // Pass 2: find consumed_at for each tensor
        for (i, node) in self.nodes.iter().enumerate() {
            for input in &node.inputs {
                if let Some(lt) = lifetimes.get_mut(input) {
                    if i > lt.consumed_at {
                        lt.consumed_at = i;
                    }
                }
            }
        }

        // Compute peak memory: at each node, sum alive buffers
        let mut peak_bytes = 0usize;
        for i in 0..self.nodes.len() {
            let alive_bytes: usize = lifetimes
                .values()
                .filter(|lt| lt.produced_at <= i && lt.consumed_at >= i)
                .map(|lt| lt.size_bytes)
                .sum();
            if alive_bytes > peak_bytes {
                peak_bytes = alive_bytes;
            }
        }

        // Add resident weight memory
        let weight_bytes: usize = self.weights.values().map(|w| w.data.len()).sum();
        peak_bytes += weight_bytes;

        MemoryPlan {
            lifetimes,
            peak_bytes,
        }
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

        let stateful = self.stateful_ops();
        if !stateful.is_empty() {
            s.push_str(&format!("  Stateful ops: {} (KV cache nodes)\n", stateful.len()));
        }

        s
    }
}
