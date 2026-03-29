//! Graph optimization passes — pattern detection and annotation
//!
//! For now, detect fusable patterns and log them. The backend can
//! check annotations to decide whether to use fused kernels.

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
            Op::SiluMul => {
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
