//! Convert HuggingFace safetensors → ANE checkpoint
//! Usage: cargo run --release --bin convert_hf -- [--model Qwen/Qwen3-0.6B] [--output ckpt.bin]
//!
//! Downloads safetensors from HuggingFace, converts bf16→f32, writes ANE checkpoint.

use ane::config;
use safetensors::SafeTensors;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let mut model_id = "Qwen/Qwen3-0.6B".to_string();
    let mut output = "ane_qwen3_06b_dyn_ckpt.bin".to_string();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--model" => {
                i += 1;
                model_id = args[i].clone();
            }
            "--output" => {
                i += 1;
                output = args[i].clone();
            }
            _ => {}
        }
        i += 1;
    }

    let cfg = config::qwen3_06b();
    eprintln!("Converting {} → {}", model_id, output);
    eprintln!(
        "Config: dim={} q_dim={} kv_dim={} hidden={} heads={} kv_heads={} hd={} layers={} vocab={}",
        cfg.dim,
        cfg.q_dim,
        cfg.kv_dim,
        cfg.hidden,
        cfg.heads,
        cfg.kv_heads,
        cfg.hd,
        cfg.n_layers,
        cfg.vocab
    );

    // Download safetensors
    eprintln!("Downloading model...");
    let cache_dir = dirs_or_default();
    let st_path = download_safetensors(&model_id, &cache_dir)?;
    eprintln!("Safetensors: {}", st_path);

    let st_bytes = std::fs::read(&st_path)?;
    let st = SafeTensors::deserialize(&st_bytes)?;
    eprintln!("Loaded {} tensors", st.names().len());

    let mut f = std::fs::File::create(&output)?;

    // Write CkptHdr (88 bytes)
    let header = CkptHeader {
        magic: 0x424C5A54,
        version: 5,
        step: 0,
        total_steps: 0,
        n_layers: cfg.n_layers as i32,
        vocab_size: cfg.vocab as i32,
        dim: cfg.dim as i32,
        hidden_dim: cfg.hidden as i32,
        n_heads: cfg.heads as i32,
        seq_len: cfg.seq as i32,
        lr: 0.0,
        loss: 0.0,
        cum_compile: 0.0,
        cum_train: 0.0,
        cum_wall: 0.0,
        cum_steps: 0,
        cum_batches: 0,
        adam_t: 0,
        kv_heads: cfg.kv_heads as i32,
        head_dim: cfg.hd as i32,
        q_dim: cfg.q_dim as i32,
    };
    let hdr_bytes = unsafe {
        std::slice::from_raw_parts(
            &header as *const CkptHeader as *const u8,
            std::mem::size_of::<CkptHeader>(),
        )
    };
    f.write_all(hdr_bytes)?;

    // Per-layer
    let adam_sizes = [
        cfg.wq_size(),
        cfg.wk_size(),
        cfg.wv_size(),
        cfg.wo_size(),
        cfg.w1_size(),
        cfg.w2_size(),
        cfg.w3_size(),
        cfg.dim,
        cfg.dim,
        cfg.hd,
        cfg.hd,
    ];

    for l in 0..cfg.n_layers {
        let pfx = format!("model.layers.{l}");
        write_tensor(&mut f, &st, &format!("{pfx}.self_attn.q_proj.weight"))?;
        write_tensor(&mut f, &st, &format!("{pfx}.self_attn.k_proj.weight"))?;
        write_tensor(&mut f, &st, &format!("{pfx}.self_attn.v_proj.weight"))?;
        write_tensor(&mut f, &st, &format!("{pfx}.self_attn.o_proj.weight"))?;
        write_tensor(&mut f, &st, &format!("{pfx}.mlp.gate_proj.weight"))?;
        write_tensor(&mut f, &st, &format!("{pfx}.mlp.down_proj.weight"))?;
        write_tensor(&mut f, &st, &format!("{pfx}.mlp.up_proj.weight"))?;
        write_tensor(&mut f, &st, &format!("{pfx}.input_layernorm.weight"))?;
        write_tensor(
            &mut f,
            &st,
            &format!("{pfx}.post_attention_layernorm.weight"),
        )?;
        write_tensor(&mut f, &st, &format!("{pfx}.self_attn.q_norm.weight"))?;
        write_tensor(&mut f, &st, &format!("{pfx}.self_attn.k_norm.weight"))?;

        // Zero Adam state (m, v for each weight)
        for &sz in &adam_sizes {
            let zeros = vec![0u8; sz * 4 * 2]; // m + v
            f.write_all(&zeros)?;
        }
        if l % 7 == 0 {
            eprintln!("  Layer {l}/{}", cfg.n_layers);
        }
    }

    // rms_final + Adam
    write_tensor(&mut f, &st, "model.norm.weight")?;
    f.write_all(&vec![0u8; cfg.dim * 4 * 2])?;

    // Embedding + Adam
    write_tensor(&mut f, &st, "model.embed_tokens.weight")?;
    f.write_all(&vec![0u8; cfg.vocab * cfg.dim * 4 * 2])?;

    let size = std::fs::metadata(&output)?.len();
    eprintln!("\nDone! {}: {:.2} GB", output, size as f64 / 1e9);
    Ok(())
}

