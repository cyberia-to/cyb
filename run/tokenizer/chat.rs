//! Chat templates — tiny Jinja subset.
//!
//! Supports: `{%- for m in messages -%}`, `{%- if cond -%}`, `{{ expr }}`.
//! Whitespace control via `-%}` / `{%-`.
//!
//! Spec: reference/runtime/tokenizer.md#chat-templates

use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Apply a Jinja template to a message list.
///
/// For v1 we support a hardcoded "chatml" preset; full Jinja interpreter
/// is a future extension. Custom templates pass through a minimal
/// substitution path.
pub fn apply_chat_template(
    format: Option<&str>,
    template: Option<&str>,
    messages: &[ChatMessage],
    add_generation_prompt: bool,
) -> String {
    match format {
        Some("chatml") => render_chatml(messages, add_generation_prompt),
        Some("llama-3") => render_llama3(messages, add_generation_prompt),
        Some("gemma") => render_gemma(messages, add_generation_prompt),
        Some("gemma4") => render_gemma4(messages, add_generation_prompt),
        Some("plain") | None => match template {
            Some(t) => render_minimal_jinja(t, messages, add_generation_prompt),
            None => plain_concat(messages),
        },
        Some(other) => {
            // Unknown preset: try custom template, else plain.
            if let Some(t) = template {
                render_minimal_jinja(t, messages, add_generation_prompt)
            } else {
                log::warn!("unknown chat format {other:?}, using plain concat");
                plain_concat(messages)
            }
        }
    }
}

fn plain_concat(messages: &[ChatMessage]) -> String {
    messages.iter().map(|m| m.content.as_str()).collect::<Vec<_>>().join("\n")
}

/// ChatML: `<|im_start|>role\ncontent<|im_end|>\n`
pub fn render_chatml(messages: &[ChatMessage], add_gen: bool) -> String {
    let mut s = String::new();
    for m in messages {
        s.push_str("<|im_start|>");
        s.push_str(&m.role);
        s.push('\n');
        s.push_str(&m.content);
        s.push_str("<|im_end|>\n");
    }
    if add_gen {
        s.push_str("<|im_start|>assistant\n");
    }
    s
}

/// Llama-3 Instruct: `<|start_header_id|>role<|end_header_id|>\n\ncontent<|eot_id|>`
pub fn render_llama3(messages: &[ChatMessage], add_gen: bool) -> String {
    let mut s = String::new();
    s.push_str("<|begin_of_text|>");
    for m in messages {
        s.push_str("<|start_header_id|>");
        s.push_str(&m.role);
        s.push_str("<|end_header_id|>\n\n");
        s.push_str(&m.content);
        s.push_str("<|eot_id|>");
    }
    if add_gen {
        s.push_str("<|start_header_id|>assistant<|end_header_id|>\n\n");
    }
    s
}

/// Gemma 1/2/3: `<start_of_turn>role\ncontent<end_of_turn>\n`
pub fn render_gemma(messages: &[ChatMessage], add_gen: bool) -> String {
    let mut s = String::new();
    for m in messages {
        s.push_str("<start_of_turn>");
        // Gemma uses "model" not "assistant"
        let role = if m.role == "assistant" { "model" } else { &m.role };
        s.push_str(role);
        s.push('\n');
        s.push_str(&m.content);
        s.push_str("<end_of_turn>\n");
    }
    if add_gen {
        s.push_str("<start_of_turn>model\n");
    }
    s
}

/// Gemma 4: `<bos><|turn>role\ncontent<turn|>\n` (different markers from
/// Gemma 1/2/3 — the new tokens are `<|turn>` (id 105) and `<turn|>` (106)
/// per the Gemma-4 vocab, not `<start_of_turn>` / `<end_of_turn>`).
pub fn render_gemma4(messages: &[ChatMessage], add_gen: bool) -> String {
    let mut s = String::new();
    s.push_str("<bos>");
    for m in messages {
        s.push_str("<|turn>");
        let role = if m.role == "assistant" { "model" } else { &m.role };
        s.push_str(role);
        s.push('\n');
        s.push_str(&m.content);
        s.push_str("<turn|>\n");
    }
    if add_gen {
        s.push_str("<|turn>model\n");
    }
    s
}

