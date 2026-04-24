//! Graph optimization passes — constant folding + structural fusion.
//!
//! Passes run at model-load time. Each is optional: the unfused graph
//! must produce the same output within accumulation-precision ε.
//!
//! The three structural fusers (NormMatmul, SkipNorm, SwiGLU) use a
//! pre-built consumers map to avoid the O(n²) consumer scan that the
//! old llm/ port had.
//!
//! Spec: reference/runtime/ir.md, reference/runtime/execution.md

use super::atoms::{self, AtomInterpreter};
use super::graph::{Graph, WeightData};
use crate::core::dtype::DType;
use crate::core::op::Op;
use std::collections::{HashMap, HashSet};

/// Structural fusion pattern hint — reported by [`detect_fusions`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FusionHint {
    NormMatmul,
    SkipNorm,
    SwiGLU,
}

/// List recognized fusion opportunities without mutating the graph.
/// Useful for diagnostics; the fuser passes rediscover patterns themselves.
pub fn detect_fusions(graph: &Graph) -> Vec<(usize, FusionHint)> {
    let consumers = graph.build_consumers();
    let mut out = Vec::new();
    for (i, node) in graph.nodes.iter().enumerate() {
        let single_consumer = |out_name: &str| -> Option<usize> {
            match consumers.get(out_name) {
                Some(v) if v.len() == 1 => Some(v[0]),
                _ => None,
            }
        };
        match &node.op {
            Op::RmsNorm { .. } => {
                if let Some(o) = node.outputs.first() {
                    if let Some(c) = single_consumer(o) {
                        if matches!(graph.nodes[c].op, Op::Matmul) {
                            out.push((i, FusionHint::NormMatmul));
                        }
                    }
                }
            }
            Op::Add => {
                if let Some(o) = node.outputs.first() {
                    if let Some(c) = single_consumer(o) {
                        if matches!(graph.nodes[c].op, Op::RmsNorm { .. }) {
                            out.push((i, FusionHint::SkipNorm));
                        }
                    }
                }
            }
            Op::SwiGlu | Op::FusedSwiGlu => out.push((i, FusionHint::SwiGLU)),
            _ => {}
        }
    }
    if !out.is_empty() {
        log::info!("fusion: {} patterns detected", out.len());
    }
    out
}

/// Merge `RmsNorm → Matmul` pairs into `FusedNormMatmul`.
/// Requires single-consumer on the norm output.
pub fn fuse_norm_matmul(graph: &mut Graph) -> usize {
    fuse_pair(graph, |this, next| match (&this.op, &next.op) {
        (Op::RmsNorm { eps }, Op::Matmul) => Some(Op::FusedNormMatmul { eps: *eps }),
        _ => None,
    })
}

/// Merge `Add → RmsNorm` pairs into `FusedSkipNorm`.
pub fn fuse_skip_norm(graph: &mut Graph) -> usize {
    fuse_pair(graph, |this, next| match (&this.op, &next.op) {
        (Op::Add, Op::RmsNorm { eps }) => Some(Op::FusedSkipNorm { eps: *eps }),
        _ => None,
    })
}

/// Merge `Silu → Mul` (SwiGLU tail) into `FusedSwiGlu`.
pub fn fuse_swiglu(graph: &mut Graph) -> usize {
    fuse_pair(graph, |this, next| match (&this.op, &next.op) {
        (Op::Silu, Op::Mul) => Some(Op::FusedSwiGlu),
        _ => None,
    })
}

/// Shared pair-fuser. For each producer node whose single consumer matches
/// `predicate`, replace the producer with `new_op`, absorb the consumer's
/// extra inputs/outputs, and drop the consumer.
///
/// Caller-supplied predicate returns `Some(new_op)` to fuse, `None` to skip.
fn fuse_pair(
    graph: &mut Graph,
    predicate: impl Fn(&super::graph::Node, &super::graph::Node) -> Option<Op>,
) -> usize {
    let mut fused = 0;
    let mut drop_ids: HashSet<usize> = HashSet::new();
    let consumers = graph.build_consumers();

    for i in 0..graph.nodes.len() {
        if drop_ids.contains(&graph.nodes[i].id) {
            continue;
        }
        let Some(out) = graph.nodes[i].outputs.first().cloned() else {
            continue;
        };
        let Some(cons) = consumers.get(&out) else {
            continue;
        };
        if cons.len() != 1 {
            continue;
        }
        let ci = cons[0];
        if ci == i || drop_ids.contains(&graph.nodes[ci].id) {
            continue;
        }
        let Some(new_op) = predicate(&graph.nodes[i], &graph.nodes[ci]) else {
            continue;
        };

        // Fold the consumer into this node.
        let mut merged_inputs = graph.nodes[i].inputs.clone();
        merged_inputs.extend(
            graph.nodes[ci]
                .inputs
                .iter()
                .filter(|inp| *inp != &out)
                .cloned(),
        );
        let merged_outputs = graph.nodes[ci].outputs.clone();

        graph.nodes[i].op = new_op;
        graph.nodes[i].inputs = merged_inputs;
        graph.nodes[i].outputs = merged_outputs;

        drop_ids.insert(graph.nodes[ci].id);
        fused += 1;
    }

    if fused > 0 {
        graph.nodes.retain(|n| !drop_ids.contains(&n.id));
        for (new_id, node) in graph.nodes.iter_mut().enumerate() {
            node.id = new_id;
        }
    }
    fused
}

