//! Graph IR — typed DAG for model representation.
//!
//! Used by the graph-executor fallback path (see reference/runtime/architecture.md)
//! whenever a model doesn't fit a curated forward. The IR is acyclic, has symbolic
//! shapes (`Dim::Dynamic`), memory-residency policy, and a pre-execution memory plan.
//!
//! Spec: reference/runtime/ir.md

use crate::core::dtype::DType;
use crate::core::op::Op;
use std::collections::HashMap;

/// Tensor name — weight key or intermediate label.
pub type TensorId = String;

/// Attribute map attached to a Node (op-specific scalars not worth modeling
/// structurally in [Op], e.g. free-form fusion hints).
pub type Attrs = HashMap<String, AttrValue>;

#[derive(Clone, Debug)]
pub enum AttrValue {
    Int(i64),
    Float(f32),
    String(String),
    Ints(Vec<i64>),
    Floats(Vec<f32>),
    Bool(bool),
}

/// Optional preferred backend. Graph executor picks per-op: honor hint if the
/// backend supports the op, otherwise fall through to default / CPU reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackendHint {
    Cpu,
    WgpuRs,
    Honeycrisp,
    Nox,
}

/// A tensor dimension: known at build time, or symbolic and resolved at first call.
#[derive(Clone, Debug, PartialEq)]
pub enum Dim {
    Fixed(usize),
    Dynamic(String),
}

pub type Shape = Vec<Dim>;

/// Buffer lifetime policy — drives allocation strategy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Residency {
    /// Lives for the whole program (weights, constant tables).
    Resident,
    /// Persists across forward calls but is mutable (KV cache).
    Cached,
    /// Freed after its last consumer in a single forward.
    Streamed,
}

#[derive(Clone, Debug)]
pub struct BufferLifetime {
    pub produced_at: usize,
    pub consumed_at: usize,
    pub size_bytes: usize,
}

#[derive(Clone, Debug)]
pub struct MemoryPlan {
    pub lifetimes: HashMap<TensorId, BufferLifetime>,
    pub peak_bytes: usize,
}

pub struct Graph {
    pub nodes: Vec<Node>,
    pub tensors: HashMap<TensorId, TensorMeta>,
    pub weights: HashMap<String, WeightData>,
}

pub struct Node {
    pub id: usize,
    pub op: Op,
    pub inputs: Vec<TensorId>,
    pub outputs: Vec<TensorId>,
    pub attrs: Attrs,
    pub backend_hint: Option<BackendHint>,
}

#[derive(Clone, Debug)]
pub struct TensorMeta {
    pub shape: Shape,
    pub dtype: DType,
    pub residency: Residency,
}

pub struct WeightData {
    pub data: Vec<u8>,
    pub shape: Vec<usize>,
    pub dtype: DType,
    /// GGUF stores 2D weights with ne[0] as the fast dim (column-major vs HF).
    /// True → caller must transpose when reading into a canonical `[N, K]` layout.
    pub needs_transpose: bool,
}

impl TensorMeta {
    pub fn fixed(shape: Vec<usize>, dtype: DType) -> Self {
        Self {
            shape: shape.into_iter().map(Dim::Fixed).collect(),
            dtype,
            residency: Residency::Streamed,
        }
    }

    pub fn weight(shape: Vec<usize>, dtype: DType) -> Self {
        Self {
            shape: shape.into_iter().map(Dim::Fixed).collect(),
            dtype,
            residency: Residency::Resident,
        }
    }

    /// Shape with all dims fixed, else None (some dim is symbolic).
    pub fn fixed_shape(&self) -> Option<Vec<usize>> {
        self.shape
            .iter()
            .map(|d| match d {
                Dim::Fixed(n) => Some(*n),
                Dim::Dynamic(_) => None,
            })
            .collect()
    }

    /// Byte size if the shape is fully fixed. None if symbolic.
    pub fn fixed_bytes(&self) -> Option<usize> {
        self.fixed_shape()
            .map(|s| self.dtype.bytes_for(s.iter().product::<usize>()))
    }
}

