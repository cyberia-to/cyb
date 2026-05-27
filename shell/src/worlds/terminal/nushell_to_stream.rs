//! PipelineData → cyb-stream chunk adapter.
//!
//! Converts nushell's structured output types into typed cyb-stream chunks
//! without ever converting to ANSI byte sequences. Runs on the eval thread
//! (not the Bevy main thread), so it must be Send.

use std::io::Read;
use std::sync::mpsc::Sender;

use nu_protocol::{PipelineData, Value};
use nu_protocol::engine::{EngineState, Stack};

use cyb_stream::{Chunk, sigil, render, encode_nested, table_chunk};

pub enum StreamMsg {
    Chunk(Chunk),
    Done { error: Option<String> },
}

/// Top-level entry point. Call this after `eval_block` returns.
/// Drains `data` to completion, sending chunks into `tx`.
/// Never panics — unknown value variants fall through to debug text.
pub fn pipeline_to_chunks(
    data: PipelineData,
    engine_state: &mut EngineState,
    stack: &mut Stack,
    tx: &Sender<StreamMsg>,
) {
    match data {
        PipelineData::Empty => {}
        PipelineData::Value(Value::Nothing { .. }, _) => {}
        PipelineData::Value(v, _) => {
            value_to_chunks(v, engine_state, stack, tx);
        }
        PipelineData::ListStream(stream, _) => {
            // Collect all records; if uniform shape → table, else → list of chunks
            let vals: Vec<Value> = stream.into_iter().collect();
            let chunk = list_to_chunk(vals, engine_state, stack);
            let _ = tx.send(StreamMsg::Chunk(chunk));
        }
        PipelineData::ByteStream(stream, _) => {
            byte_stream_to_chunks(stream, tx);
        }
    }
}

/// Convert a single `Value` to one or more chunks.
pub fn value_to_chunks(
    v: Value,
    engine_state: &mut EngineState,
    stack: &mut Stack,
    tx: &Sender<StreamMsg>,
) {
    let chunk = match v {
        Value::String  { val, .. }  => Chunk::text(&val),
        Value::Int     { val, .. }  => Chunk::text(&val.to_string()),
        Value::Float   { val, .. }  => Chunk::text(&val.to_string()),
        Value::Bool    { val, .. }  => Chunk::text(if val { "true" } else { "false" }),
        Value::Filesize{ val, .. }  => Chunk::text(&format_filesize(val.get())),
        Value::Duration{ val, .. }  => Chunk::text(&format_duration(val)),
        Value::Date    { val, .. }  => Chunk::text(&val.to_string()),
        Value::Range   { val, .. }  => Chunk::text(&format!("{val:?}")),
        Value::Record  { val, .. }  => record_to_chunk(val.into_owned().into_iter().collect()),
        Value::List    { vals, .. } => list_to_chunk(vals, engine_state, stack),
        Value::Nothing { .. }       => return,
        Value::Error   { error, .. }=> error_to_chunk(&error.to_string()),
        // Closure, Custom, Block — show as dim debug label
        other => Chunk::annotation(&format!("[{}]", other.get_type())),
    };
    let _ = tx.send(StreamMsg::Chunk(chunk));
}

/// Bytes from an external command — stream as `(~, t)` text chunks.
pub fn byte_stream_to_chunks(
    stream: nu_protocol::ByteStream,
    tx: &Sender<StreamMsg>,
) {
    const BUF: usize = 4096;
    let Some(mut reader) = stream.reader() else { return };
    let mut buf = [0u8; BUF];
    let mut pending_line = Vec::new();

    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                pending_line.extend_from_slice(&buf[..n]);
                // Flush complete lines as text chunks; keep the incomplete tail
                while let Some(pos) = pending_line.iter().position(|&b| b == b'\n') {
                    let line_bytes = pending_line.drain(..=pos).collect::<Vec<_>>();
                    let text = String::from_utf8_lossy(&line_bytes);
                    let trimmed = text.trim_end();
                    if !trimmed.is_empty() {
                        let chunk = Chunk::new(
                            sigil::SIG,
                            render::TEXT,
                            cyb_stream::bytes::Bytes::from(trimmed.to_owned().into_bytes()),
                        );
                        if tx.send(StreamMsg::Chunk(chunk)).is_err() { return; }
                    }
                }
            }
            Err(_) => break,
        }
    }

    // Flush any remaining bytes (no trailing newline)
    if !pending_line.is_empty() {
        let text = String::from_utf8_lossy(&pending_line);
        let trimmed = text.trim_end();
        if !trimmed.is_empty() {
            let chunk = Chunk::new(
                sigil::SIG,
                render::TEXT,
                cyb_stream::bytes::Bytes::from(trimmed.to_owned().into_bytes()),
            );
            let _ = tx.send(StreamMsg::Chunk(chunk));
        }
    }
}

