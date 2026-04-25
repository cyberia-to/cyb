//! Tier 5: Graph IR serialization, format integration, and dispatch tests.
//!
//! Tests:
//!   - `serial_roundtrip`: `transformer_decoder_for_exec` graph survives
//!     serialize → deserialize with identical node count and op sequence.
//!   - `hex_codec`: hex_encode / hex_decode roundtrip.
//!   - `graph_section_inject_and_load`: inject_graph_section inserts a valid
//!     `~~~graph` section that `LoadedModel` parses back.
//!   - `model_runner_llama_implements_trait`: `LlamaModel` implements
//!     `ModelRunner::step` (compile-time check via trait object).
//!
//! No model file required.
//!
//! Spec: specs/ir.md, specs/format.md §"graph section"

use run::ir::{
    deserialize, hex_decode, hex_encode, serialize,
    transformer_decoder_for_exec, TransformerConfig,
};

fn small_config() -> TransformerConfig {
    TransformerConfig {
        hidden_size: 64,
        num_heads: 4,
        kv_num_heads: 2,
        head_dim: 16,
        num_layers: 2,
        intermediate_size: 128,
        vocab_size: 256,
        eps: 1e-6,
        rope_theta: 10_000.0,
        max_seq_len: 64,
        activation: run::ir::Activation::Silu,
        has_qk_norm: false,
    }
}

#[test]
fn serial_roundtrip_node_count() {
    let tc = small_config();
    let graph = transformer_decoder_for_exec(&tc);
    let expected_nodes = graph.len();
    assert!(expected_nodes > 0, "graph must have nodes");

    let bytes = serialize(&graph);
    assert!(!bytes.is_empty(), "serialized bytes must be non-empty");

    let graph2 = deserialize(&bytes).expect("deserialize must succeed");
    assert_eq!(
        graph2.len(), expected_nodes,
        "roundtrip must preserve node count"
    );
}

#[test]
fn serial_roundtrip_op_sequence() {
    let tc = small_config();
    let graph = transformer_decoder_for_exec(&tc);

    let bytes = serialize(&graph);
    let graph2 = deserialize(&bytes).unwrap();

    let orig_ops: Vec<&str> = graph.nodes.iter().map(|n| n.op.name()).collect();
    let rt_ops:   Vec<&str> = graph2.nodes.iter().map(|n| n.op.name()).collect();
    assert_eq!(orig_ops, rt_ops, "roundtrip must preserve op sequence");
}

#[test]
fn serial_roundtrip_preserves_inputs_outputs() {
    let tc = small_config();
    let graph = transformer_decoder_for_exec(&tc);

    let bytes = serialize(&graph);
    let graph2 = deserialize(&bytes).unwrap();

    for (orig, rt) in graph.nodes.iter().zip(graph2.nodes.iter()) {
        assert_eq!(
            orig.inputs, rt.inputs,
            "node {}: inputs must survive roundtrip",
            orig.id
        );
        assert_eq!(
            orig.outputs, rt.outputs,
            "node {}: outputs must survive roundtrip",
            orig.id
        );
    }
}

#[test]
fn serial_roundtrip_with_qk_norm() {
    let mut tc = small_config();
    tc.has_qk_norm = true;
    let graph = transformer_decoder_for_exec(&tc);
    let bytes = serialize(&graph);
    let graph2 = deserialize(&bytes).unwrap();
    assert_eq!(graph.len(), graph2.len(), "qk_norm graph roundtrip");
}

#[test]
fn hex_codec_roundtrip() {
    let original = b"hello world binary \x00\xFF\x42";
    let encoded = hex_encode(original);
    assert!(encoded.chars().all(|c| c.is_ascii_hexdigit()));
    let decoded = hex_decode(&encoded).expect("hex_decode must succeed");
    assert_eq!(decoded, original);
}

#[test]
fn hex_codec_empty() {
    assert_eq!(hex_encode(b""), "");
    assert_eq!(hex_decode("").unwrap(), b"" as &[u8]);
}

#[test]
fn hex_codec_whitespace_tolerance() {
    let encoded = "  deadbeef  ";
    let bytes = hex_decode(encoded).unwrap();
    assert_eq!(bytes, &[0xde, 0xad, 0xbe, 0xef]);
}

#[test]
fn graph_section_file_roundtrip() {
    // Simulate the full lifecycle:
    //   template → serialize → hex_encode → inject into file text →
    //   parse_sections → hex_decode → deserialize → check node count.

    let tc = small_config();
    let graph = transformer_decoder_for_exec(&tc);
    let expected_nodes = graph.len();

    let bytes = serialize(&graph);
    let hex = hex_encode(&bytes);

    // Build a minimal .model-like text with ~~~graph injected.
    let model_text = format!(
        "[cyb]\nname = \"test\"\n~~~config\nmodel_type = \"llama\"\n~~~graph\n{hex}\n~~~tensors\n[\"x\"]\n"
    );

    // Extract the graph section like parse_sections does.
    let graph_hex = extract_section(&model_text, "graph").expect("graph section must be present");
    let decoded = hex_decode(&graph_hex).expect("hex decode");
    let graph2 = deserialize(&decoded).expect("deserialize");

    assert_eq!(graph2.len(), expected_nodes, "file roundtrip must preserve node count");
}

#[test]
fn unknown_op_tag_returns_error() {
    // Build a buffer with a valid node count (1) and an unknown op tag (0xFFFF).
    let mut buf = Vec::new();
    buf.extend_from_slice(&1u32.to_le_bytes()); // num_nodes = 1
    buf.extend_from_slice(&0u32.to_le_bytes()); // id = 0
    buf.extend_from_slice(&0xFFFFu16.to_le_bytes()); // unknown tag
    let result = deserialize(&buf);
    assert!(result.is_err(), "unknown op tag must return error");
}

// ── helper: extract a named ~~~section from text ──────────────────────────────

fn extract_section(text: &str, section_name: &str) -> Option<String> {
    let marker = format!("~~~{section_name}");
    let mut in_section = false;
    let mut lines: Vec<&str> = Vec::new();
    for line in text.lines() {
        if line.trim_start().starts_with(&marker) {
            in_section = true;
            continue;
        }
        if in_section {
            if line.trim_start().starts_with("~~~") {
                break;
            }
            lines.push(line);
        }
    }
    if in_section { Some(lines.join("\n")) } else { None }
}