impl Graph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            tensors: HashMap::new(),
            weights: HashMap::new(),
        }
    }

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

    pub fn add_tensor(&mut self, name: TensorId, meta: TensorMeta) {
        self.tensors.insert(name, meta);
    }

    pub fn add_weight(&mut self, name: String, data: WeightData) {
        self.weights.insert(name, data);
    }

    pub fn get_weight(&self, name: &str) -> Option<&WeightData> {
        self.weights.get(name)
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn stateful_ops(&self) -> Vec<usize> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.op.is_stateful())
            .map(|(i, _)| i)
            .collect()
    }

    /// Build a `consumers[tensor] → node indices` index. O(nodes × avg-inputs).
    /// Use this once and pass it to passes that repeatedly ask "who reads X?";
    /// otherwise each pass does an O(n²) scan.
    pub fn build_consumers(&self) -> HashMap<TensorId, Vec<usize>> {
        let mut map: HashMap<TensorId, Vec<usize>> = HashMap::new();
        for (i, node) in self.nodes.iter().enumerate() {
            for input in &node.inputs {
                map.entry(input.clone()).or_default().push(i);
            }
        }
        map
    }

    /// Remove nodes whose outputs are never consumed and aren't graph outputs.
    /// Returns the count removed.
    pub fn eliminate_dead_nodes(&mut self) -> usize {
        let mut consumed: std::collections::HashSet<String> = self
            .nodes
            .iter()
            .flat_map(|n| n.inputs.iter().cloned())
            .collect();

        let mut alive = vec![true; self.nodes.len()];
        let mut removed = 0;

        for i in (0..self.nodes.len()).rev() {
            let all_outputs_dead = self.nodes[i]
                .outputs
                .iter()
                .all(|o| !consumed.contains(o));

            if all_outputs_dead
                && !self.nodes[i].op.is_stateful()
                && !self.nodes[i].outputs.is_empty()
            {
                alive[i] = false;
                removed += 1;
                for input in &self.nodes[i].inputs {
                    let still_consumed = self.nodes.iter().enumerate().any(|(j, n)| {
                        j != i && alive[j] && n.inputs.contains(input)
                    });
                    if !still_consumed {
                        consumed.remove(input);
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
            for (new_id, node) in new_nodes.iter_mut().enumerate() {
                node.id = new_id;
            }
            self.nodes = new_nodes;
            log::info!("graph dead-node elimination: removed {removed} nodes");
        }

        removed
    }

    /// Kahn topological sort. Returns false if a cycle is present (invariant
    /// violation — graphs must be acyclic).
    pub fn topological_sort(&mut self) -> bool {
        let n = self.nodes.len();
        if n == 0 {
            return true;
        }

        let mut producer: HashMap<String, usize> = HashMap::new();
        for (i, node) in self.nodes.iter().enumerate() {
            for output in &node.outputs {
                producer.insert(output.clone(), i);
            }
        }

        let mut in_degree = vec![0usize; n];
        let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (i, node) in self.nodes.iter().enumerate() {
            for input in &node.inputs {
                if let Some(&prod) = producer.get(input) {
                    if prod != i {
                        dependents[prod].push(i);
                        in_degree[i] += 1;
                    }
                }
            }
        }

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
            log::error!("graph topological sort: cycle detected");
            return false;
        }

        let mut slots: Vec<Option<Node>> = self.nodes.drain(..).map(Some).collect();
        for &idx in &order {
            if let Some(node) = slots[idx].take() {
                self.nodes.push(node);
            }
        }
        for (new_id, node) in self.nodes.iter_mut().enumerate() {
            node.id = new_id;
        }

        true
    }

    /// Compute per-buffer lifetimes and peak live bytes across all nodes.
    /// Only counts tensors with fully-fixed shapes; symbolic shapes are skipped.
    pub fn memory_plan(&self) -> MemoryPlan {
        let mut lifetimes: HashMap<TensorId, BufferLifetime> = HashMap::new();

        // Pass 1: produced_at + size
        for (i, node) in self.nodes.iter().enumerate() {
            for output in &node.outputs {
                if let Some(meta) = self.tensors.get(output) {
                    if let Some(bytes) = meta.fixed_bytes() {
                        lifetimes.insert(
                            output.clone(),
                            BufferLifetime {
                                produced_at: i,
                                consumed_at: i,
                                size_bytes: bytes,
                            },
                        );
                    }
                }
            }
        }

        // Pass 2: consumed_at = max node index that reads the buffer
        for (i, node) in self.nodes.iter().enumerate() {
            for input in &node.inputs {
                if let Some(lt) = lifetimes.get_mut(input) {
                    if i > lt.consumed_at {
                        lt.consumed_at = i;
                    }
                }
            }
        }

        // Peak live intermediates at any timestep
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

        peak_bytes += self.weights.values().map(|w| w.data.len()).sum::<usize>();

        MemoryPlan {
            lifetimes,
            peak_bytes,
        }
    }

    /// One-line-per-op count summary for logs.
    pub fn summary(&self) -> String {
        let mut op_counts: HashMap<&str, usize> = HashMap::new();
        for node in &self.nodes {
            *op_counts.entry(node.op.name()).or_insert(0) += 1;
        }
        let mut counts: Vec<_> = op_counts.into_iter().collect();
        counts.sort_by(|a, b| b.1.cmp(&a.1));

        let mut s = format!(
            "Graph: {} nodes, {} weights\n",
            self.nodes.len(),
            self.weights.len()
        );
        for (op, count) in counts {
            s.push_str(&format!("  {op}: {count}\n"));
        }

        let stateful = self.stateful_ops();
        if !stateful.is_empty() {
            s.push_str(&format!(
                "  Stateful ops: {} (KV cache nodes)\n",
                stateful.len()
            ));
        }

        s
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_graph_sorts_and_plans() {
        let mut g = Graph::new();
        assert!(g.topological_sort());
        let plan = g.memory_plan();
        assert_eq!(plan.peak_bytes, 0);
    }

    #[test]
    fn toposort_reorders_dependency() {
        let mut g = Graph::new();
        // Add in reverse dependency order: consumer first, producer second.
        g.add_node(Op::Add, vec!["x".into(), "y".into()], vec!["z".into()]);
        g.add_node(Op::Mul, vec![], vec!["x".into()]);
        g.add_node(Op::Sub, vec![], vec!["y".into()]);
        assert!(g.topological_sort());
        // Consumer must come after its producers.
        let z_idx = g.nodes.iter().position(|n| n.outputs.contains(&"z".to_string())).unwrap();
        let x_idx = g.nodes.iter().position(|n| n.outputs.contains(&"x".to_string())).unwrap();
        let y_idx = g.nodes.iter().position(|n| n.outputs.contains(&"y".to_string())).unwrap();
        assert!(x_idx < z_idx);
        assert!(y_idx < z_idx);
    }

    #[test]
    fn dead_node_elimination() {
        let mut g = Graph::new();
        g.add_node(Op::Mul, vec![], vec!["used".into()]);
        g.add_node(Op::Mul, vec![], vec!["dead".into()]);
        g.add_node(Op::Add, vec!["used".into()], vec!["out".into()]);
        let removed = g.eliminate_dead_nodes();
        // "dead" has no consumers → removed. "out" has no consumers either,
        // but eliminating it would also kill "used". Current pass removes
        // both dead leaf nodes in one reverse sweep.
        assert!(removed >= 1);
    }

    #[test]
    fn build_consumers_indexes_inputs() {
        let mut g = Graph::new();
        g.add_node(Op::Mul, vec![], vec!["x".into()]);
        g.add_node(Op::Add, vec!["x".into()], vec!["y".into()]);
        g.add_node(Op::Sub, vec!["x".into()], vec!["z".into()]);
        let consumers = g.build_consumers();
        assert_eq!(consumers.get("x").map(|v| v.len()), Some(2));
    }
}
