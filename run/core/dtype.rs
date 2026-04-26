//! Tensor element data types.
//!
//! Spec: specs/tensor.md, specs/quant.md

use serde::{Deserialize, Serialize};

/// Element data type.
///
/// **Canonical on-disk types** (`cyb/cyb-model` spec): the only types
/// that may appear in a `.model` tensor index. Five fixed encodings.
///
/// - `U32`: 16.16 fixed-point integer (norms, biases, full precision)
/// - `U16`: 8.8 fixed-point integer (half precision)
/// - `Q8`: 32-value blocks, u16 scale + 32 i8s
/// - `Q4`: 32-value blocks, u16 scale + 16 packed nibbles
/// - `Ternary`: 2 bits per value, codes 00/01/10
///
/// **Runtime types** (`F32`, `F16`, `BF16`, `I8`, `U8`, `Bool`):
/// in-memory tensors during compute. Never serialized to a canonical
/// `.model`. Activations, intermediate results, dequant outputs.
///
/// **Legacy GGUF source types** (`Q4_0`, `Q4_K`, `Q5_K`, `Q6_K`,
/// `Q2_K`, `Q3_K`, `Q8_0`): only produced by the GGUF reader at
/// import time; never written to a canonical `.model`. The import
/// pipeline dequants source → f32 → re-encodes as canonical.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum DType {
    // ── canonical on-disk ─────────────────────────────────────
    U32,
    U16,
    Q8,
    Q4,
    Ternary,
    // ── runtime / compute ─────────────────────────────────────
    F32,
    F16,
    BF16,
    I8,
    U8,
    Bool,
    // ── legacy GGUF source (import-side only) ─────────────────
    Q8_0,
    Q4_0,
    Q2_K,
    Q3_K,
    Q4_K,
    Q5_K,
    Q6_K,
}

impl DType {
    /// Block size in elements (1 for scalar types).
    pub fn block_size(self) -> usize {
        match self {
            // canonical scalars
            DType::U32 | DType::U16 => 1,
            // canonical blocks
            DType::Q4 | DType::Q8 => 32,
            DType::Ternary => 4, // 4 values per byte (canonical)
            // runtime scalars
            DType::F32 | DType::F16 | DType::BF16 | DType::I8 | DType::U8 | DType::Bool => 1,
            // legacy GGUF
            DType::Q4_0 | DType::Q8_0 => 32,
            DType::Q2_K | DType::Q3_K | DType::Q4_K | DType::Q5_K | DType::Q6_K => 256,
        }
    }

    /// Bytes per block (for scalar types: bytes per element).
    pub fn block_bytes(self) -> usize {
        match self {
            // canonical
            DType::U32 => 4,
            DType::U16 => 2,
            DType::Q4 => 2 + 16, // u16 scale + 16 packed nibbles
            DType::Q8 => 2 + 32, // u16 scale + 32 i8
            DType::Ternary => 1, // 4 values per byte (2 bits each)
            // runtime
            DType::F32 => 4,
            DType::F16 | DType::BF16 => 2,
            DType::I8 | DType::U8 | DType::Bool => 1,
            // legacy GGUF
            DType::Q4_0 => 18,
            DType::Q8_0 => 34,
            DType::Q2_K => 84,
            DType::Q3_K => 110,
            DType::Q4_K => 144,
            DType::Q5_K => 176,
            DType::Q6_K => 210,
        }
    }

    /// Total bytes required for `n` values stored in this dtype.
    ///
    /// Panics if `n` is not a multiple of `block_size()` for quantized types.
    pub fn bytes_for(self, n: usize) -> usize {
        let bs = self.block_size();
        assert!(
            n % bs == 0,
            "DType {self:?} requires block-aligned count, got {n}"
        );
        (n / bs) * self.block_bytes()
    }

