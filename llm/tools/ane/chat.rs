//! ANE chat — tokenize -> infer -> detokenize, all in Rust
//!
//! Usage: cargo run --release --bin chat -- --ckpt PATH "Once upon a time"
//!        echo "Tell me a story" | cargo run --release --bin chat -- --ckpt PATH --stdin
//!
//! Uses HuggingFace tokenizers crate (pure Rust) for Qwen3 tokenization.
//! Downloads tokenizer.json from HuggingFace on first run.

use ane::config;
use ane::mil::{ffn, projection};
use ane::ops::{attention, embed, rmsnorm, rope, sample};
use ane::staging;
use ane::surface::{cvt_f16_f32, AneSurface};
use ane::weights;
use ane::AneModel;
use std::io::{self, Write};
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = config::qwen3_06b();
    let args: Vec<String> = std::env::args().collect();
    let mut ckpt_path = "ane_qwen3_06b_dyn_ckpt.bin".to_string();
    let mut temperature = 0.8f32;
    let mut topk = 40usize;
    let mut maxlen = cfg.seq - 1;
    let mut from_stdin = false;
    let mut prompt_parts: Vec<String> = Vec::new();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--ckpt" => { i += 1; ckpt_path = args[i].clone(); }
            "--temp" => { i += 1; temperature = args[i].parse()?; }
            "--topk" => { i += 1; topk = args[i].parse()?; }
            "--maxlen" => { i += 1; maxlen = args[i].parse::<usize>()?.min(cfg.seq - 1); }
            "--stdin" => { from_stdin = true; }
            s if !s.starts_with('-') => { prompt_parts.push(s.to_string()); }
            _ => {}
        }
        i += 1;
    }

    let prompt = if from_stdin {
        let mut s = String::new();
        io::stdin().read_line(&mut s)?;
        s.trim().to_string()
    } else if !prompt_parts.is_empty() {
        prompt_parts.join(" ")
    } else {
        eprint!("Prompt: ");
        io::stderr().flush()?;
        let mut s = String::new();
        io::stdin().read_line(&mut s)?;
        s.trim().to_string()
    };
    if prompt.is_empty() { eprintln!("Empty prompt"); return Ok(()); }

    // Load tokenizer
    eprint!("Loading tokenizer...");
    let tokenizer = load_tokenizer()?;
    eprintln!(" OK");
    let encoding = tokenizer.encode(prompt.as_str(), false)
        .map_err(|e| format!("Tokenize: {}", e))?;
    let token_ids: Vec<u32> = encoding.get_ids().to_vec();
    let prompt_len = token_ids.len().min(cfg.seq);
    eprintln!("Prompt: {} ({} tokens)", prompt, prompt_len);

    // Load model
    eprintln!("Loading checkpoint: {}", ckpt_path);
    let t0 = Instant::now();
    let (layers, rms_final, embed_w, hdr) = weights::load_checkpoint(&ckpt_path, &cfg)?;
    eprintln!("Loaded in {}ms (step {}, loss {:.4})", t0.elapsed().as_millis(), hdr.step, hdr.loss);

    // Transpose weights
    let mut wqt = Vec::new(); let mut wkt = Vec::new(); let mut wvt = Vec::new();
    let mut wot = Vec::new(); let mut w1t = Vec::new(); let mut w3t = Vec::new();
    for l in 0..cfg.n_layers {
        let mut t = vec![0.0f32; cfg.wq_size()];
        weights::transpose(&mut t, &layers[l].wq, cfg.q_dim, cfg.dim); wqt.push(t);
        let mut t = vec![0.0f32; cfg.wk_size()];
        weights::transpose(&mut t, &layers[l].wk, cfg.kv_dim, cfg.dim); wkt.push(t);
        let mut t = vec![0.0f32; cfg.wv_size()];
        weights::transpose(&mut t, &layers[l].wv, cfg.kv_dim, cfg.dim); wvt.push(t);
        let mut t = vec![0.0f32; cfg.wo_size()];
        weights::transpose(&mut t, &layers[l].wo, cfg.dim, cfg.q_dim); wot.push(t);
        let mut t = vec![0.0f32; cfg.w1_size()];
        weights::transpose(&mut t, &layers[l].w1, cfg.hidden, cfg.dim); w1t.push(t);
        let mut t = vec![0.0f32; cfg.w3_size()];
        weights::transpose(&mut t, &layers[l].w3, cfg.hidden, cfg.dim); w3t.push(t);
    }

    // Compile ANE kernels
    eprint!("Compiling ANE kernels...");
    let t0 = Instant::now();
    let qkv_prog = projection::qkv_proj(&cfg);
    let wo_prog = projection::wo_fwd(&cfg);
    let ffn_prog = ffn::ffn_fused(&cfg, 1.0);
    let mut qkv_model = AneModel::compile(&qkv_prog, &[])?;
    qkv_model.load()?;
    let mut wo_model = AneModel::compile(&wo_prog, &[])?;
    wo_model.load()?;
    let mut ffn_model = AneModel::compile(&ffn_prog, &[])?;
    ffn_model.load()?;
    eprintln!(" {}ms", t0.elapsed().as_millis());

    // Allocate surfaces and stage weights
    let mut qkv_ins = Vec::new();
    let mut wo_ins = Vec::new();
    let mut ffn_ins = Vec::new();
    let qkv_out = AneSurface::new(qkv_prog.output_bytes())?;
    let wo_out = AneSurface::new(wo_prog.output_bytes())?;
    let ffn_out = AneSurface::new(ffn_prog.output_bytes())?;
    for l in 0..cfg.n_layers {
        let qi = AneSurface::new(qkv_prog.input_bytes())?;
        let wi = AneSurface::new(wo_prog.input_bytes())?;
        let fi = AneSurface::new(ffn_prog.input_bytes())?;
        staging::stage_qkv_weights(&qi, &wqt[l], &wkt[l], &wvt[l], &cfg);
        staging::stage_wo_weights(&wi, &wot[l], &cfg);
        staging::stage_ffn_weights(&fi, &w1t[l], &w3t[l], &layers[l].w2, &cfg);
        qkv_ins.push(qi); wo_ins.push(wi); ffn_ins.push(fi);
    }
    drop((wqt, wkt, wvt, wot, w1t, w3t));

    let mut kvc: Vec<weights::KVCache> = (0..cfg.n_layers)
        .map(|_| weights::KVCache::alloc(&cfg)).collect();

    // RoPE table
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

    // Prefill
    let mut tokens = vec![0u16; cfg.seq];
    for (i, &t) in token_ids.iter().enumerate().take(prompt_len) {
        tokens[i] = t as u16;
    }
    let tpre = Instant::now();
    let mut x = vec![0.0f32; cfg.seq * cfg.dim];
    embed::lookup(&mut x, &embed_w, &tokens, cfg.dim, cfg.seq);
    prefill(&mut x, &layers, &rms_final, &cfg, &mut kvc,
        &qkv_model, &wo_model, &ffn_model,
        &qkv_ins, &wo_ins, &ffn_ins, &qkv_out, &wo_out, &ffn_out);

    let mut x_col = vec![0.0f32; cfg.dim];
    let pos = prompt_len - 1;
    for d in 0..cfg.dim { x_col[d] = x[d * cfg.seq + pos]; }
    let mut logits = vec![0.0f32; cfg.vocab];
    ane::accel::sgemv(false, cfg.vocab, cfg.dim, 1.0, &embed_w, cfg.dim, &x_col, 0.0, &mut logits);
    let mut next = sample::sample_token(&logits, cfg.vocab, temperature, topk);
    eprintln!("Prefill: {}ms", tpre.elapsed().as_millis());

    print!("{}", prompt);
    io::stdout().flush()?;
    let mut generated = vec![next as u32];
    print_token(&tokenizer, &generated);
    if next == 151643 || next == 151645 { println!(); return Ok(()); }
    tokens[prompt_len] = next as u16;

    // Decode
    let mut x_single = vec![0.0f32; cfg.dim];
    for g in 1..maxlen.min(cfg.seq - prompt_len) {
        let pos = prompt_len + g - 1;
        embed::lookup_single(&mut x_single, &embed_w, tokens[pos] as usize, cfg.dim);
        decode_step(&mut x_single, &mut logits, pos, &layers, &rms_final, &embed_w,
            &mut kvc, &cfg, &rope_cos, &rope_sin);
        next = sample::sample_token(&logits, cfg.vocab, temperature, topk);
        generated.push(next as u32);
        print_token(&tokenizer, &generated);
        if next == 151643 || next == 151645 { break; }
        if prompt_len + g < cfg.seq { tokens[prompt_len + g] = next as u16; } else { break; }
    }
    println!();
    eprintln!("Generated {} tokens", generated.len());
    Ok(())
}