/// A list of values: if all are records with the same key set → table; else struct list.
pub fn list_to_chunk(
    vals: Vec<Value>,
    _engine_state: &mut EngineState,
    _stack: &mut Stack,
) -> Chunk {
    if vals.is_empty() {
        return Chunk::annotation("(empty list)");
    }

    // Detect uniform record shape
    let first_keys = if let Value::Record { val, .. } = &vals[0] {
        Some(val.columns().cloned().collect::<Vec<_>>())
    } else {
        None
    };

    if let Some(ref headers) = first_keys {
        let all_records = vals.iter().all(|v| {
            if let Value::Record { val, .. } = v {
                val.columns().cloned().collect::<Vec<_>>() == *headers
            } else {
                false
            }
        });

        if all_records {
            return records_to_table(headers, &vals);
        }
    }

    // Mixed list — render as a struct-tree
    let inner: Vec<Chunk> = vals.into_iter()
        .enumerate()
        .map(|(i, v)| {
            let label = Chunk::annotation(&i.to_string());
            let value = value_as_chunk(v);
            let pair = encode_nested(&[label, value]);
            Chunk::new(sigil::COL, render::STRUCT, pair)
        })
        .collect();
    Chunk::new(sigil::FAS, render::STRUCT, encode_nested(&inner))
}

/// Build a `(#, T)` table chunk from a slice of uniform Records.
fn records_to_table(headers: &[String], vals: &[Value]) -> Chunk {
    let header_refs: Vec<&str> = headers.iter().map(|s| s.as_str()).collect();
    let rows: Vec<Vec<Chunk>> = vals.iter().map(|v| {
        if let Value::Record { val, .. } = v {
            headers.iter().map(|key| {
                val.get(key)
                    .map(|cell| value_as_chunk(cell.clone()))
                    .unwrap_or_else(|| Chunk::text(""))
            }).collect()
        } else {
            vec![value_as_chunk(v.clone())]
        }
    }).collect();
    table_chunk(&header_refs, rows)
}

/// Build a `(:, s)` key-value chunk from a Record.
pub fn record_to_chunk(fields: Vec<(String, Value)>) -> Chunk {
    let pairs: Vec<Chunk> = fields.into_iter().map(|(key, val)| {
        let k = Chunk::annotation(&key);
        let v = value_as_chunk(val);
        Chunk::new(sigil::COL, render::STRUCT, encode_nested(&[k, v]))
    }).collect();
    Chunk::new(sigil::FAS, render::STRUCT, encode_nested(&pairs))
}

/// Convert a nushell error into a `(!, e)` chunk.
pub fn error_to_chunk(msg: &str) -> Chunk {
    Chunk::error(msg)
}

/// Convert a single Value to its most natural Chunk representation (for cells, etc.).
fn value_as_chunk(v: Value) -> Chunk {
    match v {
        Value::String  { val, .. }  => Chunk::text(&val),
        Value::Int     { val, .. }  => Chunk::text(&val.to_string()),
        Value::Float   { val, .. }  => Chunk::text(&val.to_string()),
        Value::Bool    { val, .. }  => Chunk::text(if val { "true" } else { "false" }),
        Value::Filesize{ val, .. }  => Chunk::text(&format_filesize(val.get())),
        Value::Duration{ val, .. }  => Chunk::text(&format_duration(val)),
        Value::Date    { val, .. }  => Chunk::text(&val.to_string()),
        Value::Nothing { .. }       => Chunk::text(""),
        Value::Error   { error, .. }=> error_to_chunk(&error.to_string()),
        Value::Record  { val, .. }  => record_to_chunk(val.into_owned().into_iter().collect()),
        Value::List    { vals, .. } => {
            // Nested list in a cell — show first few items
            let preview: Vec<Chunk> = vals.into_iter().take(5).map(value_as_chunk).collect();
            Chunk::new(sigil::FAS, render::STRUCT, encode_nested(&preview))
        }
        other => Chunk::annotation(&format!("[{}]", other.get_type())),
    }
}

fn format_filesize(bytes: i64) -> String {
    const KB: i64 = 1024;
    const MB: i64 = KB * 1024;
    const GB: i64 = MB * 1024;
    match bytes.abs() {
        b if b >= GB => format!("{:.1} GB", bytes as f64 / GB as f64),
        b if b >= MB => format!("{:.1} MB", bytes as f64 / MB as f64),
        b if b >= KB => format!("{:.1} KB", bytes as f64 / KB as f64),
        _ => format!("{bytes} B"),
    }
}