/// Fold nodes whose inputs are all constants (graph weights or previously
/// folded results). Executes them on CPU via [`AtomInterpreter`] and adds
/// the result as a new resident weight.
///
/// Only handles f32 constant inputs for now — mixed-dtype folding is a
/// future extension.
pub fn constant_fold(graph: &mut Graph) -> usize {
    let mut folded = 0;
    let mut folded_tensors: HashMap<String, Vec<u8>> = HashMap::new();

    for node in &graph.nodes {
        if node.op.is_stateful() || node.op.is_layout_only() || node.inputs.is_empty() {
            continue;
        }
        let all_const = node
            .inputs
            .iter()
            .all(|i| graph.weights.contains_key(i) || folded_tensors.contains_key(i));
        if !all_const {
            continue;
        }

        let atom_seq = atoms::decompose(&node.op);
        if atom_seq.is_empty() {
            continue;
        }

        let input_data: Vec<Vec<f32>> = node
            .inputs
            .iter()
            .map(|inp| {
                if let Some(w) = graph.weights.get(inp) {
                    if w.dtype == DType::F32 {
                        w.data
                            .chunks_exact(4)
                            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                            .collect()
                    } else {
                        Vec::new()
                    }
                } else if let Some(bytes) = folded_tensors.get(inp) {
                    bytes
                        .chunks_exact(4)
                        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                        .collect()
                } else {
                    Vec::new()
                }
            })
            .collect();
        if input_data.iter().any(|d| d.is_empty()) {
            continue;
        }

        let output_size = match node.outputs.first() {
            Some(name) => match graph.tensors.get(name).and_then(|m| m.fixed_shape()) {
                Some(s) => s.iter().product::<usize>(),
                None => continue,
            },
            None => continue,
        };
        if output_size == 0 {
            continue;
        }

        let refs: Vec<&[f32]> = input_data.iter().map(|d| d.as_slice()).collect();
        let mut out = vec![0f32; output_size];
        AtomInterpreter::execute(&atom_seq, &refs, &mut out);

        let bytes: Vec<u8> = out.iter().flat_map(|v| v.to_le_bytes()).collect();
        for o in &node.outputs {
            folded_tensors.insert(o.clone(), bytes.clone());
        }
        folded += 1;
    }

    if folded > 0 {
        let folded_keys: HashSet<String> = folded_tensors.keys().cloned().collect();
        for (name, data) in folded_tensors {
            let n = data.len() / 4;
            graph.weights.insert(
                name.clone(),
                WeightData {
                    data,
                    shape: vec![n],
                    dtype: DType::F32,
                    needs_transpose: false,
                },
            );
        }
        let before = graph.nodes.len();
        graph
            .nodes
            .retain(|n| !n.outputs.iter().all(|o| folded_keys.contains(o)));
        let removed = before - graph.nodes.len();
        for (new_id, node) in graph.nodes.iter_mut().enumerate() {
            node.id = new_id;
        }
        log::info!("constant_fold: folded {folded} nodes, removed {removed}");
    }
    folded
}

/// Run the full load-time optimization pipeline:
/// toposort → DCE → constant-fold → structural fusers → toposort.
pub fn optimize(graph: &mut Graph) -> usize {
    graph.topological_sort();
    let mut total = 0;
    total += graph.eliminate_dead_nodes();
    total += constant_fold(graph);
    total += fuse_norm_matmul(graph);
    total += fuse_skip_norm(graph);
    total += fuse_swiglu(graph);
    graph.topological_sort();
    if total > 0 {
        log::info!("graph optimize: {total} transforms applied");
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::graph::TensorMeta;

    #[test]
    fn detect_norm_matmul() {
        let mut g = Graph::new();
        g.add_tensor("x".into(), TensorMeta::fixed(vec![4], DType::F32));
        g.add_tensor("n".into(), TensorMeta::fixed(vec![4], DType::F32));
        g.add_tensor("y".into(), TensorMeta::fixed(vec![4], DType::F32));
        g.add_node(Op::RmsNorm { eps: 1e-5 }, vec!["x".into(), "w".into()], vec!["n".into()]);
        g.add_node(Op::Matmul, vec!["n".into(), "wm".into()], vec!["y".into()]);
        let hints = detect_fusions(&g);
        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0].1, FusionHint::NormMatmul);
    }

    #[test]
    fn fuse_norm_matmul_merges_nodes() {
        let mut g = Graph::new();
        g.add_node(Op::RmsNorm { eps: 1e-5 }, vec!["x".into(), "gn".into()], vec!["n".into()]);
        g.add_node(Op::Matmul, vec!["n".into(), "wm".into()], vec!["y".into()]);
        assert_eq!(fuse_norm_matmul(&mut g), 1);
        assert_eq!(g.nodes.len(), 1);
        assert!(matches!(g.nodes[0].op, Op::FusedNormMatmul { .. }));
    }

    #[test]
    fn no_fuse_when_norm_has_two_consumers() {
        let mut g = Graph::new();
        g.add_node(Op::RmsNorm { eps: 1e-5 }, vec!["x".into()], vec!["n".into()]);
        g.add_node(Op::Matmul, vec!["n".into()], vec!["y1".into()]);
        g.add_node(Op::Matmul, vec!["n".into()], vec!["y2".into()]);
        assert_eq!(fuse_norm_matmul(&mut g), 0);
    }

    #[test]
    fn fuse_silu_mul_into_swiglu() {
        let mut g = Graph::new();
        g.add_node(Op::Silu, vec!["gate".into()], vec!["sgate".into()]);
        g.add_node(Op::Mul, vec!["sgate".into(), "up".into()], vec!["y".into()]);
        assert_eq!(fuse_swiglu(&mut g), 1);
        assert_eq!(g.nodes.len(), 1);
        assert!(matches!(g.nodes[0].op, Op::FusedSwiGlu));
    }
}
