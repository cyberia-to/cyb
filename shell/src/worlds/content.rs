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
    *file::Particle::hash(text.as_bytes()).as_bytes()
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
    let created = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = writeln!(
            f,
            "{{\"particle\":\"{hex}\",\"text\":\"{escaped}\",\"created\":{created}}}"
        );
    }
}

/// One file's record: the text and, where the line carries it, when it
/// was (re-)remembered. The particle is the map key; this is what it names.
/// `created` is `None` for lines soma-kernel wrote before this field
/// existed, or ever writes without it — an unknown date stays unknown,
/// never a fabricated one.
pub struct FileRecord {
    pub text: String,
    pub created: Option<u64>,
}

/// Everything the store holds, with its metadata. The store keeps the last
/// line for a given particle, so a re-remembered particle's `created` is
/// its most recent remembering, not its first.
pub fn load_with_meta() -> HashMap<[u8; 32], FileRecord> {
    let mut map = HashMap::new();
    let Ok(body) = std::fs::read_to_string(store_path()) else {
        return map;
    };
    for line in body.lines() {
        let Some(hex) = json_field(line, "particle") else {
            continue;
        };
        let Some(text) = json_field(line, "text") else {
            continue;
        };
        let Some(hash) = parse_particle_hex(&hex) else {
            continue;
        };
        let created = json_u64_field(line, "created");
        map.insert(hash, FileRecord { text, created });
    }
    map
}

/// Decode a 64-char hex particle. Any line the store holds must come back
/// intact, never panic — a corrupted or foreign-written line (soma-kernel
/// is the second writer) is just skipped, not fatal to the read.
fn parse_particle_hex(hex: &str) -> Option<[u8; 32]> {
    if hex.len() != 64 || !hex.is_ascii() {
        return None;
    }
    let mut hash = [0u8; 32];
    for i in 0..32 {
        hash[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(hash)
}

/// Everything the store holds, particle → text.
pub fn load() -> HashMap<[u8; 32], String> {
    load_with_meta()
        .into_iter()
        .map(|(k, v)| (k, v.text))
        .collect()
}

/// One unsigned-integer field out of a hand-written JSON line — the numeric
/// counterpart to [`json_field`] (no quotes, no escapes).
fn json_u64_field(line: &str, name: &str) -> Option<u64> {
    let key = format!("\"{name}\":");
    let start = line.find(&key)? + key.len();
    let rest = &line[start..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    if end == 0 {
        return None;
    }
    rest[..end].parse().ok()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_particle_hex_round_trips() {
        let hex: String = particle_of("hello")
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(parse_particle_hex(&hex), Some(particle_of("hello")));
    }

    #[test]
    fn parse_particle_hex_rejects_non_ascii_without_panicking() {
        // "a" + a 3-byte '中' + 60 more ASCII bytes = 64 bytes, but the
        // even byte-index slicing this used to run unguarded (`&hex[0..2]`)
        // lands inside '中' — a byte index that is not a char boundary —
        // and panics instead of returning None.
        let s = format!("a中{}", "a".repeat(60));
        assert_eq!(s.len(), 64);
        assert_eq!(parse_particle_hex(&s), None);
    }

    #[test]
    fn parse_particle_hex_rejects_wrong_length_and_bad_digits() {
        assert_eq!(parse_particle_hex("00"), None);
        assert_eq!(parse_particle_hex(&"zz".repeat(32)), None);
    }
}
