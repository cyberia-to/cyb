//! ANE inference with KV-cache — Qwen3-0.6B
//!
//! Prefill: QKV/Wo/FFN on ANE, QK-norm+RoPE+attention on CPU
//! Decode: all CPU with KV-cache
//!
//! Usage: cargo run --example infer -- --ckpt PATH [--temp 0.8] [--topk 40] [--maxlen 200]
//! Send token IDs via stdin (one per line, empty line to start generation)

use ane::config;
use ane::mil::{ffn, projection};
use ane::ops::{activation, attention, embed, rmsnorm, rope, sample};
use ane::staging;
use ane::surface::AneSurface;
use ane::weights::{self, KVCache, LayerWeights};
use ane::AneModel;
use std::io::BufRead;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = config::qwen3_06b();
    eprintln!(
        "=== ANE Inference: {} ({} layers, GQA {}/{}, KV-cache) ===",
        cfg.name, cfg.n_layers, cfg.heads, cfg.kv_heads
    );
    eprintln!(
        "dim={} q_dim={} kv_dim={} hd={} hidden={} seq={} vocab={}",
        cfg.dim, cfg.q_dim, cfg.kv_dim, cfg.hd, cfg.hidden, cfg.seq, cfg.vocab
    );

    // Parse args
    let args: Vec<String> = std::env::args().collect();
    let mut ckpt_path = "ane_qwen3_06b_dyn_ckpt.bin".to_string();
    let mut temperature = 0.0f32;
    let mut topk = 40usize;
    let mut maxlen = cfg.seq - 1;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--ckpt" => { i += 1; ckpt_path = args[i].clone(); }
            "--temp" => { i += 1; temperature = args[i].parse()?; }
            "--topk" => { i += 1; topk = args[i].parse()?; }
            "--maxlen" => { i += 1; maxlen = args[i].parse::<usize>()?.min(cfg.seq - 1); }
            _ => {}
        }
        i += 1;
    }
    eprintln!("temp={:.2} topk={} maxlen={}", temperature, topk, maxlen);

    // Load checkpoint
    eprintln!("Loading: {}", ckpt_path);
    let t0 = Instant::now();
    let (layers, rms_final, embed_weights, hdr) = weights::load_checkpoint(&ckpt_path, &cfg)?;
    eprintln!("Loaded in {}ms (step {}, loss {:.4})", t0.elapsed().as_millis(), hdr.step, hdr.loss);

    // Transpose weights for ANE kernels
    eprintln!("Transposing weights...");
    let mut wqt = Vec::with_capacity(cfg.n_layers);
    let mut wkt = Vec::with_capacity(cfg.n_layers);
    let mut wvt = Vec::with_capacity(cfg.n_layers);
    let mut wot = Vec::with_capacity(cfg.n_layers);
    let mut w1t = Vec::with_capacity(cfg.n_layers);
    let mut w3t = Vec::with_capacity(cfg.n_layers);
    for l in 0..cfg.n_layers {
        let mut tq = vec![0.0f32; cfg.wq_size()];
        weights::transpose(&mut tq, &layers[l].wq, cfg.q_dim, cfg.dim);
        wqt.push(tq);
        let mut tk = vec![0.0f32; cfg.wk_size()];
        weights::transpose(&mut tk, &layers[l].wk, cfg.kv_dim, cfg.dim);
        wkt.push(tk);
        let mut tv = vec![0.0f32; cfg.wv_size()];
        weights::transpose(&mut tv, &layers[l].wv, cfg.kv_dim, cfg.dim);
        wvt.push(tv);
        let mut to = vec![0.0f32; cfg.wo_size()];
        weights::transpose(&mut to, &layers[l].wo, cfg.dim, cfg.q_dim);
        wot.push(to);
        let mut t1 = vec![0.0f32; cfg.w1_size()];
        weights::transpose(&mut t1, &layers[l].w1, cfg.hidden, cfg.dim);
        w1t.push(t1);
        let mut t3 = vec![0.0f32; cfg.w3_size()];
        weights::transpose(&mut t3, &layers[l].w3, cfg.hidden, cfg.dim);
        w3t.push(t3);
    }

    // Compile ANE kernels
    eprintln!("Compiling 3 ANE kernels...");
    let t0 = Instant::now();
    let qkv_program = projection::qkv_proj(&cfg);
    let wo_program = projection::wo_fwd(&cfg);
    let ffn_program = ffn::ffn_fused(&cfg, 1.0);
    let mut qkv_model = AneModel::compile(&qkv_program, &[])?;
    let mut wo_model = AneModel::compile(&wo_program, &[])?;
    let mut ffn_model = AneModel::compile(&ffn_program, &[])?;
    qkv_model.load()?;
    wo_model.load()?;
    ffn_model.load()?;
    eprintln!("Compiled in {}ms", t0.elapsed().as_millis());

    // Per-layer IOSurfaces
    let mut qkv_ins: Vec<AneSurface> = Vec::new();
    let mut wo_ins: Vec<AneSurface> = Vec::new();
    let mut ffn_ins: Vec<AneSurface> = Vec::new();
    let qkv_out = AneSurface::new(qkv_program.output_bytes())?;
    let wo_out = AneSurface::new(wo_program.output_bytes())?;
    let ffn_out = AneSurface::new(ffn_program.output_bytes())?;
    for l in 0..cfg.n_layers {
        let qkv_in = AneSurface::new(qkv_program.input_bytes())?;
        let wo_in = AneSurface::new(wo_program.input_bytes())?;
        let ffn_in = AneSurface::new(ffn_program.input_bytes())?;
        staging::stage_qkv_weights(&qkv_in, &wqt[l], &wkt[l], &wvt[l], &cfg);
        staging::stage_wo_weights(&wo_in, &wot[l], &cfg);
        staging::stage_ffn_weights(&ffn_in, &w1t[l], &w3t[l], &layers[l].w2, &cfg);
        qkv_ins.push(qkv_in);
        wo_ins.push(wo_in);
        ffn_ins.push(ffn_in);
    }

    // Fused weight matrices for decode
    eprintln!("Fusing weight matrices...");
    let fqkv_dim = cfg.q_dim + cfg.kv_dim + cfg.kv_dim;
    let fw13_rows = 2 * cfg.hidden;
    let mut fused_qkv: Vec<Vec<f32>> = Vec::with_capacity(cfg.n_layers);
    let mut fused_w13: Vec<Vec<f32>> = Vec::with_capacity(cfg.n_layers);
    for l in 0..cfg.n_layers {
        let mut qkv = vec![0.0f32; fqkv_dim * cfg.dim];
        qkv[..cfg.q_dim * cfg.dim].copy_from_slice(&layers[l].wq);
        qkv[cfg.q_dim * cfg.dim..(cfg.q_dim + cfg.kv_dim) * cfg.dim].copy_from_slice(&layers[l].wk);
        qkv[(cfg.q_dim + cfg.kv_dim) * cfg.dim..fqkv_dim * cfg.dim].copy_from_slice(&layers[l].wv);
        fused_qkv.push(qkv);
        let mut w13 = vec![0.0f32; fw13_rows * cfg.dim];
        w13[..cfg.hidden * cfg.dim].copy_from_slice(&layers[l].w1);
        w13[cfg.hidden * cfg.dim..fw13_rows * cfg.dim].copy_from_slice(&layers[l].w3);
        fused_w13.push(w13);
    }

    // Pre-compute RoPE cos/sin table
    let half_hd = cfg.hd / 2;
    let mut rope_cos = vec![0.0f32; cfg.seq * half_hd];
    let mut rope_sin = vec![0.0f32; cfg.seq * half_hd];
    for p in 0..cfg.seq {
        for i in 0..half_hd {
            let theta = p as f32 / 10000.0f32.powf(2.0 * i as f32 / cfg.hd as f32);
            rope_cos[p * half_hd + i] = theta.cos();
            rope_sin[p * half_hd + i] = theta.sin();
        }
    }

    // KV cache & activation buffers
    let mut kvc: Vec<KVCache> = (0..cfg.n_layers).map(|_| KVCache::alloc(&cfg)).collect();
    let (seq, dim, q_dim, kv_dim) = (cfg.seq, cfg.dim, cfg.q_dim, cfg.kv_dim);
    let mut x_cur = vec![0.0f32; seq * dim];
    let mut xnorm = vec![0.0f32; seq * dim];
    let mut q_buf = vec![0.0f32; seq * q_dim];
    let mut k_buf = vec![0.0f32; seq * kv_dim];
    let mut v_buf = vec![0.0f32; seq * kv_dim];
    let mut attn_out = vec![0.0f32; seq * q_dim];
    let mut o_out = vec![0.0f32; seq * dim];
    let mut x2 = vec![0.0f32; seq * dim];
    let mut x2norm = vec![0.0f32; seq * dim];
    let mut x_final = vec![0.0f32; seq * dim];
    let mut logits = vec![0.0f32; cfg.vocab];

    eprintln!("Ready. Send token IDs via stdin (one per line, empty line to generate).");

    // Read prompt tokens
    let mut tokens = vec![0u16; seq];
    let mut prompt_len = 0;
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.is_empty() || prompt_len >= seq { break; }
        tokens[prompt_len] = line.trim().parse()?;
        prompt_len += 1;
    }
    if prompt_len == 0 { eprintln!("No prompt tokens."); return Ok(()); }
    eprintln!("Prompt: {} tokens", prompt_len);

    // ===== Prefill =====
    let tpre = Instant::now();
    embed::lookup(&mut x_cur, &embed_weights, &tokens, dim, seq);
    for l in 0..cfg.n_layers {
        rmsnorm::forward(&mut xnorm, &x_cur, &layers[l].rms_att, dim, seq);
        staging::write_qkv_acts(&qkv_ins[l], &xnorm, &cfg);
        qkv_model.run(&qkv_ins[l], &qkv_out)?;
        staging::read_qkv_output(&qkv_out, &mut q_buf, &mut k_buf, &mut v_buf, &cfg);
        rmsnorm::qk_norm(&mut q_buf, &layers[l].q_norm, q_dim, cfg.hd, seq);
        rmsnorm::qk_norm(&mut k_buf, &layers[l].k_norm, kv_dim, cfg.hd, seq);
        rope::forward(&mut q_buf, seq, q_dim, cfg.hd);
        rope::forward(&mut k_buf, seq, kv_dim, cfg.hd);
        kvc[l].k_cache[..kv_dim * seq].copy_from_slice(&k_buf[..kv_dim * seq]);
        kvc[l].v_cache[..kv_dim * seq].copy_from_slice(&v_buf[..kv_dim * seq]);
        attention::cpu_attention(&mut attn_out, &q_buf, &k_buf, &v_buf, cfg.heads, cfg.kv_heads, cfg.hd, seq);
        staging::write_wo_acts(&wo_ins[l], &attn_out, &cfg);
        wo_model.run(&wo_ins[l], &wo_out)?;
        staging::read_surface_f32(&wo_out, &mut o_out, dim * seq);
        unsafe {
            ane::accel::vDSP_vadd(x_cur.as_ptr(), 1, o_out.as_ptr(), 1, x2.as_mut_ptr(), 1, (seq * dim) as u64);
        }
        rmsnorm::forward(&mut x2norm, &x2, &layers[l].rms_ffn, dim, seq);
        staging::write_ffn_acts(&ffn_ins[l], &x2norm, &x2, &cfg);
        ffn_model.run(&ffn_ins[l], &ffn_out)?;
        staging::read_surface_f32(&ffn_out, &mut x_cur, dim * seq);
    }
    rmsnorm::forward(&mut x_final, &x_cur, &rms_final, dim, seq);

    // Extract logits at last prompt position
    let pos = prompt_len - 1;
    let mut x_col = vec![0.0f32; dim];
    for d in 0..dim { x_col[d] = x_final[d * seq + pos]; }
    ane::accel::sgemv(false, cfg.vocab, dim, 1.0, &embed_weights, dim, &x_col, 0.0, &mut logits);
    let mut next_token = sample::sample_token(&logits, cfg.vocab, temperature, topk);

    let prefill_ms = tpre.elapsed().as_millis();
    eprintln!("Prefill: {}ms", prefill_ms);
    println!("{}", next_token);
    eprintln!("  [0] token={}  (prefill {}ms)", next_token, prefill_ms);
    if next_token == 151643 || next_token == 151645 { return Ok(()); }
    tokens[prompt_len] = next_token as u16;

    // ===== Decode with KV-cache =====
    let gen_limit = maxlen.min(seq - prompt_len);
    let mut total_decode_ms = 0u128;
    let mut generated = 1usize;
    let mut decode_bufs = DecodeBufs::alloc(&cfg);
    let mut x_single = vec![0.0f32; dim];

    for g in 1..gen_limit {
        let pos = prompt_len + g - 1;
        let td = Instant::now();
        embed::lookup_single(&mut x_single, &embed_weights, tokens[pos] as usize, dim);
        forward_decode(&mut x_single, &mut logits, pos, &layers, &rms_final, &embed_weights,
            &mut kvc, &cfg, &mut decode_bufs, &rope_cos, &rope_sin);
        next_token = sample::sample_token(&logits, cfg.vocab, temperature, topk);
        let dec_ms = td.elapsed().as_millis();
        total_decode_ms += dec_ms;
        generated += 1;
        println!("{}", next_token);
        eprintln!("  [{}] token={}  {}ms", g, next_token, dec_ms);
        if next_token == 151643 || next_token == 151645 { break; }
        if prompt_len + g < seq { tokens[prompt_len + g] = next_token as u16; } else { break; }
    }

    eprintln!("\n=== Inference Report ===");
    eprintln!("Prefill: {}ms ({} tokens)", prefill_ms, prompt_len);
    if generated > 1 {
        let avg = total_decode_ms as f64 / (generated - 1) as f64;
        eprintln!("Decode: {} tokens, {}ms total ({:.1}ms/tok, {:.1} tok/s)",
            generated - 1, total_decode_ms, avg, 1000.0 / avg);
    }
    Ok(())
}

