//! cyb-stream — typed framed byte-stream protocol
//!
//! Frame format: `0x1F | sigil | render | varint | payload`
//!
//! - 0x1F  ASCII Unit Separator (never appears in valid UTF-8, spare in 256-symbol map)
//! - sigil  one byte from cybermark's 13 sigils (semantic role)
//! - render one ASCII letter from the 14-language render set (perception type)
//! - varint LEB128 unsigned integer (payload length in bytes)
//! - payload N bytes; may contain nested frames for composition chunks

pub use bytes;

use std::io::{self, Read, Write};

pub mod sigil {
    //! Cybermark sigil bytes — semantic role of a chunk.
    pub const HAX: u8 = b'#'; // 0x23 — content / particle identity
    pub const PAT: u8 = b'@'; // 0x40 — identity / agent / neuron
    pub const SIG: u8 = b'~'; // 0x7E — annotation / label / side-info
    pub const FAS: u8 = b'/'; // 0x2F — scope / containment / structure
    pub const BUC: u8 = b'$'; // 0x24 — economic / value-bearing
    pub const KET: u8 = b'^'; // 0x5E — lift / abstract / establish
    pub const ZAP: u8 = b'!'; // 0x21 — effect / imperative / action
    pub const DOT: u8 = b'.'; // 0x2E — transform / pipeline / apply
    pub const BAR: u8 = b'|'; // 0x7C — composition / code-with-data
    pub const TIS: u8 = b'='; // 0x3D — binding / equivalence
    pub const WUT: u8 = b'?'; // 0x3F — test / decision / input request
    pub const COL: u8 = b':'; // 0x3A — pair / key-value
    pub const LUS: u8 = b'+'; // 0x2B — augment / increment

    pub const ALL: [u8; 13] = [HAX, PAT, SIG, FAS, BUC, KET, ZAP, DOT, BAR, TIS, WUT, COL, LUS];

    pub fn name(byte: u8) -> &'static str {
        match byte {
            HAX => "hax", PAT => "pat", SIG => "sig", FAS => "fas",
            BUC => "buc", KET => "ket", ZAP => "zap", DOT => "dot",
            BAR => "bar", TIS => "tis", WUT => "wut", COL => "col",
            LUS => "lus", _ => "unknown",
        }
    }

    pub fn is_valid(byte: u8) -> bool {
        ALL.contains(&byte)
    }
}

pub mod render {
    //! Render type bytes — perception type / how to display a chunk.
    pub const STRUCT:    u8 = b's'; // collapsible tree (Nox)
    pub const BINARY:    u8 = b'b'; // raster image / pixels (Bt)
    pub const TEXT:      u8 = b't'; // prose / code (Rs)
    pub const FORMULA:   u8 = b'f'; // math notation (Tri)
    pub const VECTOR:    u8 = b'v'; // SVG / 2D / 3D (Arc/Ren/Dif)
    pub const MOVIE:     u8 = b'm'; // video frames (Seq)
    pub const TABLE:     u8 = b'T'; // 2D grid of records (Inf)
    pub const AUDIO:     u8 = b'a'; // waveform (Wav)
    pub const COMPONENT: u8 = b'c'; // nested composition (Ten)
    pub const TOKEN:     u8 = b'k'; // ledger / balance (Tok)
    // meta / stream-control
    pub const PROGRESS:  u8 = b'p'; // live progress bar
    pub const LOG:       u8 = b'l'; // structured log line
    pub const ERROR:     u8 = b'e'; // typed error with source location
    pub const STATUS:    u8 = b'x'; // end-of-command sentinel (exit code)
    pub const INPUT:     u8 = b'i'; // user → producer response

    pub const ALL: [u8; 15] = [
        STRUCT, BINARY, TEXT, FORMULA, VECTOR, MOVIE, TABLE, AUDIO,
        COMPONENT, TOKEN, PROGRESS, LOG, ERROR, STATUS, INPUT,
    ];

    pub fn name(byte: u8) -> &'static str {
        match byte {
            STRUCT    => "struct",   BINARY   => "binary",  TEXT     => "text",
            FORMULA   => "formula",  VECTOR   => "vector",  MOVIE    => "movie",
            TABLE     => "table",    AUDIO    => "audio",   COMPONENT => "component",
            TOKEN     => "token",    PROGRESS => "progress",LOG      => "log",
            ERROR     => "error",    STATUS   => "status",  INPUT    => "input",
            _ => "unknown",
        }
    }

    pub fn is_valid(byte: u8) -> bool {
        ALL.contains(&byte)
    }
}

