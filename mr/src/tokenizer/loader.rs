//! Build a Tokenizer from .model vocab + config sections.

use super::{Bpe, Tokenizer};
use crate::format::{FormatError, LoadedModel};

pub fn build_tokenizer(lm: &LoadedModel) -> Result<Tokenizer, FormatError> {
    let (mut tokens, merges) = parse_vocab(&lm.file.vocab_toml)?;

    let cfg: toml::Value = toml::from_str(&lm.file.config)
        .map_err(|e| FormatError::Invalid(format!("config parse: {e}")))?;

    // eos_token_ids from config [tokenizer].eos_token_ids
    let eos_token_ids: Vec<u32> = cfg
        .get("tokenizer")
        .and_then(|t| t.get("eos_token_ids"))
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_integer().map(|i| i as u32))
                .collect()
        })
        .unwrap_or_default();

    // Some imports (older cyb .model files) drop HF `added_tokens` — the
    // specials like <|im_start|> / <|im_end|> / <|endoftext|> aren't in the
    // [tokens] table even though the model's lm_head/embed rows for those
    // IDs exist. Without them chatml-templated prompts decompose into
    // single-char BPE pieces and the model outputs garbage. We reconstruct
    // them from config.
    inject_missing_specials(&mut tokens, &cfg, &eos_token_ids);

    let bpe = Bpe::new(tokens, merges);

    // chat section (optional)
    let (chat_format, chat_template) = if lm.file.chat_toml.is_empty() {
        // Try auto-detecting from eos token strings or model_type
        let model_type = cfg.get("model_type").and_then(|v| v.as_str()).unwrap_or("");
        let auto_fmt = detect_chat_format(&bpe, model_type);
        (auto_fmt, None)
    } else {
        let chat: toml::Value = toml::from_str(&lm.file.chat_toml)
            .map_err(|e| FormatError::Invalid(format!("chat parse: {e}")))?;
        let fmt = chat.get("format").and_then(|v| v.as_str()).map(String::from);
        let tpl = chat
            .get("template")
            .and_then(|v| v.as_str())
            .map(String::from);
        (fmt, tpl)
    };

    Ok(Tokenizer {
        bpe,
        eos_token_ids,
        chat_format,
        chat_template,
    })
}

/// Parse [tokens] and [merges] from the TOML vocab section.
/// Format:
///   [tokens]
///   0 = "<unk>"
///   1 = "<s>"
///   ...
///   [merges]
///   0 = ["a", "b"]
pub fn parse_vocab(vocab_toml: &str) -> Result<(Vec<(u32, String)>, Vec<(String, String)>), FormatError> {
    let mut tokens = Vec::new();
    let mut merges = Vec::new();
    let mut section = "";
    for line in vocab_toml.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if t == "[tokens]" {
            section = "tokens";
            continue;
        }
        if t == "[merges]" {
            section = "merges";
            continue;
        }
        if section == "tokens" {
            if let Some((id_str, rest)) = t.split_once('=') {
                let id: u32 = match id_str.trim().parse() {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let val = rest.trim();
                if let Some(s) = parse_toml_string(val) {
                    tokens.push((id, s));
                }
            }
        } else if section == "merges" {
            if let Some((_, rest)) = t.split_once('=') {
                if let Some((a, b)) = parse_merge_pair(rest.trim()) {
                    merges.push((a, b));
                }
            }
        }
    }
    Ok((tokens, merges))
}

fn parse_toml_string(s: &str) -> Option<String> {
    let s = s.trim();
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        unescape(&s[1..s.len() - 1])
    } else if s.starts_with('\'') && s.ends_with('\'') && s.len() >= 2 {
        Some(s[1..s.len() - 1].to_string())
    } else {
        None
    }
}

