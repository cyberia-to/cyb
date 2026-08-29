//! The local content store: particle → the bytes behind it.
//!
//! The cybergraph is the record, and it records *particles* — 32-byte
//! content hashes — and the links between them. It does not carry content,
//! and it must not: the graph gossips and converges, content is fetched by
//! hash from whoever holds it. On the network that store is IPFS; on this
//! machine it is `~/cyb/particles.jsonl`, append-only, one `{particle, text}`
//! object per line. soma-kernel writes the same file in the same format —
//! the store has two writers and one truth.
//!
//! Everything here is best-effort on the write side and total on the read
//! side: failing to remember a text must never break the act that produced
//! it, and any line the store does hold must come back intact.

use std::collections::HashMap;
use std::io::Write as _;
use std::path::PathBuf;

/// A particle is the canonical hash of its content — hemera, like everywhere
/// else in cyber. The same text is the same particle on every machine.
pub fn particle_of(text: &str) -> [u8; 32] {
    let h = hemera::hash(text.as_bytes());
    let b = h.as_bytes();
    let mut out = [0u8; 32];
    let n = b.len().min(32);
    out[..n].copy_from_slice(&b[..n]);
    out
}

/// The well-known particle com's own casts hang off: everything you typed,
/// reachable from one root, exactly as soma's exchanges hang off soma's.
pub fn com_anchor() -> [u8; 32] {
    particle_of("com")
}

fn store_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join("cyb").join("particles.jsonl")
}

/// Put `text` into the store under its own hash. Idempotent in effect —
/// re-remembering appends a duplicate line, and the reader keeps the last.
pub fn remember(text: &str) {
    // The anchors name themselves the first time anything is remembered, so
    // a graph view can always call the roots what they are.
    static ANCHORS: std::sync::Once = std::sync::Once::new();
    ANCHORS.call_once(|| {
        append("com");
    });
    append(text);
}

fn append(text: &str) {
    let path = store_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let hex: String = particle_of(text)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let escaped = text
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(f, "{{\"particle\":\"{hex}\",\"text\":\"{escaped}\"}}");
    }
}

/// Everything the store holds, particle → text.
pub fn load() -> HashMap<[u8; 32], String> {
    let mut map = HashMap::new();
    let Ok(body) = std::fs::read_to_string(store_path()) else { return map };
    for line in body.lines() {
        let Some(hex) = json_field(line, "particle") else { continue };
        let Some(text) = json_field(line, "text") else { continue };
        if hex.len() != 64 {
            continue;
        }
        let mut hash = [0u8; 32];
        let ok = (0..32).all(|i| {
            u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
                .map(|b| hash[i] = b)
                .is_ok()
        });
        if ok {
            map.insert(hash, text);
        }
    }
    map
}

/// One string field out of a hand-written JSON line — the exact mirror of the
/// writer above and of soma-kernel's. Escapes are undone here, so the map
/// holds the text as it was said.
pub fn json_field(line: &str, name: &str) -> Option<String> {
    let key = format!("\"{name}\":\"");
    let start = line.find(&key)? + key.len();
    let mut out = String::new();
    let mut chars = line[start..].chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some('n') => out.push('\n'),
                Some(other) => out.push(other),
                None => return None,
            },
            '"' => return Some(out),
            _ => out.push(c),
        }
    }
    None
}
