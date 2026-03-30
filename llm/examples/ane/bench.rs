//! ANE kernel benchmark — measure compilation time and throughput
//! cargo run --release --example bench

use ane::config;
use ane::mil::{self, ffn, projection, sdpa};
use ane::surface::{f32_to_fp16, AneSurface};
use ane::{AneModel, MilProgram};
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = config::qwen3_06b();
    println!("=== ANE Kernel Benchmark: {} ===\n", cfg.name);

    // Benchmark matmul at various sizes
    println!("── Matmul compile + eval ──");
    for &(ic, oc, seq) in &[
        (64, 64, 64),
        (128, 128, 128),
        (256, 256, 256),
        (512, 512, 256),
        (1024, 1024, 256),
    ] {
        let program = mil::matmul(ic, oc, seq);
        let t0 = Instant::now();
        let mut model = AneModel::compile(&program, &[])?;
        model.load()?;
        let compile_ms = t0.elapsed().as_millis();

        let input = AneSurface::new(program.input_bytes())?;
        let output = AneSurface::new(program.output_bytes())?;
        input.with_data_mut(|d| d.iter_mut().for_each(|v| *v = f32_to_fp16(0.01)));

        // Warmup
        model.run(&input, &output)?;

        // Benchmark
        let iters = 100;
        let t1 = Instant::now();
        for _ in 0..iters {
            model.run(&input, &output)?;
        }
        let eval_us = t1.elapsed().as_micros() as f64 / iters as f64;
        let flops = 2.0 * ic as f64 * oc as f64 * seq as f64;
        let tflops = flops / eval_us / 1e6;

        println!("  [{ic:4}×{oc:4}×{seq:3}] compile {compile_ms:4}ms | eval {eval_us:6.1}us | {tflops:.2} TFLOPS");
    }

    // Benchmark Qwen3 kernels
    println!("\n── Qwen3-0.6B kernel eval latency (100 iters) ──");

    let kernels: Vec<(&str, MilProgram, Vec<(&str, Vec<u8>)>)> = vec![
        ("qkv_proj", projection::qkv_proj(&cfg), vec![]),
        ("wo_fwd", projection::wo_fwd(&cfg), vec![]),
        ("ffn_fused", ffn::ffn_fused(&cfg, 1.0), vec![]),
        ("ffn_bwd_w2t", projection::ffn_bwd_w2t(&cfg), vec![]),
        ("ffn_bwd_w13t", projection::ffn_bwd_w13t(&cfg), vec![]),
        ("wot_bwd", projection::wot_bwd(&cfg), vec![]),
        ("q_bwd", projection::q_bwd(&cfg), vec![]),
        ("kv_bwd", projection::kv_bwd(&cfg), vec![]),
    ];

    for (name, program, weights) in &kernels {
        let w_ref: Vec<(&str, &[u8])> = weights.iter().map(|(k, v)| (*k, v.as_slice())).collect();
        let mut model = AneModel::compile(program, &w_ref)?;
        model.load()?;
        let input = AneSurface::new(program.input_bytes())?;
        let output = AneSurface::new(program.output_bytes())?;
        input.with_data_mut(|d| d.iter_mut().for_each(|v| *v = f32_to_fp16(0.01)));

        model.run(&input, &output)?; // warmup

        let iters = 100;
        let t = Instant::now();
        for _ in 0..iters {
            model.run(&input, &output)?;
        }
        let us = t.elapsed().as_micros() as f64 / iters as f64;
        println!("  {:20} {:6.1}us", name, us);
    }

    // SDPA kernels with weights
    println!("\n── SDPA kernels (with BLOBFILE weights) ──");
    let sdpa_cases: Vec<(&str, MilProgram, Vec<(&str, Vec<u8>)>)> = vec![
        (
            "sdpa_fwd",
            sdpa::sdpa_fwd(&cfg),
            sdpa::sdpa_fwd_weights(&cfg)
                .into_iter()
                .map(|(k, v)| (k, v))
                .collect(),
        ),
        (
            "sdpa_bwd1",
            sdpa::sdpa_bwd1(&cfg),
            sdpa::sdpa_bwd1_weights(&cfg)
                .into_iter()
                .map(|(k, v)| (k, v))
                .collect(),
        ),
        ("sdpa_bwd2", sdpa::sdpa_bwd2(&cfg), vec![]),
    ];

    for (name, program, weights) in &sdpa_cases {
        let w_ref: Vec<(&str, &[u8])> = weights.iter().map(|(k, v)| (*k, v.as_slice())).collect();
        let mut model = AneModel::compile(program, &w_ref)?;
        model.load()?;
        let input = AneSurface::new(program.input_bytes())?;
        let output = AneSurface::new(program.output_bytes())?;
        input.with_data_mut(|d| d.iter_mut().for_each(|v| *v = f32_to_fp16(0.01)));

        model.run(&input, &output)?;

        let iters = 50;
        let t = Instant::now();
        for _ in 0..iters {
            model.run(&input, &output)?;
        }
        let us = t.elapsed().as_micros() as f64 / iters as f64;
        println!("  {:20} {:6.1}us", name, us);
    }

    println!("\nBenchmark complete.");
    Ok(())
}