fn unescape(s: &str) -> Option<String> {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next()? {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                'r' => out.push('\r'),
                '\\' => out.push('\\'),
                '"' => out.push('"'),
                '\'' => out.push('\''),
                'u' => {
                    if chars.peek() == Some(&'{') {
                        chars.next();
                        let mut hex = String::new();
                        while let Some(&c) = chars.peek() {
                            if c == '}' {
                                chars.next();
                                break;
                            }
                            hex.push(c);
                            chars.next();
                        }
                        if let Ok(code) = u32::from_str_radix(&hex, 16) {
                            if let Some(c) = char::from_u32(code) {
                                out.push(c);
                            }
                        }
                    }
                }
                other => out.push(other),
            }
        } else {
            out.push(c);
        }
    }
    Some(out)
}

fn parse_merge_pair(s: &str) -> Option<(String, String)> {
    // `["a", "b"]`
    let s = s.trim();
    let s = s.strip_prefix('[')?.strip_suffix(']')?.trim();
    let comma = s.find(',')?;
    let a = parse_toml_string(s[..comma].trim())?;
    let b = parse_toml_string(s[comma + 1..].trim())?;
    Some((a, b))
}

/// Patch `tokens` with specials that are referenced by config but missing
/// from the [tokens] table. Uses three sources of information:
///   1. [tokenizer].eos_token string + eos_token_ids[0] (name ↔ id)
///   2. [tokenizer].pad_token string + eos_token_ids[1] (fallback pad id)
///   3. Well-known family conventions (qwen chatml: im_start = im_end - 1,
///      endoftext = im_start - 1)
fn inject_missing_specials(
    tokens: &mut Vec<(u32, String)>,
    cfg: &toml::Value,
    eos_ids: &[u32],
) {
    let have_id = |tokens: &[(u32, String)], id: u32| tokens.iter().any(|(i, _)| *i == id);
    let have_str = |tokens: &[(u32, String)], s: &str| tokens.iter().any(|(_, t)| t == s);
    let mut add = |tokens: &mut Vec<(u32, String)>, id: u32, name: String| {
        if !have_id(tokens, id) && !have_str(tokens, &name) {
            log::debug!("inject special: {id} = {name:?}");
            tokens.push((id, name));
        }
    };

    let tok_section = cfg.get("tokenizer");
    let eos_name = tok_section
        .and_then(|t| t.get("eos_token"))
        .and_then(|v| v.as_str());
    let pad_name = tok_section
        .and_then(|t| t.get("pad_token"))
        .and_then(|v| v.as_str());

    // (eos name, eos id)
    if let (Some(name), Some(&id)) = (eos_name, eos_ids.first()) {
        add(tokens, id, name.to_string());
    }
    // Secondary eos / pad — eos_token_ids often lists [main_eos, endoftext_id]
    if let (Some(name), Some(&id)) = (pad_name, eos_ids.get(1)) {
        add(tokens, id, name.to_string());
    }

    // ChatML convention: qwen models place specials in a dense block ending
    // at eos_token_ids[0]. If eos is `<|im_end|>`, <|im_start|> sits at id-1
    // and <|endoftext|> at id-2.
    if eos_name == Some("<|im_end|>") {
        if let Some(&eos) = eos_ids.first() {
            if eos >= 1 {
                add(tokens, eos - 1, "<|im_start|>".into());
            }
            if eos >= 2 {
                add(tokens, eos - 2, "<|endoftext|>".into());
            }
        }
    }
}

fn detect_chat_format(bpe: &Bpe, model_type: &str) -> Option<String> {
    // Heuristic: presence of specific special tokens.
    if bpe.id("<|im_end|>").is_some() {
        return Some("chatml".into());
    }
    if bpe.id("<|eot_id|>").is_some() {
        return Some("llama-3".into());
    }
    // Gemma 4 uses <|turn> / <turn|> markers — distinct from Gemma 1/2/3.
    if bpe.id("<|turn>").is_some() && bpe.id("<turn|>").is_some() {
        return Some("gemma4".into());
    }
    if bpe.id("<start_of_turn>").is_some() {
        return Some("gemma".into());
    }
    match model_type {
        "qwen2" | "qwen3" => Some("chatml".into()),
        _ => None,
    }
}