fn format_duration(nanos: i64) -> String {
    let abs = nanos.unsigned_abs();
    let sign = if nanos < 0 { "-" } else { "" };
    const US: u64 = 1_000;
    const MS: u64 = US * 1000;
    const SEC: u64 = MS * 1000;
    const MIN: u64 = SEC * 60;
    const HR: u64 = MIN * 60;
    match abs {
        n if n >= HR  => format!("{sign}{}h {}m", n / HR, (n % HR) / MIN),
        n if n >= MIN => format!("{sign}{}m {}s", n / MIN, (n % MIN) / SEC),
        n if n >= SEC => format!("{sign}{:.2}s", n as f64 / SEC as f64),
        n if n >= MS  => format!("{sign}{:.1}ms", n as f64 / MS as f64),
        n if n >= US  => format!("{sign}{:.1}µs", n as f64 / US as f64),
        n             => format!("{sign}{n}ns"),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests (no Bevy, no actual nushell eval)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::{record, Span};

    fn span() -> Span { Span::new(0, 0) }

    // ── value_as_chunk ────────────────────────────────────────────────────────

    #[test]
    fn string_becomes_text_chunk() {
        let v = Value::string("hello", span());
        let c = value_as_chunk(v);
        assert_eq!(c.sigil, sigil::HAX);
        assert_eq!(c.render, render::TEXT);
        assert_eq!(&c.payload[..], b"hello");
    }

    #[test]
    fn int_becomes_text_chunk() {
        let v = Value::int(42, span());
        let c = value_as_chunk(v);
        assert_eq!(&c.payload[..], b"42");
    }

    #[test]
    fn bool_becomes_text_chunk() {
        let v = Value::bool(true, span());
        let c = value_as_chunk(v);
        assert_eq!(&c.payload[..], b"true");
    }

    #[test]
    fn nothing_becomes_empty_text() {
        let v = Value::nothing(span());
        let c = value_as_chunk(v);
        assert!(c.payload.is_empty());
    }

    // ── record_to_chunk ───────────────────────────────────────────────────────

    #[test]
    fn record_becomes_struct_chunk() {
        let fields = vec![
            ("name".to_string(), Value::string("alice", span())),
            ("age".to_string(),  Value::int(30, span())),
        ];
        let c = record_to_chunk(fields);
        assert_eq!(c.sigil, sigil::FAS);
        assert_eq!(c.render, render::STRUCT);
        // Inner is a sequence of (: s) pairs
        let inner = cyb_stream::decode_nested(&c.payload);
        assert_eq!(inner.len(), 2);
        assert_eq!(inner[0].sigil, sigil::COL);
        assert_eq!(inner[0].render, render::STRUCT);
    }

    // ── list_to_chunk → table ─────────────────────────────────────────────────

    #[test]
    fn uniform_records_become_table() {
        let rows = vec![
            Value::record(
                record! { "name" => Value::string("foo", span()),
                          "size" => Value::int(100, span()) },
                span(),
            ),
            Value::record(
                record! { "name" => Value::string("bar", span()),
                          "size" => Value::int(200, span()) },
                span(),
            ),
        ];

        let mut es = nu_cmd_lang::create_default_context();
        let mut stack = nu_protocol::engine::Stack::new();
        let c = list_to_chunk(rows, &mut es, &mut stack);
        assert_eq!(c.sigil, sigil::HAX);
        assert_eq!(c.render, render::TABLE);
        // Inner: schema row + 2 data rows
        let inner = cyb_stream::decode_nested(&c.payload);
        assert_eq!(inner.len(), 3); // schema + row1 + row2
    }

    // ── mixed list → struct ───────────────────────────────────────────────────

    #[test]
    fn mixed_list_becomes_struct() {
        let vals = vec![
            Value::string("hello", span()),
            Value::int(42, span()),
        ];
        let mut es = nu_cmd_lang::create_default_context();
        let mut stack = nu_protocol::engine::Stack::new();
        let c = list_to_chunk(vals, &mut es, &mut stack);
        assert_eq!(c.sigil, sigil::FAS);
        assert_eq!(c.render, render::STRUCT);
    }

    // ── filesize / duration formatting ────────────────────────────────────────

    #[test]
    fn filesize_formats() {
        assert_eq!(format_filesize(0), "0 B");
        assert_eq!(format_filesize(1023), "1023 B");
        assert_eq!(format_filesize(1024), "1.0 KB");
        assert_eq!(format_filesize(1024 * 1024), "1.0 MB");
        assert_eq!(format_filesize(1024 * 1024 * 1024), "1.0 GB");
    }

    #[test]
    fn duration_formats() {
        assert!(format_duration(500).ends_with("ns"));
        assert!(format_duration(1_500).contains("µs"));
        assert!(format_duration(1_500_000).contains("ms"));
        assert!(format_duration(2_000_000_000).contains('s'));
    }

    // ── error_to_chunk ────────────────────────────────────────────────────────

    #[test]
    fn error_chunk_is_zap_error() {
        let c = error_to_chunk("file not found");
        assert_eq!(c.sigil, sigil::ZAP);
        assert_eq!(c.render, render::ERROR);
        let payload = std::str::from_utf8(&c.payload).unwrap();
        assert!(payload.contains("file not found"));
    }
}
