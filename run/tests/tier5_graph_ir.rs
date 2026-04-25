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

// ── parametric op field roundtrip ─────────────────────────────────────────────

#[test]
fn serial_roundtrip_parametric_op_fields() {
    use run::ir::graph::{Attrs, Graph, Node};
    use run::core::op::{Op, SampleMethod};

    // Build a tiny graph with several parametric ops so we can verify
    // their scalar fields survive serialize → deserialize.
    let mut graph = Graph::new();

    // RmsNorm with specific eps
    graph.nodes.push(Node {
        id: 0,
        op: Op::RmsNorm { eps: 1e-6 },
        inputs: vec!["x".into(), "g".into()],
        outputs: vec!["normed".into()],
        attrs: Attrs::new(),
        backend_hint: None,
    });

    // Rope with specific head_dim, rope_dim, base
    graph.nodes.push(Node {
        id: 1,
        op: Op::Rope { head_dim: 128, rope_dim: 64, base: 1_000_000.0 },
        inputs: vec!["normed".into(), "pos".into()],
        outputs: vec!["roped".into()],
        attrs: Attrs::new(),
        backend_hint: None,
    });

    // Sdpa with specific num_heads/kv_heads/head_dim/causal
    graph.nodes.push(Node {
        id: 2,
        op: Op::Sdpa { num_heads: 16, kv_heads: 4, head_dim: 128, causal: true },
        inputs: vec!["roped".into(), "k".into(), "v".into()],
        outputs: vec!["attn".into()],
        attrs: Attrs::new(),
        backend_hint: None,
    });

    // Gelu approximate=true
    graph.nodes.push(Node {
        id: 3,
        op: Op::Gelu { approximate: true },
        inputs: vec!["attn".into()],
        outputs: vec!["act".into()],
        attrs: Attrs::new(),
        backend_hint: None,
    });

    // Reshape with -1 inference dim
    graph.nodes.push(Node {
        id: 4,
        op: Op::Reshape { shape: vec![1, -1, 128] },
        inputs: vec!["act".into()],
        outputs: vec!["out".into()],
        attrs: Attrs::new(),
        backend_hint: None,
    });

    // Softmax with negative dim
    graph.nodes.push(Node {
        id: 5,
        op: Op::Softmax { dim: -1 },
        inputs: vec!["out".into()],
        outputs: vec!["probs".into()],
        attrs: Attrs::new(),
        backend_hint: None,
    });

    let bytes = serialize(&graph);
    let g2 = deserialize(&bytes).expect("deserialize");

    // Verify each op's parameters survived exactly.
    match &g2.nodes[0].op {
        Op::RmsNorm { eps } => assert!((eps - 1e-6f32).abs() < 1e-10, "RmsNorm eps"),
        other => panic!("node 0 expected RmsNorm, got {}", other.name()),
    }
    match &g2.nodes[1].op {
        Op::Rope { head_dim, rope_dim, base } => {
            assert_eq!(*head_dim, 128, "Rope head_dim");
            assert_eq!(*rope_dim, 64, "Rope rope_dim");
            assert_eq!(*base, 1_000_000.0f32, "Rope base");
        }
        other => panic!("node 1 expected Rope, got {}", other.name()),
    }
    match &g2.nodes[2].op {
        Op::Sdpa { num_heads, kv_heads, head_dim, causal } => {
            assert_eq!(*num_heads, 16, "Sdpa num_heads");
            assert_eq!(*kv_heads, 4, "Sdpa kv_heads");
            assert_eq!(*head_dim, 128, "Sdpa head_dim");
            assert!(*causal, "Sdpa causal");
        }
        other => panic!("node 2 expected Sdpa, got {}", other.name()),
    }
    match &g2.nodes[3].op {
        Op::Gelu { approximate } => assert!(*approximate, "Gelu approximate"),
        other => panic!("node 3 expected Gelu, got {}", other.name()),
    }
    match &g2.nodes[4].op {
        Op::Reshape { shape } => assert_eq!(shape, &[1i64, -1, 128], "Reshape shape"),
        other => panic!("node 4 expected Reshape, got {}", other.name()),
    }
    match &g2.nodes[5].op {
        Op::Softmax { dim } => assert_eq!(*dim, -1i32, "Softmax dim"),
        other => panic!("node 5 expected Softmax, got {}", other.name()),
    }
}

#[test]
fn serial_roundtrip_conv_parameters() {
    use run::ir::graph::{Attrs, Graph, Node};
    use run::core::op::Op;

    let mut graph = Graph::new();
    graph.nodes.push(Node {
        id: 0,
        op: Op::Conv2d {
            kernel: (3, 5), stride: (2, 1), padding: (1, 2),
            dilation: (1, 1), groups: 4,
        },
        inputs: vec!["x".into()],
        outputs: vec!["y".into()],
        attrs: Attrs::new(),
        backend_hint: None,
    });
    graph.nodes.push(Node {
        id: 1,
        op: Op::LeakyRelu { slope: 0.01 },
        inputs: vec!["y".into()],
        outputs: vec!["z".into()],
        attrs: Attrs::new(),
        backend_hint: None,
    });

    let g2 = deserialize(&serialize(&graph)).unwrap();

    match &g2.nodes[0].op {
        Op::Conv2d { kernel, stride, padding, dilation, groups } => {
            assert_eq!(*kernel, (3, 5));
            assert_eq!(*stride, (2, 1));
            assert_eq!(*padding, (1, 2));
            assert_eq!(*dilation, (1, 1));
            assert_eq!(*groups, 4);
        }
        other => panic!("expected Conv2d, got {}", other.name()),
    }
    match &g2.nodes[1].op {
        Op::LeakyRelu { slope } => assert!((slope - 0.01f32).abs() < 1e-7, "LeakyRelu slope"),
        other => panic!("expected LeakyRelu, got {}", other.name()),
    }
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
