//! Test: compile all MIL kernels on ANE hardware
//! cargo run --example compile_kernels

use ane::config;
use ane::mil::{self, ffn, projection, sdpa};
use ane::AneModel;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = config::qwen3_06b();
    println!("Compiling all ANE kernels for {}...\n", cfg.name);

    let kernels: Vec<(&str, Box<dyn Fn() -> ane::MilProgram>, Vec<(&str, Vec<u8>)>)> = vec![
        (
            "matmul(64,64,64)",
            Box::new(|| mil::matmul(64, 64, 64)),
            vec![],
        ),
        ("qkv_proj", Box::new(|| projection::qkv_proj(&cfg)), vec![]),
        ("wo_fwd", Box::new(|| projection::wo_fwd(&cfg)), vec![]),
        (
            "ffn_fused(a=1.0)",
            Box::new(|| ffn::ffn_fused(&cfg, 1.0)),
            vec![],
        ),
        (
            "ffn_bwd_w2t",
            Box::new(|| projection::ffn_bwd_w2t(&cfg)),
            vec![],
        ),
        (
            "ffn_bwd_w13t",
            Box::new(|| projection::ffn_bwd_w13t(&cfg)),
            vec![],
        ),
        ("wot_bwd", Box::new(|| projection::wot_bwd(&cfg)), vec![]),
        ("q_bwd", Box::new(|| projection::q_bwd(&cfg)), vec![]),
        ("kv_bwd", Box::new(|| projection::kv_bwd(&cfg)), vec![]),
    ];

    let mut ok = 0;
    let mut fail = 0;

    for (name, gen, _weights) in &kernels {
        let program = gen();
        let (ic, isp) = program.input_shape();
        let (oc, osp) = program.output_shape();
        print!("  {:20} [{},{}] → [{},{}] ... ", name, ic, isp, oc, osp);
        let t = Instant::now();
        match AneModel::compile(&program, &[]) {
            Ok(_model) => {
                println!("OK ({}ms)", t.elapsed().as_millis());
                ok += 1;
            }
            Err(e) => {
                println!("FAIL: {}", e);
                fail += 1;
            }
        }
    }

    // SDPA kernels need BLOBFILE weights
    println!("\n  SDPA kernels (with BLOBFILE weights):");

    let sdpa_fwd_prog = sdpa::sdpa_fwd(&cfg);
    let sdpa_fwd_w = sdpa::sdpa_fwd_weights(&cfg);
    let weights_ref: Vec<(&str, &[u8])> =
        sdpa_fwd_w.iter().map(|(k, v)| (*k, v.as_slice())).collect();
    print!(
        "  {:20} [{},{}] → [{},{}] ... ",
        "sdpa_fwd",
        sdpa_fwd_prog.input_shape().0,
        sdpa_fwd_prog.input_shape().1,
        sdpa_fwd_prog.output_shape().0,
        sdpa_fwd_prog.output_shape().1
    );
    let t = Instant::now();
    match AneModel::compile(&sdpa_fwd_prog, &weights_ref) {
        Ok(_) => {
            println!("OK ({}ms)", t.elapsed().as_millis());
            ok += 1;
        }
        Err(e) => {
            println!("FAIL: {}", e);
            fail += 1;
        }
    }

    let sdpa_bwd1_prog = sdpa::sdpa_bwd1(&cfg);
    let sdpa_bwd1_w = sdpa::sdpa_bwd1_weights(&cfg);
    let weights_ref: Vec<(&str, &[u8])> = sdpa_bwd1_w
        .iter()
        .map(|(k, v)| (*k, v.as_slice()))
        .collect();
    print!(
        "  {:20} [{},{}] → [{},{}] ... ",
        "sdpa_bwd1",
        sdpa_bwd1_prog.input_shape().0,
        sdpa_bwd1_prog.input_shape().1,
        sdpa_bwd1_prog.output_shape().0,
        sdpa_bwd1_prog.output_shape().1
    );
    let t = Instant::now();
    match AneModel::compile(&sdpa_bwd1_prog, &weights_ref) {
        Ok(_) => {
            println!("OK ({}ms)", t.elapsed().as_millis());
            ok += 1;
        }
        Err(e) => {
            println!("FAIL: {}", e);
            fail += 1;
        }
    }

    let sdpa_bwd2_prog = sdpa::sdpa_bwd2(&cfg);
    print!(
        "  {:20} [{},{}] → [{},{}] ... ",
        "sdpa_bwd2",
        sdpa_bwd2_prog.input_shape().0,
        sdpa_bwd2_prog.input_shape().1,
        sdpa_bwd2_prog.output_shape().0,
        sdpa_bwd2_prog.output_shape().1
    );
    let t = Instant::now();
    match AneModel::compile(&sdpa_bwd2_prog, &[]) {
        Ok(_) => {
            println!("OK ({}ms)", t.elapsed().as_millis());
            ok += 1;
        }
        Err(e) => {
            println!("FAIL: {}", e);
            fail += 1;
        }
    }

    println!("\n  Result: {ok} OK, {fail} FAIL (total {})", ok + fail);
    if fail > 0 {
        std::process::exit(1);
    }
    Ok(())
}
