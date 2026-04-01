#![allow(unused_imports)]
//! Kernel benchmark — wgpu vs Metal dispatch comparison
//!
//! Measures raw GFLOPS for matmul and matvec kernels at realistic model dimensions.
//! Reports speedup and estimated tok/s for decode.
//!
//! Usage:
//!   cargo run --release -p cyb-llm --bin bench-metal
//!   cargo run --release -p cyb-llm --bin bench-metal -- --iters 200

use std::time::Instant;

// Q4_0 block: 2-byte scale + 16 bytes of nibbles = 18 bytes for 32 weights
const Q4_BLOCK_SIZE: usize = 18;

fn main() {
    env_logger::init();

    let iters: usize = std::env::args()
        .position(|a| a == "--iters")
        .and_then(|i| std::env::args().nth(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);

    let warmup = 5;

    println!("=== cyb-llm Kernel Benchmark ===");
    println!("Iterations: {} (warmup: {})\n", iters, warmup);

    // ── Metal backend ──
    #[cfg(target_os = "macos")]
    {
        use cyb_llm::backend::metal::{dispatch as mtl_disp, MetalPipelines};

        let p = MetalPipelines::new().expect("Metal init failed");
        println!("Metal: {}\n", "ready");

        // Qwen3-0.6b dimensions
        let hidden = 896usize;
        let inter = 4864usize;
        let vocab = 151936usize;
        let kv_dim = 128usize;
        let n_layers = 24usize;

        // ── Matmul (prefill) ──
        println!("── Matmul fp16 (prefill) ──");
        println!("{:<35} {:>10} {:>10}", "Kernel", "GFLOPS", "time_us");
        println!("{}", "-".repeat(57));

        for &(m, n, k) in &[
            (512, hidden, hidden),
            (512, hidden, inter),
            (512, inter, hidden),
        ] {
            let flops = 2.0 * m as f64 * n as f64 * k as f64;
            let a = random_f16_buffer(&p, m * k);
            let b = random_f16_buffer(&p, k * n);
            let c = p.alloc(m * n * 2).unwrap();

            // Warmup
            for _ in 0..warmup {
                mtl_disp::matmul_f16(&p, &a, &b, &c, m as u32, n as u32, k as u32);
            }

            // Timed
            let params: [u32; 3] = [m as u32, n as u32, k as u32];
            let mut total_gpu = 0.0;
            for _ in 0..iters {
                total_gpu += mtl_disp::timed_dispatch(
                    &p,
                    &p.matmul_f16,
                    &[(&a, 0), (&b, 1), (&c, 2)],
                    bytemuck::cast_slice(&params),
                    3,
                    (div_ceil(n, 64), div_ceil(m, 64), 1),
                    (512, 1, 1),
                );
            }
            let avg_us = total_gpu / iters as f64 * 1e6;
            let gflops = flops / (avg_us * 1e3);
            println!("{:<35} {:>10.0} {:>10.1}", format!("matmul_f16 {}x{}x{}", m, n, k), gflops, avg_us);
        }

        // ── Matvec Q4 (decode single) ──
        println!("\n── Matvec Q4 (decode batch=1) ──");
        println!("{:<35} {:>10} {:>10}", "Kernel", "GFLOPS", "time_us");
        println!("{}", "-".repeat(57));

        let decode_projections: &[(&str, usize, usize)] = &[
            ("q_proj", hidden, hidden),
            ("k_proj", hidden, kv_dim),
            ("v_proj", hidden, kv_dim),
            ("o_proj", hidden, hidden),
            ("gate_proj", hidden, inter),
            ("up_proj", hidden, inter),
            ("down_proj", inter, hidden),
        ];

        let mut total_layer_us = 0.0;
        for &(name, k, n) in decode_projections {
            let flops = 2.0 * n as f64 * k as f64;
            let x = random_f16_buffer(&p, k);
            let w = random_q4_buffer(&p, k, n);
            let y = p.alloc(n * 2).unwrap();

            for _ in 0..warmup {
                mtl_disp::matvec_q4(&p, &x, &w, &y, n as u32, k as u32);
            }

            let params: [u32; 2] = [n as u32, k as u32];
            let mut total_gpu = 0.0;
            for _ in 0..iters {
                total_gpu += mtl_disp::timed_dispatch(
                    &p,
                    &p.matvec_q4,
                    &[(&x, 0), (&w, 1), (&y, 2)],
                    bytemuck::cast_slice(&params),
                    3,
                    (div_ceil(n, 256), 1, 1),
                    (256, 1, 1),
                );
            }
            let avg_us = total_gpu / iters as f64 * 1e6;
            let gflops = flops / (avg_us * 1e3);
            total_layer_us += avg_us;
            println!("{:<35} {:>10.0} {:>10.1}", format!("matvec_q4 {} {}x{}", name, k, n), gflops, avg_us);
        }

        // LM head
        {
            let k = hidden;
            let n = vocab;
            let flops = 2.0 * n as f64 * k as f64;
            let x = random_f16_buffer(&p, k);
            let w = random_q4_buffer(&p, k, n);
            let y = p.alloc(n * 2).unwrap();

            for _ in 0..warmup {
                mtl_disp::matvec_q4(&p, &x, &w, &y, n as u32, k as u32);
            }

            let params: [u32; 2] = [n as u32, k as u32];
            let mut total_gpu = 0.0;
            for _ in 0..iters {
                total_gpu += mtl_disp::timed_dispatch(
                    &p,
                    &p.matvec_q4,
                    &[(&x, 0), (&w, 1), (&y, 2)],
                    bytemuck::cast_slice(&params),
                    3,
                    (div_ceil(n, 256), 1, 1),
                    (256, 1, 1),
                );
            }
            let avg_us = total_gpu / iters as f64 * 1e6;
            let gflops = flops / (avg_us * 1e3);
            println!("{:<35} {:>10.0} {:>10.1}", format!("matvec_q4 lm_head {}x{}", k, n), gflops, avg_us);

            let total_decode_us = total_layer_us * n_layers as f64 + avg_us;
            let toks = 1e6 / total_decode_us;
            println!("\n── Estimated decode (qwen3-0.6b, {} layers) ──", n_layers);
            println!("  per-layer: {:.0} us", total_layer_us);
            println!("  lm_head:   {:.0} us", avg_us);
            println!("  total:     {:.0} us", total_decode_us);
            println!("  tok/s:     {:.1}", toks);
        }

        // ── Matvec Q4 batched ──
        println!("\n── Matvec Q4 (decode batch=8) ──");
        println!("{:<35} {:>10} {:>10}", "Kernel", "GFLOPS", "time_us");
        println!("{}", "-".repeat(57));

        let batch = 8usize;
        let mut total_layer_batch_us = 0.0;
        for &(name, k, n) in decode_projections {
            let flops = 2.0 * batch as f64 * n as f64 * k as f64;
            let x = random_f16_buffer(&p, batch * k);
            let w = random_q4_buffer(&p, k, n);
            let y = p.alloc(batch * n * 2).unwrap();

            for _ in 0..warmup {
                mtl_disp::matvec_q4_batch(&p, &x, &w, &y, n as u32, k as u32);
            }

            let params: [u32; 2] = [n as u32, k as u32];
            let mut total_gpu = 0.0;
            for _ in 0..iters {
                total_gpu += mtl_disp::timed_dispatch(
                    &p,
                    &p.matvec_q4_batch,
                    &[(&x, 0), (&w, 1), (&y, 2)],
                    bytemuck::cast_slice(&params),
                    3,
                    (div_ceil(n, 256), 1, 1),
                    (256, 1, 1),
                );
            }
            let avg_us = total_gpu / iters as f64 * 1e6;
            let gflops = flops / (avg_us * 1e3);
            total_layer_batch_us += avg_us;
            println!("{:<35} {:>10.0} {:>10.1}", format!("matvec_q4_b8 {} {}x{}", name, k, n), gflops, avg_us);
        }

        {
            let k = hidden;
            let n = vocab;
            let flops = 2.0 * batch as f64 * n as f64 * k as f64;
            let x = random_f16_buffer(&p, batch * k);
            let w = random_q4_buffer(&p, k, n);
            let y = p.alloc(batch * n * 2).unwrap();

            for _ in 0..warmup {
                mtl_disp::matvec_q4_batch(&p, &x, &w, &y, n as u32, k as u32);
            }

            let params: [u32; 2] = [n as u32, k as u32];
            let mut total_gpu = 0.0;
            for _ in 0..iters {
                total_gpu += mtl_disp::timed_dispatch(
                    &p,
                    &p.matvec_q4_batch,
                    &[(&x, 0), (&w, 1), (&y, 2)],
                    bytemuck::cast_slice(&params),
                    3,
                    (div_ceil(n, 256), 1, 1),
                    (256, 1, 1),
                );
            }
            let avg_us = total_gpu / iters as f64 * 1e6;
            let gflops = flops / (avg_us * 1e3);
            println!("{:<35} {:>10.0} {:>10.1}", format!("matvec_q4_b8 lm_head {}x{}", k, n), gflops, avg_us);

            let total_decode_us = total_layer_batch_us * n_layers as f64 + avg_us;
            let toks = batch as f64 * 1e6 / total_decode_us;
            println!("\n── Estimated decode batch=8 (qwen3-0.6b) ──");
            println!("  per-layer: {:.0} us", total_layer_batch_us);
            println!("  lm_head:   {:.0} us", avg_us);
            println!("  total:     {:.0} us", total_decode_us);
            println!("  tok/s:     {:.1}", toks);
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        println!("Metal backend not available on this platform.");
        println!("wgpu-only benchmarks not yet implemented.");
    }
}

#[cfg(target_os = "macos")]
fn random_f16_buffer(
    p: &cyb_llm::backend::metal::MetalPipelines,
    count: usize,
) -> aruminium::MtlBuffer {
    let data: Vec<u16> = (0..count)
        .map(|i| aruminium::f32_to_fp16(((i % 256) as f32 - 128.0) * 0.001))
        .collect();
    p.upload_f16(&data).unwrap()
}

#[cfg(target_os = "macos")]
fn random_q4_buffer(
    p: &cyb_llm::backend::metal::MetalPipelines,
    k: usize,
    n: usize,
) -> aruminium::MtlBuffer {
    let blocks_per_col = k / 32;
    let total_blocks = blocks_per_col * n;
    let mut data = vec![0u8; total_blocks * Q4_BLOCK_SIZE];
    for i in 0..total_blocks {
        let off = i * Q4_BLOCK_SIZE;
        // scale = 0.01 as fp16
        let scale = aruminium::f32_to_fp16(0.01);
        data[off] = (scale & 0xFF) as u8;
        data[off + 1] = (scale >> 8) as u8;
        // nibbles: fill with pattern (4=zero-centered for Q4)
        for j in 0..16 {
            data[off + 2 + j] = 0x48; // nibbles (4,8) -> weights (-4, 0)
        }
    }
    p.upload_bytes(&data).unwrap()
}

fn div_ceil(a: usize, b: usize) -> usize {
    (a + b - 1) / b
}