fn print_token(tokenizer: &tokenizers::Tokenizer, ids: &[u32]) {
    if let Ok(text) = tokenizer.decode(ids, true) {
        print!("\r{}", text);
        io::stdout().flush().ok();
    }
}

fn load_tokenizer() -> Result<tokenizers::Tokenizer, Box<dyn std::error::Error>> {
    let cache = format!("{}/.cache/huggingface/hub/models--Qwen--Qwen3-0.6B",
        std::env::var("HOME").unwrap_or_default());
    if let Ok(entries) = std::fs::read_dir(format!("{}/snapshots", cache)) {
        for e in entries.flatten() {
            let p = e.path().join("tokenizer.json");
            if p.exists() {
                return tokenizers::Tokenizer::from_file(&p).map_err(|e| e.to_string().into());
            }
        }
    }
    eprintln!(" downloading tokenizer.json...");
    let url = "https://huggingface.co/Qwen/Qwen3-0.6B/resolve/main/tokenizer.json";
    let resp = ureq::get(url).call()?;
    let body = resp.into_body().read_to_string()?;
    std::fs::write("tokenizer.json", &body)?;
    tokenizers::Tokenizer::from_file("tokenizer.json").map_err(|e| e.to_string().into())
}

// ── Prefill (ANE) ──