    /// Canonical encoding string used in `.model` tensor index.
    ///
    /// Only the five canonical encodings + the legacy GGUF source
    /// names are stringifiable. Runtime types (F32, F16, BF16) are
    /// not stored on disk and therefore have no canonical string.
    pub fn as_str(self) -> &'static str {
        match self {
            // canonical (the only valid on-disk strings)
            DType::U32 => "u32",
            DType::U16 => "u16",
            DType::Q4 => "q4",
            DType::Q8 => "q8",
            DType::Ternary => "ternary",
            // runtime types — debug names only, never written
            DType::F32 => "f32",
            DType::F16 => "f16",
            DType::BF16 => "bf16",
            DType::I8 => "i8",
            DType::U8 => "u8",
            DType::Bool => "bool",
            // legacy GGUF source names — only present in source, never in canonical .model
            DType::Q8_0 => "q8_0",
            DType::Q4_0 => "q4_0",
            DType::Q2_K => "q2_k",
            DType::Q3_K => "q3_k",
            DType::Q4_K => "q4_k",
            DType::Q5_K => "q5_k",
            DType::Q6_K => "q6_k",
        }
    }

    /// Parse from canonical encoding string.
    ///
    /// `"u32"` / `"u16"` / `"q4"` / `"q8"` / `"ternary"` resolve to the
    /// canonical types. Other names (legacy GGUF, runtime IEEE) are
    /// recognised so import-time code can name source dtypes, but a
    /// canonical reader rejects anything outside the five canonical
    /// names.
    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            // canonical
            "u32" => DType::U32,
            "u16" => DType::U16,
            "q4" => DType::Q4,
            "q8" => DType::Q8,
            "ternary" => DType::Ternary,
            // runtime / source aliases (recognised, not canonical-on-disk)
            "f32" => DType::F32,
            "f16" => DType::F16,
            "bf16" => DType::BF16,
            "i8" => DType::I8,
            "u8" => DType::U8,
            "bool" => DType::Bool,
            "q4_0" => DType::Q4_0,
            "q8_0" => DType::Q8_0,
            "q2k" | "q2_k" => DType::Q2_K,
            "q3k" | "q3_k" => DType::Q3_K,
            "q4k" | "q4_k" => DType::Q4_K,
            "q5k" | "q5_k" => DType::Q5_K,
            "q6k" | "q6_k" => DType::Q6_K,
            _ => return None,
        })
    }

    /// True if this dtype is one of the five canonical on-disk encodings.
    /// A canonical `.model` reader must reject any tensor whose dtype is
    /// not canonical.
    pub fn is_canonical(self) -> bool {
        matches!(
            self,
            DType::U32 | DType::U16 | DType::Q4 | DType::Q8 | DType::Ternary
        )
    }

    /// True if this is a floating-point scalar type.
    pub fn is_float(self) -> bool {
        matches!(self, DType::F32 | DType::F16 | DType::BF16)
    }

    /// Stable u8 tag for binary serialization (e.g. graph section).
    pub fn tag(self) -> u8 {
        match self {
            DType::F32 => 0, DType::F16 => 1, DType::BF16 => 2,
            DType::I8 => 3,  DType::U8 => 4,  DType::Bool => 5,
            DType::Q8_0 => 6, DType::Q4_0 => 7,
            DType::Q2_K => 8, DType::Q3_K => 9, DType::Q4_K => 10,
            DType::Q5_K => 11, DType::Q6_K => 12, DType::Ternary => 13,
            DType::U32 => 14, DType::U16 => 15, DType::Q4 => 16, DType::Q8 => 17,
        }
    }

    pub fn from_tag(tag: u8) -> Option<Self> {
        Some(match tag {
            0 => DType::F32, 1 => DType::F16, 2 => DType::BF16,
            3 => DType::I8,  4 => DType::U8,  5 => DType::Bool,
            6 => DType::Q8_0, 7 => DType::Q4_0,
            8 => DType::Q2_K, 9 => DType::Q3_K, 10 => DType::Q4_K,
            11 => DType::Q5_K, 12 => DType::Q6_K, 13 => DType::Ternary,
            14 => DType::U32, 15 => DType::U16, 16 => DType::Q4, 17 => DType::Q8,
            _ => return None,
        })
    }

    /// True if this is a block-quantized type (needs dequant to use).
    pub fn is_quantized(self) -> bool {
        matches!(
            self,
            // canonical block types
            DType::Q4 | DType::Q8 | DType::Ternary
            // legacy GGUF block types
            | DType::Q4_0
            | DType::Q8_0
            | DType::Q2_K
            | DType::Q3_K
            | DType::Q4_K
            | DType::Q5_K
            | DType::Q6_K
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_block_sizes_match_spec() {
        // canonical
        assert_eq!(DType::U32.block_bytes(), 4);
        assert_eq!(DType::U16.block_bytes(), 2);
        assert_eq!(DType::Q4.block_size(), 32);
        assert_eq!(DType::Q4.block_bytes(), 18); // u16 scale + 16 packed nibbles
        assert_eq!(DType::Q8.block_size(), 32);
        assert_eq!(DType::Q8.block_bytes(), 34); // u16 scale + 32 i8s
        assert_eq!(DType::Ternary.block_size(), 4);
        assert_eq!(DType::Ternary.block_bytes(), 1);
    }

    #[test]
    fn legacy_block_sizes_kept_for_source_reading() {
        // GGUF source dtypes the import pipeline reads.
        assert_eq!(DType::Q4_0.block_size(), 32);
        assert_eq!(DType::Q4_K.block_size(), 256);
        assert_eq!(DType::Q4_K.block_bytes(), 144);
        assert_eq!(DType::Q6_K.block_bytes(), 210);
    }

    #[test]
    fn bytes_for_roundtrip() {
        assert_eq!(DType::U32.bytes_for(1024), 4096);
        assert_eq!(DType::Q4.bytes_for(32), 18);
        assert_eq!(DType::Q4.bytes_for(1024), 1024 / 32 * 18);
        assert_eq!(DType::Q4_K.bytes_for(256), 144);
    }

    #[test]
    #[should_panic(expected = "block-aligned")]
    fn bytes_for_unaligned_panics() {
        DType::Q4.bytes_for(30); // not a multiple of 32
    }

    #[test]
    fn canonical_string_roundtrip() {
        for dt in [DType::U32, DType::U16, DType::Q4, DType::Q8, DType::Ternary] {
            assert_eq!(DType::from_str(dt.as_str()), Some(dt));
            assert!(dt.is_canonical());
        }
    }

    #[test]
    fn runtime_and_legacy_dtypes_not_canonical() {
        for dt in [
            DType::F32,
            DType::F16,
            DType::BF16,
            DType::Q4_0,
            DType::Q4_K,
            DType::Q6_K,
            DType::Q8_0,
        ] {
            assert!(!dt.is_canonical(), "{dt:?} must not claim canonical");
        }
    }

    #[test]
    fn canonical_string_does_not_alias_legacy() {
        // The string "u32" used to mean F32. Canonical contract is "u32" =>
        // U32 (16.16 fixed-point), not IEEE F32. Lock that in.
        assert_eq!(DType::from_str("u32"), Some(DType::U32));
        assert_eq!(DType::from_str("u16"), Some(DType::U16));
        assert_eq!(DType::from_str("q4"), Some(DType::Q4));
        assert_eq!(DType::from_str("q8"), Some(DType::Q8));
        // IEEE float types use their own names now.
        assert_eq!(DType::from_str("f32"), Some(DType::F32));
        assert_eq!(DType::from_str("f16"), Some(DType::F16));
    }
}