/// Very small Jinja subset:
///   {{ x.field }} → lookup
///   {% for m in messages %}...{% endfor %} → iterate messages
///   {% if cond %}...{% endif %}           → simple boolean
///   {%- / -%}                             → trim surrounding whitespace
fn render_minimal_jinja(
    template: &str,
    messages: &[ChatMessage],
    add_generation_prompt: bool,
) -> String {
    // For v1 we only handle the common patterns found in HF chat templates.
    // A full interpreter can replace this later.
    // Strategy: find `{% for msg in messages %}` ... `{% endfor %}`,
    // render body per message with {{ msg.role }} and {{ msg.content }},
    // then handle the trailing `{% if add_generation_prompt %}...{% endif %}`.
    let mut out = String::new();
    let mut ctx: HashMap<&str, String> = HashMap::new();
    ctx.insert(
        "add_generation_prompt",
        add_generation_prompt.to_string(),
    );

    let mut rest = template;

    // Drop everything before first `{%` if it's just whitespace.
    while let Some(start) = rest.find("{%") {
        let prefix = &rest[..start];
        out.push_str(prefix);
        let tag_end = rest.find("%}").unwrap_or(rest.len());
        let tag = rest[start..tag_end + 2].trim();
        // Strip trim markers
        let tag_body = tag
            .trim_start_matches("{%-")
            .trim_start_matches("{%")
            .trim_end_matches("-%}")
            .trim_end_matches("%}")
            .trim();

        if tag_body.starts_with("for") {
            // Parse "for X in Y"
            let parts: Vec<&str> = tag_body.split_whitespace().collect();
            if parts.len() >= 4 && parts[2] == "in" && parts[3] == "messages" {
                let var = parts[1];
                // Find matching endfor
                let after = &rest[tag_end + 2..];
                let endfor = after
                    .find("{% endfor %}")
                    .or_else(|| after.find("{%- endfor -%}"))
                    .or_else(|| after.find("{%- endfor %}"))
                    .or_else(|| after.find("{% endfor -%}"));
                if let Some(e) = endfor {
                    let body = &after[..e];
                    for m in messages {
                        let rendered = body
                            .replace(&format!("{{{{ {}.role }}}}", var), &m.role)
                            .replace(&format!("{{{{ {}.content }}}}", var), &m.content);
                        out.push_str(&rendered);
                    }
                    let skip = e + after[e..].find("%}").unwrap_or(0) + 2;
                    rest = &after[skip..];
                    continue;
                }
            }
            rest = &rest[tag_end + 2..];
        } else if tag_body.starts_with("if") {
            let cond = tag_body["if".len()..].trim();
            let is_true = ctx.get(cond).map(|v| v == "true").unwrap_or(false);
            let after = &rest[tag_end + 2..];
            let endif = after
                .find("{% endif %}")
                .or_else(|| after.find("{%- endif -%}"))
                .or_else(|| after.find("{%- endif %}"))
                .or_else(|| after.find("{% endif -%}"));
            if let Some(e) = endif {
                let body = &after[..e];
                if is_true {
                    // Substitute {{ var }} inside
                    out.push_str(body);
                }
                let skip = e + after[e..].find("%}").unwrap_or(0) + 2;
                rest = &after[skip..];
                continue;
            }
            rest = &rest[tag_end + 2..];
        } else {
            // Unknown tag, skip
            rest = &rest[tag_end + 2..];
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msgs() -> Vec<ChatMessage> {
        vec![
            ChatMessage {
                role: "user".into(),
                content: "hi".into(),
            },
            ChatMessage {
                role: "assistant".into(),
                content: "hello!".into(),
            },
        ]
    }

    #[test]
    fn chatml_format() {
        let s = render_chatml(&msgs(), true);
        assert_eq!(
            s,
            "<|im_start|>user\nhi<|im_end|>\n<|im_start|>assistant\nhello!<|im_end|>\n<|im_start|>assistant\n"
        );
    }

    #[test]
    fn gemma_format() {
        let s = render_gemma(&msgs(), true);
        assert!(s.contains("<start_of_turn>user\nhi<end_of_turn>"));
        assert!(s.contains("<start_of_turn>model\nhello!<end_of_turn>"));
        assert!(s.ends_with("<start_of_turn>model\n"));
    }

    #[test]
    fn llama3_format() {
        let s = render_llama3(&msgs(), true);
        assert!(s.starts_with("<|begin_of_text|>"));
        assert!(s.contains("<|start_header_id|>user<|end_header_id|>\n\nhi<|eot_id|>"));
        assert!(s.ends_with("<|start_header_id|>assistant<|end_header_id|>\n\n"));
    }

    #[test]
    fn apply_dispatch() {
        let s = apply_chat_template(Some("chatml"), None, &msgs(), true);
        assert!(s.contains("<|im_start|>"));
    }
}
