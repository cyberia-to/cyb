//! Canonical .model quantization round-trip tests.
//!
//! Cross-checks the encoders in `import::quant::canonical` against the
//! decoders in `run::backend::cpu::quant::canonical`. Both must agree
//! byte-for-byte; the f32 → encode → decode → f32 path must reproduce
//! the input within the per-encoding tolerance.
//!
//! Reference: `cyb/cyb-model` (the canonical .model spec).

use import::quant::canonical as enc;
use run::backend::cpu::quant::canonical as dec;

fn assert_within(actual: &[f32], expected: &[f32], tol: f32, label: &str) {
    assert_eq!(actual.len(), expected.len(), "{label}: length mismatch");
    let mut max_err = 0.0f32;
    for (i, (&a, &e)) in actual.iter().zip(expected.iter()).enumerate() {
        let err = (a - e).abs();
        if err > tol {
            panic!(
                "{label}: index {i} actual={a} expected={e} err={err} > tol={tol}"
            );
        }
        if err > max_err {
            max_err = err;
        }
    }
    eprintln!("{label}: max_err = {max_err:.6} (tol {tol})");
}

// ── u32 (16.16 fixed-point) ──────────────────────────────────────────────────

#[test]
fn u32_roundtrip_typical_weights() {
    // norms / biases live in u32 — typical magnitudes well below 1.
    let input = vec![0.0, 1.0, -1.0, 0.5, -0.5, 1e-3, -1e-3, 32767.0, -32768.0];
    let bytes = enc::f32_to_u32(&input);
    let back = dec::u32_to_f32(&bytes);
    // Resolution: 1/65536 ≈ 1.53e-5.
    assert_within(&back, &input, 2.0 / 65536.0, "u32 roundtrip");
}

#[test]
fn u32_zero_is_exact() {
    let input = vec![0.0; 16];
    let bytes = enc::f32_to_u32(&input);
    let back = dec::u32_to_f32(&bytes);
    for v in back {
        assert_eq!(v, 0.0);
    }
}

// ── u16 (8.8 fixed-point) ────────────────────────────────────────────────────

#[test]
fn u16_roundtrip_typical_weights() {
    // Half-precision slot — typical f16-source weights are within ±10.
    let input = vec![0.0, 1.0, -1.0, 0.5, -0.5, 0.1, -0.1, 127.0, -127.0];
    let bytes = enc::f32_to_u16(&input);
    let back = dec::u16_to_f32(&bytes);
    // Resolution: 1/256 ≈ 3.9e-3.
    assert_within(&back, &input, 2.0 / 256.0, "u16 roundtrip");
}

#[test]
fn u16_saturates_silently_above_128() {
    // Values beyond ±128 saturate. Caller's responsibility to ensure inputs
    // fit; this test documents the behaviour.
    let input = vec![1000.0, -1000.0];
    let bytes = enc::f32_to_u16(&input);
    let back = dec::u16_to_f32(&bytes);
    // i16::MAX / 256 ≈ 127.996; i16::MIN / 256 ≈ -128.0
    assert!(back[0] > 127.0 && back[0] < 128.0, "saturated +max: {}", back[0]);
    assert!(back[1] >= -128.0 && back[1] < -127.0, "saturated -min: {}", back[1]);
}

// ── q8 (32-value blocks, u16 scale + 32 i8) ──────────────────────────────────

#[test]
fn q8_roundtrip_random_block() {
    // One block of 32 values in a typical weight range.
    let input: Vec<f32> = (0..32).map(|i| (i as f32 - 16.0) * 0.05).collect();
    let bytes = enc::f32_to_q8(&input);
    assert_eq!(bytes.len(), 2 + 32);
    let back = dec::q8_to_f32(&bytes);
    // q8 resolution: 1/127 of the block max → ~0.006 for max ~ 0.8.
    let block_max = input.iter().fold(0.0f32, |a, &v| a.max(v.abs()));
    let q_step = block_max / 127.0;
    assert_within(&back, &input, q_step + 1.0 / 256.0, "q8 roundtrip");
}

#[test]
fn q8_zero_block_is_zeros() {
    let input = vec![0.0f32; 32];
    let bytes = enc::f32_to_q8(&input);
    let back = dec::q8_to_f32(&bytes);
    for v in back {
        assert_eq!(v, 0.0);
    }
}

// ── q4 (32-value blocks, u16 scale + 16 packed nibbles) ──────────────────────

