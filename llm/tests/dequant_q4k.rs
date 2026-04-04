//! Test Q4_K dequantization against Python reference values

use cyb_llm::backend::wgpu::model::safetensors_to_f32;
use cyb_llm::ir::DType;

/// Parse hex string to bytes
fn from_hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

#[test]
fn test_q4k_dequant_first_block() {
    // First Q4_K superblock from blk.0.attn_q.weight in Qwen2.5-Coder-14B GGUF
    let raw = from_hex(concat!(
        "7b0cb91253ce5952a294ffaa618f1265",
        "92054f3584280455a48353b03a5413e3",
        "7463367854ac33492588944a80f54879",
        "67476a6888e7f748ab99eb4c985d57f6",
        "984c383178aa05ba5c685556e03b6aaf",
        "15373a1d1836312a482823080c1a0237",
        "392a28472713390f36100a23f8272e06",
        "467c6af67c6b559a12262625162b0477",
        "908e57538a1989285636485a6a597b58",
    ));

    assert_eq!(raw.len(), 144, "Q4_K block is 144 bytes");

    let result = safetensors_to_f32(&raw, DType::Q4_K);
    assert_eq!(result.len(), 256, "Q4_K block decodes to 256 values");

    // Reference values from Python dequant (first 32)
    let expected: [f32; 32] = [
        -0.017509937, -0.001922369, 0.050036192, -0.001922369,
        -0.007118225, 0.013665199, -0.007118225, -0.001922369,
        -0.007118225, -0.012314081, -0.012314081, -0.027901649,
        0.024056911, -0.007118225, -0.012314081, -0.012314081,
        -0.007118225, -0.012314081, 0.003273487, 0.013665199,
        -0.007118225, 0.034448624, -0.012314081, 0.018861055,
        -0.001922369, 0.013665199, -0.007118225, 0.024056911,
        -0.027901649, -0.001922369, 0.013665199, 0.018861055,
    ];

    for (i, (&got, &exp)) in result.iter().zip(expected.iter()).enumerate() {
        let diff = (got - exp).abs();
        assert!(
            diff < 1e-4,
            "val[{i}]: got={got:.6}, expected={exp:.6}, diff={diff:.2e}"
        );
    }
}
