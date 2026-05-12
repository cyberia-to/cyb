//! `mr graph <model>` — embed a binary IR graph section into a `.model` file.
//!
//! Loads the model, builds a Graph via `family_graph(config)`, serializes it
//! to binary, hex-encodes it, and rewrites the `.model` file with a
//! `~~~graph\n<hex>\n` section inserted after `~~~program\n`.
//!
//! Skips models whose architecture has no template yet (family_graph returns None).
//!
//! Usage:
//!   mr graph <model>            embed / refresh graph section
//!   mr graph <model> --verify   embed then verify by loading back

use crate::util::resolve_model_path;
use run::arch::decoder::config::LlamaConfig;
use run::format::LoadedModel;
use run::ir::{family_graph, hex_encode, serialize};

pub fn run(args: Vec<String>) {
    if args.is_empty() {
        eprintln!("usage: mr graph <model> [--verify]");
        std::process::exit(2);
    }
    let model_arg = &args[0];
    let verify = args.iter().any(|a| a == "--verify");

    let path = resolve_model_path(model_arg);
    if !path.exists() {
        eprintln!("model not found: {}", path.display());
        std::process::exit(1);
    }

    println!("Loading: {}", path.display());
    let lm = LoadedModel::load(&path).unwrap_or_else(|e| {
        eprintln!("load failed: {e}");
        std::process::exit(1);
    });

    // Parse config to get LlamaConfig (needed for family_graph).
    let llama_cfg = LlamaConfig::parse(&lm.file.config, &lm.tensors).unwrap_or_else(|e| {
        eprintln!("config parse failed: {e}");
        eprintln!("note: only LlamaStyle families are supported currently");
        std::process::exit(1);
    });

    let graph = match family_graph(&llama_cfg) {
        Some(g) => g,
        None => {
            eprintln!(
                "no template for model_type '{}' — skipping (MoE/DiT/Whisper/BERT not yet ported)",
                llama_cfg.model_type
            );
            std::process::exit(0);
        }
    };

    let node_count = graph.len();
    let bytes = serialize(&graph);
    let hex = hex_encode(&bytes);

    println!(
        "Graph: {} nodes, {} bytes binary, {} bytes hex",
        node_count,
        bytes.len(),
        hex.len()
    );

    // Rewrite the .model file: insert / replace the ~~~graph section.
    // The file has a binary weights section after ~~~weights\n that must not
    // be processed as UTF-8 text — invalid bytes would be silently replaced
    // with U+FFFD, corrupting the weight data. Split at the marker, operate
    // only on the text header, then concatenate with the unchanged binary.
    let file_bytes = std::fs::read(&path).unwrap_or_else(|e| {
        eprintln!("read failed: {e}");
        std::process::exit(1);
    });
    let weights_marker = b"~~~weights\n";
    let weights_data_start = file_bytes
        .windows(weights_marker.len())
        .position(|w| w == weights_marker)
        .map(|p| p + weights_marker.len())
        .unwrap_or(file_bytes.len());
    let text_header_bytes = &file_bytes[..weights_data_start];
    let binary_weights = &file_bytes[weights_data_start..];
    let text_header = std::str::from_utf8(text_header_bytes).unwrap_or_else(|e| {
        eprintln!("model text header is not valid UTF-8: {e}");
        std::process::exit(1);
    });

    let new_text = inject_graph_section(text_header, &hex);

    let mut new_content = new_text.into_bytes();
    new_content.extend_from_slice(binary_weights);

    std::fs::write(&path, &new_content).unwrap_or_else(|e| {
        eprintln!("write failed: {e}");
        std::process::exit(1);
    });

    println!("Written: {}", path.display());

    if verify {
        let lm2 = LoadedModel::load(&path).unwrap_or_else(|e| {
            eprintln!("verify load failed: {e}");
            std::process::exit(1);
        });
        match lm2.graph() {
            Some(g) => println!("Verified: graph loads back with {} nodes ✓", g.len()),
            None => {
                eprintln!("verify failed: graph section missing after write");
                std::process::exit(1);
            }
        }
    }
}