#[test]
fn q4_roundtrip_block() {
    // Symmetric block.
    let input: Vec<f32> = (0..32).map(|i| (i as f32 - 16.0) * 0.05).collect();
    let bytes = enc::f32_to_q4(&input);
    assert_eq!(bytes.len(), 2 + 16);
    let back = dec::q4_to_f32(&bytes);
    // q4 resolution: 1/8 of the block max.
    let block_max = input.iter().fold(0.0f32, |a, &v| a.max(v.abs()));
    let q_step = block_max / 8.0;
    assert_within(&back, &input, q_step + 1.0 / 256.0, "q4 roundtrip");
}

#[test]
fn q4_zero_block_is_zeros() {
    let input = vec![0.0f32; 32];
    let bytes = enc::f32_to_q4(&input);
    let back = dec::q4_to_f32(&bytes);
    for v in back {
        assert_eq!(v, 0.0);
    }
}

#[test]
fn q4_block_size_strictness() {
    // Inputs that aren't a multiple of 32 must panic — caller bug, not silent
    // truncation.
    let input = vec![0.1f32; 30];
    let result = std::panic::catch_unwind(|| enc::f32_to_q4(&input));
    assert!(result.is_err(), "non-32-multiple should panic");
}

// ── ternary (2 bits per value) ───────────────────────────────────────────────

#[test]
fn ternary_roundtrip_exact_for_canonical_values() {
    // Inputs that are already in {-1, 0, +1} round-trip exactly.
    let input: Vec<f32> = (0..32)
        .map(|i| match i % 3 {
            0 => 0.0,
            1 => 1.0,
            _ => -1.0,
        })
        .collect();
    let bytes = enc::f32_to_ternary(&input);
    assert_eq!(bytes.len(), 8);
    let back = dec::ternary_to_f32(&bytes);
    assert_eq!(back, input);
}

#[test]
fn ternary_quantizes_by_threshold() {
    // |v| < 0.5 → 0; v >= 0.5 → +1; v <= -0.5 → -1.
    let input: Vec<f32> = vec![
        0.0, 0.49, -0.49, 0.5, -0.5, 0.7, -0.7, 0.1, -0.1, 0.6, -0.6, 1.5, -1.5, 0.0, 0.0, 0.0,
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    ];
    let bytes = enc::f32_to_ternary(&input);
    let back = dec::ternary_to_f32(&bytes);
    assert_eq!(back[0], 0.0);
    assert_eq!(back[1], 0.0); // 0.49 → 0
    assert_eq!(back[2], 0.0); // -0.49 → 0
    assert_eq!(back[3], 1.0); // 0.5 → +1
    assert_eq!(back[4], -1.0); // -0.5 → -1
    assert_eq!(back[5], 1.0);
    assert_eq!(back[6], -1.0);
    assert_eq!(back[7], 0.0);
    assert_eq!(back[8], 0.0);
    assert_eq!(back[9], 1.0);
    assert_eq!(back[10], -1.0);
    assert_eq!(back[11], 1.0); // 1.5 → +1
    assert_eq!(back[12], -1.0); // -1.5 → -1
}

// ── byte-layout sanity (encoder produces exactly the bytes the spec describes)

#[test]
fn q4_first_byte_is_u16_scale_lo_byte() {
    // For values [+max, -max, 0, 0, ..., 0] with max = 1.0, scale = 1.0.
    // u16 encoding of 1.0 = round(1.0 * 256) = 256 → bytes [0x00, 0x01].
    let mut input = vec![0.0f32; 32];
    input[0] = 1.0;
    input[1] = -1.0;
    let bytes = enc::f32_to_q4(&input);
    assert_eq!(bytes[0], 0x00, "u16 scale lo byte");
    assert_eq!(bytes[1], 0x01, "u16 scale hi byte");
    // Nibble pair (lo=index 0=+1.0→7+8=15=0xF; hi=index 1=-1.0→-8+8=0=0x0)
    // Packed: lo | (hi << 4) = 0x0F.
    assert_eq!(bytes[2], 0x0F, "first nibble pair");
}

#[test]
fn q8_first_two_bytes_are_u16_scale() {
    let mut input = vec![0.0f32; 32];
    input[0] = 1.0;
    let bytes = enc::f32_to_q8(&input);
    let scale_u16 = i16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(scale_u16, 256, "scale u16 = round(1.0 * 256) = 256");
    // First i8 should be +127 (max).
    assert_eq!(bytes[2] as i8, 127);
}