/// Convert tensor from safetensors (bf16/f16/f32) → f32 bytes, write to file
fn write_tensor(
    f: &mut std::fs::File,
    st: &SafeTensors,
    name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let view = st.tensor(name).map_err(|e| format!("{}: {}", name, e))?;
    let data = view.data();
    let dtype = view.dtype();

    match dtype {
        safetensors::Dtype::BF16 => {
            let n = data.len() / 2;
            let mut f32_buf = vec![0.0f32; n];
            for i in 0..n {
                let bits = u16::from_le_bytes([data[2 * i], data[2 * i + 1]]);
                f32_buf[i] = bf16_to_f32(bits);
            }
            let bytes = unsafe { std::slice::from_raw_parts(f32_buf.as_ptr() as *const u8, n * 4) };
            f.write_all(bytes)?;
        }
        safetensors::Dtype::F16 => {
            let n = data.len() / 2;
            let mut f32_buf = vec![0.0f32; n];
            for i in 0..n {
                let bits = u16::from_le_bytes([data[2 * i], data[2 * i + 1]]);
                f32_buf[i] = ane::surface::fp16_to_f32(bits);
            }
            let bytes = unsafe { std::slice::from_raw_parts(f32_buf.as_ptr() as *const u8, n * 4) };
            f.write_all(bytes)?;
        }
        safetensors::Dtype::F32 => {
            f.write_all(data)?;
        }
        _ => return Err(format!("{}: unsupported dtype {:?}", name, dtype).into()),
    }
    Ok(())
}

fn bf16_to_f32(bits: u16) -> f32 {
    f32::from_bits((bits as u32) << 16)
}

#[repr(C)]
struct CkptHeader {
    magic: u32,
    version: u32,
    step: i32,
    total_steps: i32,
    n_layers: i32,
    vocab_size: i32,
    dim: i32,
    hidden_dim: i32,
    n_heads: i32,
    seq_len: i32,
    lr: f32,
    loss: f32,
    cum_compile: f64,
    cum_train: f64,
    cum_wall: f64,
    cum_steps: i32,
    cum_batches: i32,
    adam_t: i32,
    kv_heads: i32,
    head_dim: i32,
    q_dim: i32,
}

fn dirs_or_default() -> String {
    std::env::var("HF_HOME").unwrap_or_else(|_| {
        format!(
            "{}/.cache/huggingface",
            std::env::var("HOME").unwrap_or_default()
        )
    })
}

fn download_safetensors(
    model_id: &str,
    _cache_dir: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // Check if already cached
    let cache_model = format!(
        "{}/.cache/huggingface/hub/models--{}",
        std::env::var("HOME").unwrap_or_default(),
        model_id.replace('/', "--")
    );

    // Look for safetensors in cache
    if let Ok(entries) = std::fs::read_dir(format!("{}/snapshots", cache_model)) {
        for entry in entries.flatten() {
            let snap_dir = entry.path();
            if let Ok(files) = std::fs::read_dir(&snap_dir) {
                for f in files.flatten() {
                    let name = f.file_name().to_string_lossy().to_string();
                    if name.ends_with(".safetensors") {
                        return Ok(f.path().to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    // Download via HuggingFace API
    eprintln!("Downloading from HuggingFace API...");
    let url = format!(
        "https://huggingface.co/{}/resolve/main/model.safetensors",
        model_id
    );
    let out_path = format!("{}.safetensors", model_id.replace('/', "_"));

    let resp = ureq::get(&url).call()?;
    let mut reader = resp.into_body().into_reader();
    let mut out = std::fs::File::create(&out_path)?;
    std::io::copy(&mut reader, &mut out)?;

    Ok(out_path)
}