/// Insert or replace the `~~~graph` section in the .model text content.
///
/// The graph section is placed immediately after `~~~program` (or after
/// `~~~config` if no program section exists), and before `~~~tensors`.
/// This keeps it in the text header before `~~~weights`.
fn inject_graph_section(content: &str, hex: &str) -> String {
    let graph_section = format!("~~~graph\n{hex}\n");

    // If there's already a ~~~graph section, replace it.
    if let Some(start) = find_section_start(content, "~~~graph") {
        let end = find_next_section(content, start + 1).unwrap_or(content.len());
        let mut out = String::with_capacity(content.len() + graph_section.len());
        out.push_str(&content[..start]);
        out.push_str(&graph_section);
        out.push_str(&content[end..]);
        return out;
    }

    // No existing graph section — insert it.
    // Insert point: after ~~~program if present, else after ~~~config, else before ~~~tensors.
    let insert_after = find_section_start(content, "~~~program")
        .or_else(|| find_section_start(content, "~~~config"));

    let insert_pos = if let Some(section_start) = insert_after {
        // Insert after the end of that section's content.
        find_next_section(content, section_start + 1).unwrap_or_else(|| {
            // No next section — insert before ~~~weights.
            find_section_start(content, "~~~weights").unwrap_or(content.len())
        })
    } else {
        // No program or config — insert before ~~~tensors or ~~~weights.
        find_section_start(content, "~~~tensors")
            .or_else(|| find_section_start(content, "~~~weights"))
            .unwrap_or(content.len())
    };

    let mut out = String::with_capacity(content.len() + graph_section.len());
    out.push_str(&content[..insert_pos]);
    out.push_str(&graph_section);
    out.push_str(&content[insert_pos..]);
    out
}

/// Find the byte offset of the line starting with `marker` in `content`.
fn find_section_start(content: &str, marker: &str) -> Option<usize> {
    let mut pos = 0;
    for line in content.lines() {
        if line.trim_start().starts_with(marker) {
            return Some(pos);
        }
        pos += line.len() + 1; // +1 for \n
    }
    None
}

/// Find the byte offset of the next `~~~` section line after `start`.
fn find_next_section(content: &str, start: usize) -> Option<usize> {
    let rest = &content[start..];
    // Skip the first line (it's the current section header).
    let after_first_newline = rest.find('\n').map(|i| i + 1).unwrap_or(rest.len());
    let rest2 = &rest[after_first_newline..];
    let mut pos = start + after_first_newline;
    for line in rest2.lines() {
        if line.trim_start().starts_with("~~~") {
            return Some(pos);
        }
        pos += line.len() + 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inject_adds_graph_section() {
        let content = "frontmatter\n~~~config\nkey = \"val\"\n~~~tensors\n[\"a\"]\n~~~weights\n";
        let result = inject_graph_section(content, "deadbeef");
        assert!(result.contains("~~~graph\ndeadbeef\n"));
        // Graph section must appear before tensors and weights.
        let g_pos = result.find("~~~graph").unwrap();
        let t_pos = result.find("~~~tensors").unwrap();
        let w_pos = result.find("~~~weights").unwrap();
        assert!(g_pos < t_pos);
        assert!(g_pos < w_pos);
    }

    #[test]
    fn inject_replaces_existing_graph_section() {
        let content = "front\n~~~config\ncfg\n~~~graph\noldhex\n~~~tensors\nt\n~~~weights\n";
        let result = inject_graph_section(content, "newhex");
        assert!(result.contains("~~~graph\nnewhex\n"));
        assert!(!result.contains("oldhex"));
    }

    #[test]
    fn inject_inserts_after_program_section() {
        let content = "front\n~~~config\ncfg\n~~~program\ncode\n~~~tensors\nt\n~~~weights\n";
        let result = inject_graph_section(content, "graphhex");
        let prog_end = result.find("~~~tensors").unwrap();
        let g_pos = result.find("~~~graph").unwrap();
        // graph must come between program content and tensors
        assert!(g_pos < prog_end);
    }
}
