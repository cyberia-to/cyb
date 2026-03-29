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

/// Constant folding — identify nodes where ALL inputs are resident weights
/// (constants), execute them on CPU using the atom interpreter, and replace
/// with a constant tensor. Removes unnecessary compute from the graph.
///
/// Returns number of nodes folded.
pub fn constant_fold(graph: &mut Graph) -> usize {
    use super::atoms;
    use super::dtype::DType;

    let mut folded = 0;
    let mut folded_tensors: std::collections::HashMap<String, Vec<u8>> = std::collections::HashMap::new();

    for node in &graph.nodes {
        // Skip stateful and layout-only ops
        if node.op.is_stateful() || node.op.is_layout_only() {
            continue;
        }

        // Check if ALL inputs are either graph weights or previously folded constants
        let all_inputs_constant = node.inputs.iter().all(|inp| {
            graph.weights.contains_key(inp) || folded_tensors.contains_key(inp)
        });

        if !all_inputs_constant || node.inputs.is_empty() {
            continue;
        }

        // Decompose the op into atoms
        let atom_seq = atoms::decompose(&node.op);
        if atom_seq.is_empty() {
            continue;
        }

        // Collect input data as f32 slices
        let input_data: Vec<Vec<f32>> = node.inputs.iter().map(|inp| {
            if let Some(w) = graph.weights.get(inp) {
                match w.dtype {
                    DType::F32 => {
                        w.data.chunks_exact(4)
                            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                            .collect()
                    }
                    _ => vec![] // Only fold f32 constants for now
                }
            } else if let Some(data) = folded_tensors.get(inp) {
                data.chunks_exact(4)
                    .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                    .collect()
            } else {
                vec![]
            }
        }).collect();

        // Skip if any input couldn't be read as f32
        if input_data.iter().any(|d| d.is_empty()) {
            continue;
        }

        // Determine output size from the first output's metadata
        let output_size = if let Some(out_name) = node.outputs.first() {
            if let Some(meta) = graph.tensors.get(out_name) {
                if let Some(shape) = meta.fixed_shape() {
                    shape.iter().product::<usize>()
                } else {
                    continue; // dynamic shape, can't fold
                }
            } else {
                // No metadata, use first input size as estimate
                input_data[0].len()
            }
        } else {
            continue;
        };

        if output_size == 0 {
            continue;
        }

        // Execute on CPU using atom interpreter
        let input_refs: Vec<&[f32]> = input_data.iter().map(|d| d.as_slice()).collect();
        let mut output = vec![0.0f32; output_size];
        atoms::AtomInterpreter::execute(&atom_seq, &input_refs, &mut output);

        // Store folded result
        let output_bytes: Vec<u8> = output.iter().flat_map(|v| v.to_le_bytes()).collect();
        for out_name in &node.outputs {
            folded_tensors.insert(out_name.clone(), output_bytes.clone());
        }
        folded += 1;
    }

    // Replace folded nodes: add as weights and remove from graph
    if folded > 0 {
        let folded_node_outputs: std::collections::HashSet<String> = folded_tensors.keys().cloned().collect();

        for (name, data) in folded_tensors {
            let size = data.len() / 4;
            graph.weights.insert(name.clone(), super::graph::WeightData {
                data,
                shape: vec![size],
                dtype: DType::F32,
            });
        }

        // Remove nodes whose outputs were all folded
        let before = graph.nodes.len();
        graph.nodes.retain(|n| {
            !n.outputs.iter().all(|o| folded_node_outputs.contains(o))
        });
        let removed = before - graph.nodes.len();

        // Re-index
        for (new_id, node) in graph.nodes.iter_mut().enumerate() {
            node.id = new_id;
        }

        log::info!("constant_fold: folded {folded} nodes, removed {removed} from graph");
    }

    folded
}

/// Run all optimization passes in order:
/// 1. Topological sort
/// 2. Dead node elimination
/// 3. Constant folding
/// 4. Fusion passes (norm+matmul, skip+norm, swiglu)
/// 5. Re-sort after fusion
///
/// Returns total number of optimizations applied.
pub fn optimize(graph: &mut Graph) -> usize {
    let mut total = 0;

    graph.topological_sort();

    total += graph.eliminate_dead_nodes();
    total += constant_fold(graph);
    total += fuse_norm_matmul(graph);
    total += fuse_skip_norm(graph);
    total += fuse_swiglu(graph);

    graph.topological_sort();

    if total > 0 {
        log::info!("Graph optimization: {total} total transforms applied");
    }

    total
}
