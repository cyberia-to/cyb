//! Graph optimization passes — pattern detection and node fusion
//!
//! Real graph transforms that merge nodes into fused ops,
//! eliminate dead code, and plan memory.

use super::graph::Graph;
use super::ops::Op;

/// Fusion annotation on a node
#[derive(Clone, Debug)]
pub enum FusionHint {
    /// RmsNorm feeds exactly one Matmul — can use fused_norm_q4 kernel
    NormMatmul,
    /// Add + RmsNorm — can use fused_skip_norm kernel
    SkipNorm,
    /// Gate + Up + SiLU + Mul — SwiGLU pattern
    SwiGLU,
}

/// Run fusion analysis on a graph, returns list of (node_id, hint)
pub fn detect_fusions(graph: &Graph) -> Vec<(usize, FusionHint)> {
    let mut hints = Vec::new();

    for (i, node) in graph.nodes.iter().enumerate() {
        match &node.op {
            Op::RmsNorm { .. } => {
                // Check if the next node is a Matmul consuming our output
                if let Some(output) = node.outputs.first() {
                    let consumers: Vec<_> = graph.nodes.iter()
                        .filter(|n| n.inputs.contains(output))
                        .collect();
                    if consumers.len() == 1 {
                        if let Op::Matmul = &consumers[0].op {
                            hints.push((i, FusionHint::NormMatmul));
                        }
                    }
                }
            }
            Op::Add => {
                // Check if output feeds into RmsNorm
                if let Some(output) = node.outputs.first() {
                    let consumers: Vec<_> = graph.nodes.iter()
                        .filter(|n| n.inputs.contains(output))
                        .collect();
                    if consumers.len() == 1 {
                        if let Op::RmsNorm { .. } = &consumers[0].op {
                            hints.push((i, FusionHint::SkipNorm));
                        }
                    }
                }
            }
            Op::SwiGlu | Op::FusedSwiGlu => {
                hints.push((i, FusionHint::SwiGLU));
            }
            _ => {}
        }
    }

    if !hints.is_empty() {
        log::info!("Fusion analysis: {} patterns detected", hints.len());
        for (id, hint) in &hints {
            log::debug!("  Node {id}: {hint:?}");
        }
    }

    hints
}

/// Fuse RmsNorm + Matmul pairs into FusedNormMatmul nodes.
/// Returns number of fusions applied.
pub fn fuse_norm_matmul(graph: &mut Graph) -> usize {
    let mut fused = 0;
    let mut skip: std::collections::HashSet<usize> = std::collections::HashSet::new();

    let mut i = 0;
    while i < graph.nodes.len() {
        if skip.contains(&i) {
            i += 1;
            continue;
        }

        if let Op::RmsNorm { eps } = &graph.nodes[i].op {
            let eps = *eps;
            if let Some(output) = graph.nodes[i].outputs.first().cloned() {
                // Find the single consumer
                let consumer_idx = graph.nodes.iter().position(|n| {
                    n.id != graph.nodes[i].id && n.inputs.contains(&output)
                });
                if let Some(ci) = consumer_idx {
                    if let Op::Matmul = &graph.nodes[ci].op {
                        // Check single consumer
                        let num_consumers = graph.nodes.iter()
                            .filter(|n| n.inputs.contains(&output))
                            .count();
                        if num_consumers == 1 {
                            // Merge: take norm inputs + matmul's other inputs
                            let norm_inputs = graph.nodes[i].inputs.clone();
                            let matmul_inputs: Vec<_> = graph.nodes[ci].inputs.iter()
                                .filter(|inp| *inp != &output)
                                .cloned()
                                .collect();
                            let mut merged_inputs = norm_inputs;
                            merged_inputs.extend(matmul_inputs);

                            let merged_outputs = graph.nodes[ci].outputs.clone();

                            graph.nodes[i].op = Op::FusedNormMatmul { eps };
                            graph.nodes[i].inputs = merged_inputs;
                            graph.nodes[i].outputs = merged_outputs;

                            skip.insert(ci);
                            fused += 1;
                        }
                    }
                }
            }
        }
        i += 1;
    }

    // Remove skipped nodes
    if fused > 0 {
        graph.nodes.retain(|n| !skip.contains(&n.id));
        for (new_id, node) in graph.nodes.iter_mut().enumerate() {
            node.id = new_id;
        }
        log::info!("fuse_norm_matmul: {fused} fusions applied");
    }

    fused
}