/// Frame marker byte. ASCII Unit Separator (0x1F).
/// - Never appears in valid UTF-8 (multibyte sequences use 0xC0-0xFF leads)
/// - Not in the nox ISA (which uses 0x01-0x1E)
/// - Not a printable ASCII byte (0x20-0x7E)
pub const MARKER: u8 = 0x1F;

/// Opaque chunk id for in-place updates (e.g. progress dedup).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkId(pub u64);

/// A parsed cyb-stream chunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    pub sigil:   u8,
    pub render:  u8,
    pub payload: bytes::Bytes,
    pub id:      Option<ChunkId>,
}

impl Chunk {
    /// Create a plain UTF-8 text chunk `(#, t)`.
    pub fn text(s: &str) -> Self {
        Self::new(sigil::HAX, render::TEXT, bytes::Bytes::copy_from_slice(s.as_bytes()))
    }

    /// Create an annotation / label text chunk `(~, t)`.
    pub fn annotation(s: &str) -> Self {
        Self::new(sigil::SIG, render::TEXT, bytes::Bytes::copy_from_slice(s.as_bytes()))
    }

    /// Create a typed error chunk `(!, e)` — JSON payload `{level, source, message}`.
    pub fn error(message: &str) -> Self {
        let payload = format!(r#"{{"level":"error","source":"","message":{message:?}}}"#);
        Self::new(sigil::ZAP, render::ERROR, bytes::Bytes::from(payload.into_bytes()))
    }

    /// Create a log line chunk `(., l)` — JSON payload `{level, source, message}`.
    pub fn log(level: &str, source: &str, message: &str) -> Self {
        let payload = format!(r#"{{"level":{level:?},"source":{source:?},"message":{message:?}}}"#);
        Self::new(sigil::DOT, render::LOG, bytes::Bytes::from(payload.into_bytes()))
    }

    /// Create a progress chunk `(., p)` — JSON payload `{id, label, current, total}`.
    pub fn progress(id: u64, label: &str, current: u64, total: u64) -> Self {
        let payload = format!(r#"{{"id":{id},"label":{label:?},"current":{current},"total":{total}}}"#);
        let mut chunk = Self::new(sigil::DOT, render::PROGRESS, bytes::Bytes::from(payload.into_bytes()));
        chunk.id = Some(ChunkId(id));
        chunk
    }

    /// Create an end-of-command status chunk `(., x)` — JSON payload `{code}`.
    pub fn status(code: i32) -> Self {
        let payload = format!(r#"{{"code":{code}}}"#);
        Self::new(sigil::DOT, render::STATUS, bytes::Bytes::from(payload.into_bytes()))
    }

    /// Create a scope / breadcrumb chunk `(/, c)` with nested payload bytes.
    pub fn scope(nested: bytes::Bytes) -> Self {
        Self::new(sigil::FAS, render::COMPONENT, nested)
    }

    /// Create a composition container `(|, c)` with nested payload bytes.
    pub fn compose(nested: bytes::Bytes) -> Self {
        Self::new(sigil::BAR, render::COMPONENT, nested)
    }

    /// Base constructor.
    pub fn new(sigil: u8, render: u8, payload: bytes::Bytes) -> Self {
        Self { sigil, render, payload, id: None }
    }

    pub fn with_id(mut self, id: ChunkId) -> Self {
        self.id = Some(id);
        self
    }

    /// Encode this chunk to bytes (no id encoding — ids are transport-layer metadata).
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(3 + 9 + self.payload.len());
        buf.push(MARKER);
        buf.push(self.sigil);
        buf.push(self.render);
        encode_varint(self.payload.len() as u64, &mut buf);
        buf.extend_from_slice(&self.payload);
        buf
    }
}

// ── varint ───────────────────────────────────────────────────────────────────

/// Encode an unsigned 64-bit integer as LEB128 into `buf`.
pub fn encode_varint(mut n: u64, buf: &mut Vec<u8>) {
    loop {
        let low7 = (n & 0x7F) as u8;
        n >>= 7;
        if n == 0 {
            buf.push(low7);
            break;
        }
        buf.push(low7 | 0x80);
    }
}

/// Decode an unsigned LEB128 from `bytes` starting at `pos`.
/// Returns `(value, new_pos)` or `None` if the stream is truncated.
pub fn decode_varint(bytes: &[u8], pos: usize) -> Option<(u64, usize)> {
    let mut value: u64 = 0;
    let mut shift: u32 = 0;
    let mut cur = pos;
    loop {
        if cur >= bytes.len() {
            return None;
        }
        let b = bytes[cur];
        cur += 1;
        let low7 = (b & 0x7F) as u64;
        value |= low7 << shift;
        shift += 7;
        if b & 0x80 == 0 {
            return Some((value, cur));
        }
        if shift >= 64 {
            return None; // overflow
        }
    }
}

// ── Writer ────────────────────────────────────────────────────────────────────

/// Framing writer. Wraps any `io::Write` and emits cyb-stream frames.
pub struct Writer<W: Write> {
    inner: W,
}

impl<W: Write> Writer<W> {
    pub fn new(inner: W) -> Self {
        Self { inner }
    }

    pub fn write_chunk(&mut self, chunk: &Chunk) -> io::Result<()> {
        let mut hdr = Vec::with_capacity(3 + 9);
        hdr.push(MARKER);
        hdr.push(chunk.sigil);
        hdr.push(chunk.render);
        encode_varint(chunk.payload.len() as u64, &mut hdr);
        self.inner.write_all(&hdr)?;
        self.inner.write_all(&chunk.payload)
    }

    pub fn into_inner(self) -> W {
        self.inner
    }
}

// ── Reader ────────────────────────────────────────────────────────────────────

/// Result of a single `Reader::next_chunk` call.
#[derive(Debug)]
pub enum ReadResult {
    /// A complete, valid chunk.
    Chunk(Chunk),
    /// Not enough bytes yet — call again when more data is available.
    Pending,
    /// End of stream (clean EOF with no partial frame).
    Eof,
}

/// Framing reader state — buffers incoming bytes and emits chunks.
///
/// Designed for incremental feeding: call `feed()` as bytes arrive, then
/// drain with `next_chunk()` until `ReadResult::Pending`.
pub struct Reader {
    buf: Vec<u8>,
    pos: usize,
}

impl Reader {
    pub fn new() -> Self {
        Self { buf: Vec::new(), pos: 0 }
    }

    /// Append newly received bytes to the internal buffer.
    pub fn feed(&mut self, data: &[u8]) {
        if self.pos > 4096 && self.pos > self.buf.len() / 2 {
            // Compact: discard already-consumed bytes
            self.buf.drain(..self.pos);
            self.pos = 0;
        }
        self.buf.extend_from_slice(data);
    }

    /// Try to parse the next chunk from the buffer.
    pub fn next_chunk(&mut self) -> ReadResult {
        loop {
            let remaining = &self.buf[self.pos..];
            if remaining.is_empty() {
                return ReadResult::Eof;
            }

            // Scan for the 0x1F marker
            let marker_offset = match remaining.iter().position(|&b| b == MARKER) {
                Some(o) => o,
                None => {
                    // No marker anywhere — consume all, signal pending
                    self.pos = self.buf.len();
                    return ReadResult::Pending;
                }
            };

            if marker_offset > 0 {
                // Skip non-frame bytes before the marker
                self.pos += marker_offset;
                continue;
            }

            // We're at a 0x1F byte. Need: marker + sigil + render + at least 1 varint byte
            if remaining.len() < 4 {
                return ReadResult::Pending;
            }

            let sigil  = remaining[1];
            let render = remaining[2];

            // Decode varint starting at byte 3
            let varint_start = self.pos + 3;
            match decode_varint(&self.buf, varint_start) {
                None => return ReadResult::Pending,
                Some((payload_len, payload_start)) => {
                    let payload_end = payload_start + payload_len as usize;
                    if payload_end > self.buf.len() {
                        return ReadResult::Pending;
                    }
                    let payload = bytes::Bytes::copy_from_slice(&self.buf[payload_start..payload_end]);
                    self.pos = payload_end;

                    // Skip frames with unknown sigil/render rather than hard-erroring.
                    // (forward-compatibility: newer producers may emit chunk types we don't
                    //  know yet; still advance the cursor so we don't re-scan forever)
                    let chunk = Chunk { sigil, render, payload, id: None };
                    return ReadResult::Chunk(chunk);
                }
            }
        }
    }

    /// Convenience: parse all chunks from a `Read` source at once (non-streaming).
    pub fn read_all<R: Read>(mut r: R) -> io::Result<Vec<Chunk>> {
        let mut data = Vec::new();
        r.read_to_end(&mut data)?;
        let mut reader = Self::new();
        reader.feed(&data);
        let mut chunks = Vec::new();
        loop {
            match reader.next_chunk() {
                ReadResult::Chunk(c) => chunks.push(c),
                ReadResult::Pending | ReadResult::Eof => break,
            }
        }
        Ok(chunks)
    }
}

impl Default for Reader {
    fn default() -> Self {
        Self::new()
    }
}

// ── convenience: write chunks into a nested payload buffer ───────────────────

/// Encode a slice of chunks into a `Bytes` suitable as a component payload.
pub fn encode_nested(chunks: &[Chunk]) -> bytes::Bytes {
    let mut buf = Vec::new();
    for c in chunks {
        buf.extend_from_slice(&c.encode());
    }
    bytes::Bytes::from(buf)
}

/// Decode all chunks from a nested payload (component, table, etc.).
pub fn decode_nested(payload: &[u8]) -> Vec<Chunk> {
    let mut reader = Reader::new();
    reader.feed(payload);
    let mut out = Vec::new();
    loop {
        match reader.next_chunk() {
            ReadResult::Chunk(c) => out.push(c),
            ReadResult::Pending | ReadResult::Eof => break,
        }
    }
    out
}

// ── table helpers ─────────────────────────────────────────────────────────────

/// Build a `(#, T)` table chunk from column headers and rows of cell chunks.
///
/// Schema row: `(/, s)` containing `(~, t)` for each header.
/// Data rows:  `(:, s)` containing one cell chunk per column.
pub fn table_chunk(headers: &[&str], rows: Vec<Vec<Chunk>>) -> Chunk {
    let mut inner = Vec::new();

    // Schema row
    let header_chunks: Vec<Chunk> = headers.iter()
        .map(|h| Chunk::annotation(h))
        .collect();
    let schema = Chunk::new(sigil::FAS, render::STRUCT, encode_nested(&header_chunks));
    inner.push(schema);

    // Data rows
    for row in rows {
        let row_chunk = Chunk::new(sigil::COL, render::STRUCT, encode_nested(&row));
        inner.push(row_chunk);
    }

    Chunk::new(sigil::HAX, render::TABLE, encode_nested(&inner))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── varint roundtrip ──────────────────────────────────────────────────────

    fn varint_rt(n: u64) -> u64 {
        let mut buf = Vec::new();
        encode_varint(n, &mut buf);
        let (v, _) = decode_varint(&buf, 0).expect("decode");
        v
    }

    #[test]
    fn varint_roundtrip() {
        for n in [0u64, 1, 127, 128, 255, 256, 16383, 16384, u32::MAX as u64, u64::MAX] {
            assert_eq!(varint_rt(n), n, "varint failed for {n}");
        }
    }

    #[test]
    fn varint_single_byte_for_small() {
        let mut buf = Vec::new();
        encode_varint(63, &mut buf);
        assert_eq!(buf.len(), 1);
    }

    #[test]
    fn varint_two_bytes_for_128() {
        let mut buf = Vec::new();
        encode_varint(128, &mut buf);
        assert_eq!(buf.len(), 2);
    }

    // ── chunk encode / decode roundtrip ──────────────────────────────────────

    fn roundtrip(chunk: Chunk) -> Chunk {
        let encoded = chunk.encode();
        let chunks = Reader::read_all(encoded.as_slice()).unwrap();
        assert_eq!(chunks.len(), 1, "expected 1 chunk, got {}", chunks.len());
        chunks.into_iter().next().unwrap()
    }

    #[test]
    fn text_chunk_roundtrip() {
        let c = Chunk::text("hello, cyber");
        let r = roundtrip(c.clone());
        assert_eq!(r.sigil, sigil::HAX);
        assert_eq!(r.render, render::TEXT);
        assert_eq!(&r.payload[..], b"hello, cyber");
    }

    #[test]
    fn error_chunk_roundtrip() {
        let c = Chunk::error("something went wrong");
        let r = roundtrip(c);
        assert_eq!(r.sigil, sigil::ZAP);
        assert_eq!(r.render, render::ERROR);
        assert!(r.payload.starts_with(b"{"));
    }

    #[test]
    fn log_chunk_roundtrip() {
        let c = Chunk::log("info", "compiler", "build complete");
        let r = roundtrip(c);
        assert_eq!(r.sigil, sigil::DOT);
        assert_eq!(r.render, render::LOG);
    }

    #[test]
    fn progress_chunk_roundtrip() {
        let c = Chunk::progress(42, "building", 30, 100);
        let r = roundtrip(c.clone());
        assert_eq!(r.sigil, sigil::DOT);
        assert_eq!(r.render, render::PROGRESS);
    }

    #[test]
    fn status_chunk_roundtrip() {
        let c = Chunk::status(0);
        let r = roundtrip(c);
        assert_eq!(r.sigil, sigil::DOT);
        assert_eq!(r.render, render::STATUS);
        assert_eq!(&r.payload[..], b"{\"code\":0}");
    }

    #[test]
    fn annotation_chunk_roundtrip() {
        let c = Chunk::annotation("~/cyber/cyb");
        let r = roundtrip(c);
        assert_eq!(r.sigil, sigil::SIG);
        assert_eq!(r.render, render::TEXT);
    }

    #[test]
    fn empty_payload_chunk() {
        let c = Chunk::new(sigil::DOT, render::STATUS, bytes::Bytes::new());
        let r = roundtrip(c);
        assert!(r.payload.is_empty());
    }

    #[test]
    fn large_payload_roundtrip() {
        let big = "x".repeat(100_000);
        let c = Chunk::text(&big);
        let r = roundtrip(c);
        assert_eq!(r.payload.len(), 100_000);
    }

    // ── v0 chunk catalog — every defined type ─────────────────────────────────

    #[test]
    fn all_v0_chunk_types() {
        let catalog: &[(u8, u8)] = &[
            (sigil::HAX, render::TEXT),
            (sigil::HAX, render::TABLE),
            (sigil::PAT, render::TEXT),
            (sigil::SIG, render::TEXT),
            (sigil::FAS, render::COMPONENT),
            (sigil::ZAP, render::COMPONENT),
            (sigil::WUT, render::COMPONENT),
            (sigil::WUT, render::INPUT),
            (sigil::BUC, render::TEXT),
            (sigil::BUC, render::TOKEN),
            (sigil::LUS, render::TEXT),
            (sigil::BAR, render::COMPONENT),
            (sigil::DOT, render::COMPONENT),
            (sigil::COL, render::STRUCT),
            (sigil::FAS, render::STRUCT),
            (sigil::DOT, render::PROGRESS),
            (sigil::DOT, render::LOG),
            (sigil::ZAP, render::ERROR),
            (sigil::DOT, render::STATUS),
        ];
        for &(s, r) in catalog {
            let payload = bytes::Bytes::from_static(b"test");
            let c = Chunk::new(s, r, payload);
            let enc = c.encode();
            assert_eq!(enc[0], MARKER);
            assert_eq!(enc[1], s);
            assert_eq!(enc[2], r);
            let back = roundtrip(c);
            assert_eq!(&back.payload[..], b"test",
                "roundtrip failed for ({}, {})", sigil::name(s), render::name(r));
        }
    }

    // ── nested chunks ─────────────────────────────────────────────────────────

    #[test]
    fn nested_component_roundtrip() {
        let inner_text  = Chunk::text("hello");
        let inner_table = table_chunk(&["col1", "col2"], vec![
            vec![Chunk::text("a"), Chunk::text("b")],
            vec![Chunk::text("c"), Chunk::text("d")],
        ]);
        let nested_bytes = encode_nested(&[inner_text, inner_table]);
        let outer = Chunk::compose(nested_bytes);

        let enc = outer.encode();
        let decoded = Reader::read_all(enc.as_slice()).unwrap();
        assert_eq!(decoded.len(), 1);

        let inner_chunks = decode_nested(&decoded[0].payload);
        assert_eq!(inner_chunks.len(), 2);
        assert_eq!(inner_chunks[0].render, render::TEXT);
        assert_eq!(inner_chunks[1].render, render::TABLE);
    }

    #[test]
    fn table_chunk_structure() {
        let t = table_chunk(
            &["name", "size"],
            vec![
                vec![Chunk::text("Cargo.toml"), Chunk::text("512")],
            ],
        );
        assert_eq!(t.sigil, sigil::HAX);
        assert_eq!(t.render, render::TABLE);
        let rows = decode_nested(&t.payload);
        // First inner chunk: schema row (/, s)
        assert_eq!(rows[0].sigil, sigil::FAS);
        assert_eq!(rows[0].render, render::STRUCT);
        // Second inner chunk: data row (:, s)
        assert_eq!(rows[1].sigil, sigil::COL);
        assert_eq!(rows[1].render, render::STRUCT);
        // Cell in data row
        let cells = decode_nested(&rows[1].payload);
        assert_eq!(cells.len(), 2);
        assert_eq!(&cells[0].payload[..], b"Cargo.toml");
    }

    // ── partial / truncated frames ────────────────────────────────────────────

    #[test]
    fn truncated_frame_returns_pending() {
        let full = Chunk::text("hello").encode();
        // Feed only the first 3 bytes (marker + sigil + render, no varint/payload)
        let mut reader = Reader::new();
        reader.feed(&full[..3]);
        assert!(matches!(reader.next_chunk(), ReadResult::Pending));
    }

    #[test]
    fn truncated_payload_returns_pending() {
        let full = Chunk::text("hello world").encode();
        // Feed everything except the last byte
        let mut reader = Reader::new();
        reader.feed(&full[..full.len() - 1]);
        assert!(matches!(reader.next_chunk(), ReadResult::Pending));
    }

    #[test]
    fn incremental_feeding_assembles_chunk() {
        let full = Chunk::text("streaming data").encode();
        let mut reader = Reader::new();
        // Feed byte by byte
        for (i, &byte) in full.iter().enumerate() {
            reader.feed(&[byte]);
            if i < full.len() - 1 {
                assert!(matches!(reader.next_chunk(), ReadResult::Pending),
                    "should be Pending after {i} bytes");
            }
        }
        // After all bytes, should get the chunk
        match reader.next_chunk() {
            ReadResult::Chunk(c) => assert_eq!(&c.payload[..], b"streaming data"),
            other => panic!("expected Chunk, got {other:?}"),
        }
    }

    // ── malformed frames — skip to next 0x1F ─────────────────────────────────

    #[test]
    fn garbage_before_marker_is_skipped() {
        let mut data = vec![0x01, 0x02, 0x03, 0xFF, 0xAB]; // junk
        data.extend_from_slice(&Chunk::text("found").encode());
        let chunks = Reader::read_all(data.as_slice()).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(&chunks[0].payload[..], b"found");
    }

    #[test]
    fn multiple_chunks_sequential() {
        let mut data = Vec::new();
        data.extend_from_slice(&Chunk::text("first").encode());
        data.extend_from_slice(&Chunk::status(0).encode());
        data.extend_from_slice(&Chunk::error("oops").encode());
        let chunks = Reader::read_all(data.as_slice()).unwrap();
        assert_eq!(chunks.len(), 3);
        assert_eq!(&chunks[0].payload[..], b"first");
        assert_eq!(chunks[1].sigil, sigil::DOT);
        assert_eq!(chunks[2].sigil, sigil::ZAP);
    }

    #[test]
    fn writer_matches_encode() {
        let chunk = Chunk::text("via writer");
        let mut buf = Vec::new();
        let mut w = Writer::new(&mut buf);
        w.write_chunk(&chunk).unwrap();
        assert_eq!(buf, chunk.encode());
    }

    // ── constants sanity ──────────────────────────────────────────────────────

    #[test]
    fn marker_byte_is_0x1f() {
        assert_eq!(MARKER, 0x1F);
    }

    #[test]
    fn sigil_bytes_are_ascii() {
        for &s in &sigil::ALL {
            assert!(s.is_ascii(), "sigil 0x{s:02X} is not ASCII");
            assert!(sigil::name(s) != "unknown", "sigil 0x{s:02X} has no name");
        }
    }

    #[test]
    fn render_bytes_are_ascii_letters() {
        for &r in &render::ALL {
            assert!(r.is_ascii_alphabetic(), "render 0x{r:02X} ({}) is not ASCII alphabetic", r as char);
            assert!(render::name(r) != "unknown", "render 0x{r:02X} has no name");
        }
    }
}