// ── CPU decode pass ──

struct DecodeBufs {
    xnorm: Vec<f32>, q: Vec<f32>, k: Vec<f32>, v: Vec<f32>,
    a_out: Vec<f32>, o_out: Vec<f32>, x2: Vec<f32>, x2norm: Vec<f32>,
    h1: Vec<f32>, h3: Vec<f32>, gate: Vec<f32>, x_final: Vec<f32>,
}

impl DecodeBufs {
    fn alloc(cfg: &config::ModelConfig) -> Self {
        DecodeBufs {
            xnorm: vec![0.0; cfg.dim], q: vec![0.0; cfg.q_dim],
            k: vec![0.0; cfg.kv_dim], v: vec![0.0; cfg.kv_dim],
            a_out: vec![0.0; cfg.q_dim], o_out: vec![0.0; cfg.dim],
            x2: vec![0.0; cfg.dim], x2norm: vec![0.0; cfg.dim],
            h1: vec![0.0; cfg.hidden], h3: vec![0.0; cfg.hidden],
            gate: vec![0.0; cfg.hidden], x_final: vec![0.0; cfg.dim],
        }
    }
}

fn forward_decode(
    x: &mut [f32], logits: &mut [f32], pos: usize,
    layers: &[LayerWeights], rms_final: &[f32], embed_w: &[f32],
    kvc: &mut [KVCache], cfg: &config::ModelConfig,
    buf: &mut DecodeBufs, rope_cos: &[f32], rope_sin: &[f32],
) {
    let (dim, q_dim, kv_dim, hd, hidden) = (cfg.dim, cfg.q_dim, cfg.kv_dim, cfg.hd, cfg.hidden);
    let half_hd = hd / 2;
    for l in 0..cfg.n_layers {
        rmsnorm::forward_single(&mut buf.xnorm, x, &layers[l].rms_att, dim);
        matvec(&mut buf.q, &layers[l].wq, &buf.xnorm, q_dim, dim);
        matvec(&mut buf.k, &layers[l].wk, &buf.xnorm, kv_dim, dim);
        matvec(&mut buf.v, &layers[l].wv, &buf.xnorm, kv_dim, dim);
        rmsnorm::qk_norm_single(&mut buf.q, &layers[l].q_norm, q_dim, hd);
        rmsnorm::qk_norm_single(&mut buf.k, &layers[l].k_norm, kv_dim, hd);
        rope_apply_table(&mut buf.q, pos, q_dim, hd, half_hd, rope_cos, rope_sin);
        rope_apply_table(&mut buf.k, pos, kv_dim, hd, half_hd, rope_cos, rope_sin);
        unsafe {
            ane::accel::cblas_scopy(kv_dim as i32, buf.k.as_ptr(), 1,
                kvc[l].k_cache.as_mut_ptr().add(pos), cfg.seq as i32);
            ane::accel::cblas_scopy(kv_dim as i32, buf.v.as_ptr(), 1,
                kvc[l].v_cache.as_mut_ptr().add(pos), cfg.seq as i32);
        }
        attention::cpu_attention_cached(&mut buf.a_out, &buf.q, &kvc[l].k_cache,
            &kvc[l].v_cache, cfg.heads, cfg.kv_heads, hd, pos + 1, cfg.seq);
        matvec(&mut buf.o_out, &layers[l].wo, &buf.a_out, dim, q_dim);
        unsafe {
            ane::accel::vDSP_vadd(x.as_ptr(), 1, buf.o_out.as_ptr(), 1,
                buf.x2.as_mut_ptr(), 1, dim as u64);
        }
        rmsnorm::forward_single(&mut buf.x2norm, &buf.x2, &layers[l].rms_ffn, dim);
        matvec(&mut buf.h1, &layers[l].w1, &buf.x2norm, hidden, dim);
        matvec(&mut buf.h3, &layers[l].w3, &buf.x2norm, hidden, dim);
        unsafe {
            let h = hidden as u64;
            let n = hidden as i32;
            let neg1 = -1.0f32;
            ane::accel::vDSP_vsmul(buf.h1.as_ptr(), 1, &neg1, buf.gate.as_mut_ptr(), 1, h);
            ane::accel::vvexpf(buf.gate.as_mut_ptr(), buf.gate.as_ptr(), &n);
            let one = 1.0f32;
            ane::accel::vDSP_vsadd(buf.gate.as_ptr(), 1, &one, buf.gate.as_mut_ptr(), 1, h);
            extern "C" {
                fn vDSP_vdiv(A: *const f32, IA: i64, B: *const f32, IB: i64,
                    C: *mut f32, IC: i64, N: u64);
            }
            vDSP_vdiv(buf.gate.as_ptr(), 1, buf.h1.as_ptr(), 1, buf.gate.as_mut_ptr(), 1, h);
            ane::accel::vDSP_vmul(buf.gate.as_ptr(), 1, buf.h3.as_ptr(), 1,
                buf.gate.as_mut_ptr(), 1, h);
        }
        matvec(&mut buf.o_out, &layers[l].w2, &buf.gate, dim, hidden);
        unsafe {
            ane::accel::vDSP_vadd(buf.x2.as_ptr(), 1, buf.o_out.as_ptr(), 1,
                x.as_mut_ptr(), 1, dim as u64);
        }
    }
    rmsnorm::forward_single(&mut buf.x_final, x, rms_final, dim);
    ane::accel::sgemv(false, cfg.vocab, dim, 1.0, embed_w, dim, &buf.x_final, 0.0, logits);
}

fn rope_apply_table(x: &mut [f32], pos: usize, dim: usize, hd: usize,
    half_hd: usize, cos_table: &[f32], sin_table: &[f32]) {
    let nheads = dim / hd;
    let cos_row = &cos_table[pos * half_hd..(pos + 1) * half_hd];
    let sin_row = &sin_table[pos * half_hd..(pos + 1) * half_hd];
    for h in 0..nheads {
        for i in 0..half_hd {
            let (idx0, idx1) = (h * hd + 2 * i, h * hd + 2 * i + 1);
            let (v0, v1) = (x[idx0], x[idx1]);
            x[idx0] = v0 * cos_row[i] - v1 * sin_row[i];
            x[idx1] = v0 * sin_row[i] + v1 * cos_row[i];
        }
    }
}

fn matvec(out: &mut [f32], w: &[f32], x: &[f32], rows: usize, cols: usize) {
    ane::accel::sgemv(false, rows, cols, 1.0, w, cols, x, 0.0, out);
}