fn prefill(
    x: &mut [f32], layers: &[weights::LayerWeights], rms_final: &[f32],
    cfg: &config::ModelConfig, kvc: &mut [weights::KVCache],
    qkv_m: &AneModel, wo_m: &AneModel, ffn_m: &AneModel,
    qkv_ins: &[AneSurface], wo_ins: &[AneSurface], ffn_ins: &[AneSurface],
    qkv_out: &AneSurface, wo_out: &AneSurface, ffn_out: &AneSurface,
) {
    let (dim, seq, q_dim, kv_dim) = (cfg.dim, cfg.seq, cfg.q_dim, cfg.kv_dim);
    let mut xnorm = vec![0.0f32; seq * dim];
    let mut q = vec![0.0f32; seq * q_dim];
    let mut k = vec![0.0f32; seq * kv_dim];
    let mut v = vec![0.0f32; seq * kv_dim];
    let mut attn = vec![0.0f32; seq * q_dim];
    let mut o = vec![0.0f32; seq * dim];
    let mut x2 = vec![0.0f32; seq * dim];
    let mut x2n = vec![0.0f32; seq * dim];

    for l in 0..cfg.n_layers {
        rmsnorm::forward(&mut xnorm, x, &layers[l].rms_att, dim, seq);
        staging::write_qkv_acts(&qkv_ins[l], &xnorm, cfg);
        qkv_m.run(&qkv_ins[l], qkv_out).unwrap();
        staging::read_qkv_output(qkv_out, &mut q, &mut k, &mut v, cfg);
        rmsnorm::qk_norm(&mut q, &layers[l].q_norm, q_dim, cfg.hd, seq);
        rmsnorm::qk_norm(&mut k, &layers[l].k_norm, kv_dim, cfg.hd, seq);
        rope::forward(&mut q, seq, q_dim, cfg.hd);
        rope::forward(&mut k, seq, kv_dim, cfg.hd);
        kvc[l].k_cache[..kv_dim * seq].copy_from_slice(&k[..kv_dim * seq]);
        kvc[l].v_cache[..kv_dim * seq].copy_from_slice(&v[..kv_dim * seq]);
        attention::cpu_attention(&mut attn, &q, &k, &v, cfg.heads, cfg.kv_heads, cfg.hd, seq);
        staging::write_wo_acts(&wo_ins[l], &attn, cfg);
        wo_m.run(&wo_ins[l], wo_out).unwrap();
        wo_out.with_data(|buf| cvt_f16_f32(&mut o, &buf[..dim * seq]));
        unsafe {
            ane::accel::vDSP_vadd(x.as_ptr(), 1, o.as_ptr(), 1,
                x2.as_mut_ptr(), 1, (seq * dim) as u64);
        }
        rmsnorm::forward(&mut x2n, &x2, &layers[l].rms_ffn, dim, seq);
        staging::write_ffn_acts(&ffn_ins[l], &x2n, &x2, cfg);
        ffn_m.run(&ffn_ins[l], ffn_out).unwrap();
        ffn_out.with_data(|buf| cvt_f16_f32(x, &buf[..dim * seq]));
    }
    let mut xf = vec![0.0f32; seq * dim];
    rmsnorm::forward(&mut xf, x, rms_final, dim, seq);
    x.copy_from_slice(&xf);
}