/// Fuse Add + RmsNorm pairs into FusedSkipNorm nodes.
/// Returns number of fusions applied.
pub fn fuse_skip_norm(graph: &mut Graph) -> usize {
    let mut fused = 0;
    let mut skip: std::collections::HashSet<usize> = std::collections::HashSet::new();

    let mut i = 0;
    while i < graph.nodes.len() {
        if skip.contains(&i) {
            i += 1;
            continue;
        }

        if let Op::Add = &graph.nodes[i].op {
            if let Some(output) = graph.nodes[i].outputs.first().cloned() {
                let consumer_idx = graph.nodes.iter().position(|n| {
                    n.id != graph.nodes[i].id && n.inputs.contains(&output)
                });
                if let Some(ci) = consumer_idx {
                    if let Op::RmsNorm { eps } = &graph.nodes[ci].op {
                        let eps = *eps;
                        let num_consumers = graph.nodes.iter()
                            .filter(|n| n.inputs.contains(&output))
                            .count();
                        if num_consumers == 1 {
                            // Merge: add inputs + norm weight input
                            let add_inputs = graph.nodes[i].inputs.clone();
                            let norm_extra: Vec<_> = graph.nodes[ci].inputs.iter()
                                .filter(|inp| *inp != &output)
                                .cloned()
                                .collect();
                            let mut merged_inputs = add_inputs;
                            merged_inputs.extend(norm_extra);

                            let merged_outputs = graph.nodes[ci].outputs.clone();

                            graph.nodes[i].op = Op::FusedSkipNorm { eps };
                            graph.nodes[i].inputs = merged_inputs;
                            graph.nodes[i].outputs = merged_outputs;

                            skip.insert(ci);
                            fused += 1;
                        }
                    }
                }
            }
        }
        i += 1;
    }

    if fused > 0 {
        graph.nodes.retain(|n| !skip.contains(&n.id));
        for (new_id, node) in graph.nodes.iter_mut().enumerate() {
            node.id = new_id;
        }
        log::info!("fuse_skip_norm: {fused} fusions applied");
    }

    fused
}

/// Detect and fuse SwiGLU patterns: gate_proj + up_proj + silu + mul -> FusedSwiGlu.
/// Looks for Silu node whose output feeds into Mul alongside a parallel matmul.
/// Returns number of fusions applied.
pub fn fuse_swiglu(graph: &mut Graph) -> usize {
    let mut fused = 0;
    let mut skip: std::collections::HashSet<usize> = std::collections::HashSet::new();

    let mut i = 0;
    while i < graph.nodes.len() {
        if skip.contains(&i) {
            i += 1;
            continue;
        }

        if let Op::Silu = &graph.nodes[i].op {
            if let Some(silu_out) = graph.nodes[i].outputs.first().cloned() {
                // Find Mul node that consumes silu output
                let mul_idx = graph.nodes.iter().position(|n| {
                    n.id != graph.nodes[i].id
                        && matches!(&n.op, Op::Mul)
                        && n.inputs.contains(&silu_out)
                });
                if let Some(mi) = mul_idx {
                    let num_consumers = graph.nodes.iter()
                        .filter(|n| n.inputs.contains(&silu_out))
                        .count();
                    if num_consumers == 1 {
                        // Find silu's input (the gate matmul output)
                        let silu_inputs = graph.nodes[i].inputs.clone();
                        // Find mul's other input (the up proj output)
                        let mul_other: Vec<_> = graph.nodes[mi].inputs.iter()
                            .filter(|inp| *inp != &silu_out)
                            .cloned()
                            .collect();

                        let mut merged_inputs = silu_inputs;
                        merged_inputs.extend(mul_other);
                        let merged_outputs = graph.nodes[mi].outputs.clone();

                        graph.nodes[i].op = Op::FusedSwiGlu;
                        graph.nodes[i].inputs = merged_inputs;
                        graph.nodes[i].outputs = merged_outputs;

                        skip.insert(mi);
                        fused += 1;
                    }
                }
            }
        }
        i += 1;
    }

    if fused > 0 {
        graph.nodes.retain(|n| !skip.contains(&n.id));
        for (new_id, node) in graph.nodes.iter_mut().enumerate() {
            node.id = new_id;
        }
        log::info!("fuse_swiglu: {fused} fusions applied");
    }

    fused
}

/// Run all optimization passes in order:
/// 1. Topological sort
/// 2. Dead node elimination
/// 3. Fusion passes (norm+matmul, skip+norm, swiglu)
/// 4. Re-sort after fusion
///
/// Returns total number of optimizations applied.
pub fn optimize(graph: &mut Graph) -> usize {
    let mut total = 0;

    graph.topological_sort();

    total += graph.eliminate_dead_nodes();
    total += fuse_norm_matmul(graph);
    total += fuse_skip_norm(graph);
    total += fuse_swiglu(graph);

    graph.topological_sort();

    if total > 0 {
        log::info!("Graph optimization: {total} total transforms applied");
    }

    total
}