fn decode_step(
    x: &mut [f32], logits: &mut [f32], pos: usize,
    layers: &[weights::LayerWeights], rms_final: &[f32], embed_w: &[f32],
    kvc: &mut [weights::KVCache], cfg: &config::ModelConfig,
    rope_cos: &[f32], rope_sin: &[f32],
) {
    let (dim, q_dim, kv_dim, hd, hidden) = (cfg.dim, cfg.q_dim, cfg.kv_dim, cfg.hd, cfg.hidden);
    let half_hd = hd / 2;
    let mut xn = vec![0.0f32; dim];
    let mut q = vec![0.0f32; q_dim];
    let mut k = vec![0.0f32; kv_dim];
    let mut v = vec![0.0f32; kv_dim];
    let mut ao = vec![0.0f32; q_dim];
    let mut oo = vec![0.0f32; dim];
    let mut x2 = vec![0.0f32; dim];
    let mut x2n = vec![0.0f32; dim];
    let mut h1 = vec![0.0f32; hidden];
    let mut h3 = vec![0.0f32; hidden];
    let mut gate = vec![0.0f32; hidden];

    for l in 0..cfg.n_layers {
        rmsnorm::forward_single(&mut xn, x, &layers[l].rms_att, dim);
        ane::accel::sgemv(false, q_dim, dim, 1.0, &layers[l].wq, dim, &xn, 0.0, &mut q);
        ane::accel::sgemv(false, kv_dim, dim, 1.0, &layers[l].wk, dim, &xn, 0.0, &mut k);
        ane::accel::sgemv(false, kv_dim, dim, 1.0, &layers[l].wv, dim, &xn, 0.0, &mut v);
        rmsnorm::qk_norm_single(&mut q, &layers[l].q_norm, q_dim, hd);
        rmsnorm::qk_norm_single(&mut k, &layers[l].k_norm, kv_dim, hd);
        apply_rope(&mut q, pos, q_dim, hd, half_hd, rope_cos, rope_sin);
        apply_rope(&mut k, pos, kv_dim, hd, half_hd, rope_cos, rope_sin);
        unsafe {
            ane::accel::cblas_scopy(kv_dim as i32, k.as_ptr(), 1,
                kvc[l].k_cache.as_mut_ptr().add(pos), cfg.seq as i32);
            ane::accel::cblas_scopy(kv_dim as i32, v.as_ptr(), 1,
                kvc[l].v_cache.as_mut_ptr().add(pos), cfg.seq as i32);
        }
        attention::cpu_attention_cached(&mut ao, &q, &kvc[l].k_cache, &kvc[l].v_cache,
            cfg.heads, cfg.kv_heads, hd, pos + 1, cfg.seq);
        ane::accel::sgemv(false, dim, q_dim, 1.0, &layers[l].wo, q_dim, &ao, 0.0, &mut oo);
        unsafe {
            ane::accel::vDSP_vadd(x.as_ptr(), 1, oo.as_ptr(), 1, x2.as_mut_ptr(), 1, dim as u64);
        }
        rmsnorm::forward_single(&mut x2n, &x2, &layers[l].rms_ffn, dim);
        ane::accel::sgemv(false, hidden, dim, 1.0, &layers[l].w1, dim, &x2n, 0.0, &mut h1);
        ane::accel::sgemv(false, hidden, dim, 1.0, &layers[l].w3, dim, &x2n, 0.0, &mut h3);
        unsafe {
            let h = hidden as u64;
            let n = hidden as i32;
            let neg1 = -1.0f32;
            ane::accel::vDSP_vsmul(h1.as_ptr(), 1, &neg1, gate.as_mut_ptr(), 1, h);
            ane::accel::vvexpf(gate.as_mut_ptr(), gate.as_ptr(), &n);
            let one = 1.0f32;
            ane::accel::vDSP_vsadd(gate.as_ptr(), 1, &one, gate.as_mut_ptr(), 1, h);
            extern "C" {
                fn vDSP_vdiv(A: *const f32, IA: i64, B: *const f32, IB: i64,
                    C: *mut f32, IC: i64, N: u64);
            }
            vDSP_vdiv(gate.as_ptr(), 1, h1.as_ptr(), 1, gate.as_mut_ptr(), 1, h);
            ane::accel::vDSP_vmul(gate.as_ptr(), 1, h3.as_ptr(), 1, gate.as_mut_ptr(), 1, h);
        }
        ane::accel::sgemv(false, dim, hidden, 1.0, &layers[l].w2, hidden, &gate, 0.0, &mut oo);
        unsafe {
            ane::accel::vDSP_vadd(x2.as_ptr(), 1, oo.as_ptr(), 1, x.as_mut_ptr(), 1, dim as u64);
        }
    }
    let mut xf = vec![0.0f32; dim];
    rmsnorm::forward_single(&mut xf, x, rms_final, dim);
    ane::accel::sgemv(false, cfg.vocab, dim, 1.0, embed_w, dim, &xf, 0.0, logits);
}

fn apply_rope(x: &mut [f32], pos: usize, dim: usize, hd: usize,
    half_hd: usize, cos_t: &[f32], sin_t: &[f32]) {
    let nheads = dim / hd;
    let c = &cos_t[pos * half_hd..(pos + 1) * half_hd];
    let s = &sin_t[pos * half_hd..(pos + 1) * half_hd];
    for h in 0..nheads {
        for i in 0..half_hd {
            let (i0, i1) = (h * hd + 2 * i, h * hd + 2 * i + 1);
            let (v0, v1) = (x[i0], x[i1]);
            x[i0] = v0 * c[i] - v1 * s[i];
            x[i1] = v0 * s[i] + v1 * c[i];
        }
    }
}
