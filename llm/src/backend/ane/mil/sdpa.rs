//! SDPA (Scaled Dot-Product Attention) MIL kernels
//! Full port of gen_sdpa_fwd_dynamic, gen_sdpa_bwd1_noweight, gen_sdpa_bwd2

use ane::{build_weight_blob, mil_footer, mil_header, MilProgram};
use crate::config::ModelConfig;
use ane::f32_to_fp16;

/// Generate causal mask blob: mask[t,t2] = 0 if t2<=t, -65504 otherwise
pub fn causal_mask_blob(seq: usize) -> Vec<u8> {
    let mut mask = vec![0u16; seq * seq];
    for t in 0..seq {
        for t2 in 0..seq {
            mask[t * seq + t2] = if t2 <= t { 0 } else { f32_to_fp16(-65504.0) };
        }
    }
    build_weight_blob(&mask)
}

/// Generate RoPE cos blob [SEQ, HD] — cos(pos * freq) duplicated for pairs
pub fn rope_cos_blob(seq: usize, hd: usize) -> Vec<u8> {
    let mut buf = vec![0u16; seq * hd];
    for p in 0..seq {
        for i in 0..hd / 2 {
            let theta = p as f32 / 10000.0f32.powf(2.0 * i as f32 / hd as f32);
            let cv = f32_to_fp16(theta.cos());
            buf[p * hd + 2 * i] = cv;
            buf[p * hd + 2 * i + 1] = cv;
        }
    }
    build_weight_blob(&buf)
}

/// Generate RoPE sin blob [SEQ, HD]
pub fn rope_sin_blob(seq: usize, hd: usize) -> Vec<u8> {
    let mut buf = vec![0u16; seq * hd];
    for p in 0..seq {
        for i in 0..hd / 2 {
            let theta = p as f32 / 10000.0f32.powf(2.0 * i as f32 / hd as f32);
            let sv = f32_to_fp16(theta.sin());
            buf[p * hd + 2 * i] = sv;
            buf[p * hd + 2 * i + 1] = sv;
        }
    }
    build_weight_blob(&buf)
}

/// Weight blobs needed for SDPA forward compilation
pub fn sdpa_fwd_weights(cfg: &ModelConfig) -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("@model_path/weights/mask.bin", causal_mask_blob(cfg.seq)),
        (
            "@model_path/weights/rope_cos.bin",
            rope_cos_blob(cfg.seq, cfg.hd),
        ),
        (
            "@model_path/weights/rope_sin.bin",
            rope_sin_blob(cfg.seq, cfg.hd),
        ),
    ]
}

/// Weight blobs for SDPA backward 1 (causal mask only)
pub fn sdpa_bwd1_weights(cfg: &ModelConfig) -> Vec<(&'static str, Vec<u8>)> {
    vec![("@model_path/weights/mask.bin", causal_mask_blob(cfg.seq))]
}

/// SDPA forward: QKV projection + RoPE + GQA tile + attention + softmax
/// Input: [1, DIM, 1, SDPA_FWD_SP]
/// Output: [1, Q_DIM+Q_DIM+KV_DIM+KV_DIM+DIM, 1, SEQ]
///   = concat(attn_out, Q_rope, K_rope, V, xnorm_pass)
pub fn sdpa_fwd(cfg: &ModelConfig) -> MilProgram {
    let dim = cfg.dim;
    let q_dim = cfg.q_dim;
    let kv_dim = cfg.kv_dim;
    let heads = cfg.heads;
    let kv_heads = cfg.kv_heads;
    let hd = cfg.hd;
    let seq = cfg.seq;
    let gqa = cfg.gqa_ratio();
    let sp = cfg.sdpa_fwd_sp();
    let out_ch = q_dim + q_dim + kv_dim + kv_dim + dim;
    let sc = 1.0 / (hd as f32).sqrt();
    let pairs_q = seq * hd / 2;
    let pairs_k = seq * hd / 2;

    let mut m = mil_header(dim, sp);

    // Slice xnorm, Wq, Wk, Wv from input
    m += "        tensor<int32, [4]> bx = const()[name=string(\"bx\"), val=tensor<int32, [4]>([0,0,0,0])];\n";
    m += &format!("        tensor<int32, [4]> sx = const()[name=string(\"sx\"), val=tensor<int32, [4]>([1,{dim},1,{seq}])];\n");
    m += &format!("        tensor<fp16, [1,{dim},1,{seq}]> xn = slice_by_size(x=x,begin=bx,size=sx)[name=string(\"xn\")];\n");

    m += &format!("        tensor<int32, [4]> bq = const()[name=string(\"bq\"), val=tensor<int32, [4]>([0,0,0,{seq}])];\n");
    m += &format!("        tensor<int32, [4]> swq = const()[name=string(\"swq\"), val=tensor<int32, [4]>([1,{dim},1,{q_dim}])];\n");
    m += &format!("        tensor<fp16, [1,{dim},1,{q_dim}]> Wq = slice_by_size(x=x,begin=bq,size=swq)[name=string(\"Wq\")];\n");

    m += &format!("        tensor<int32, [4]> bk = const()[name=string(\"bk\"), val=tensor<int32, [4]>([0,0,0,{}])];\n", seq + q_dim);
    m += &format!("        tensor<int32, [4]> swk = const()[name=string(\"swk\"), val=tensor<int32, [4]>([1,{dim},1,{kv_dim}])];\n");
    m += &format!("        tensor<fp16, [1,{dim},1,{kv_dim}]> Wk = slice_by_size(x=x,begin=bk,size=swk)[name=string(\"Wk\")];\n");

    m += &format!("        tensor<int32, [4]> bv = const()[name=string(\"bv\"), val=tensor<int32, [4]>([0,0,0,{}])];\n", seq + q_dim + kv_dim);
    m += &format!("        tensor<fp16, [1,{dim},1,{kv_dim}]> Wv = slice_by_size(x=x,begin=bv,size=swk)[name=string(\"Wv\")];\n");

    // Reshape xnorm for matmul
    m += &format!("        tensor<int32, [4]> r2 = const()[name=string(\"r2\"), val=tensor<int32, [4]>([1,1,{dim},{seq}])];\n");
    m += "        tensor<int32, [4]> pm = const()[name=string(\"pm\"), val=tensor<int32, [4]>([0,1,3,2])];\n";
    m += &format!("        tensor<fp16, [1,1,{dim},{seq}]> xn2 = reshape(shape=r2,x=xn)[name=string(\"xn2\")];\n");
    m += &format!("        tensor<fp16, [1,1,{seq},{dim}]> xnt = transpose(perm=pm,x=xn2)[name=string(\"xnt\")];\n");

    // Reshape weights
    m += &format!("        tensor<int32, [4]> rwq = const()[name=string(\"rwq\"), val=tensor<int32, [4]>([1,1,{dim},{q_dim}])];\n");
    m += &format!("        tensor<int32, [4]> rwk = const()[name=string(\"rwk\"), val=tensor<int32, [4]>([1,1,{dim},{kv_dim}])];\n");
    m += &format!("        tensor<fp16, [1,1,{dim},{q_dim}]> Wq2 = reshape(shape=rwq,x=Wq)[name=string(\"Wq2\")];\n");
    m += &format!("        tensor<fp16, [1,1,{dim},{kv_dim}]> Wk2 = reshape(shape=rwk,x=Wk)[name=string(\"Wk2\")];\n");
    m += &format!("        tensor<fp16, [1,1,{dim},{kv_dim}]> Wv2 = reshape(shape=rwk,x=Wv)[name=string(\"Wv2\")];\n");

    // QKV matmul
    m += "        bool bF = const()[name=string(\"bF\"), val=bool(false)];\n";
    m += "        bool bT = const()[name=string(\"bT\"), val=bool(true)];\n";
    m += &format!("        tensor<fp16, [1,1,{seq},{q_dim}]> qm = matmul(transpose_x=bF,transpose_y=bF,x=xnt,y=Wq2)[name=string(\"qm\")];\n");
    m += &format!("        tensor<fp16, [1,1,{seq},{kv_dim}]> km = matmul(transpose_x=bF,transpose_y=bF,x=xnt,y=Wk2)[name=string(\"km\")];\n");
    m += &format!("        tensor<fp16, [1,1,{seq},{kv_dim}]> vm = matmul(transpose_x=bF,transpose_y=bF,x=xnt,y=Wv2)[name=string(\"vm\")];\n");

    // Transpose back and reshape
    m += &format!("        tensor<fp16, [1,1,{q_dim},{seq}]> qt = transpose(perm=pm,x=qm)[name=string(\"qt\")];\n");
    m += &format!("        tensor<fp16, [1,1,{kv_dim},{seq}]> kt = transpose(perm=pm,x=km)[name=string(\"kt\")];\n");
    m += &format!("        tensor<fp16, [1,1,{kv_dim},{seq}]> vt = transpose(perm=pm,x=vm)[name=string(\"vt\")];\n");
    m += &format!("        tensor<int32, [4]> qsh = const()[name=string(\"qsh\"), val=tensor<int32, [4]>([1,{q_dim},1,{seq}])];\n");
    m += &format!("        tensor<int32, [4]> kvsh = const()[name=string(\"kvsh\"), val=tensor<int32, [4]>([1,{kv_dim},1,{seq}])];\n");
    m += &format!("        tensor<fp16, [1,{q_dim},1,{seq}]> qf = reshape(shape=qsh,x=qt)[name=string(\"qf\")];\n");
    m += &format!("        tensor<fp16, [1,{kv_dim},1,{seq}]> kf = reshape(shape=kvsh,x=kt)[name=string(\"kf\")];\n");
    m += &format!("        tensor<fp16, [1,{kv_dim},1,{seq}]> vf = reshape(shape=kvsh,x=vt)[name=string(\"vf\")];\n");

    // Reshape to heads
    m += &format!("        tensor<int32, [4]> qhsh = const()[name=string(\"qhsh\"), val=tensor<int32, [4]>([1,{heads},{hd},{seq}])];\n");
    m += &format!("        tensor<fp16, [1,{heads},{hd},{seq}]> q4 = reshape(shape=qhsh,x=qf)[name=string(\"rq\")];\n");
    m += &format!("        tensor<fp16, [1,{heads},{seq},{hd}]> q = transpose(perm=pm,x=q4)[name=string(\"tq\")];\n");
    m += &format!("        tensor<int32, [4]> khsh = const()[name=string(\"khsh\"), val=tensor<int32, [4]>([1,{kv_heads},{hd},{seq}])];\n");
    m += &format!("        tensor<fp16, [1,{kv_heads},{hd},{seq}]> k4 = reshape(shape=khsh,x=kf)[name=string(\"rk\")];\n");
    m += &format!("        tensor<fp16, [1,{kv_heads},{seq},{hd}]> k = transpose(perm=pm,x=k4)[name=string(\"tk\")];\n");
    m += &format!("        tensor<fp16, [1,{kv_heads},{hd},{seq}]> v4 = reshape(shape=khsh,x=vf)[name=string(\"rv\")];\n");
    m += &format!("        tensor<fp16, [1,{kv_heads},{seq},{hd}]> v = transpose(perm=pm,x=v4)[name=string(\"tv\")];\n");

    // RoPE on Q
    m += &format!("        tensor<fp16, [1,1,{seq},{hd}]> rope_cos = const()[name=string(\"rc\"), val=tensor<fp16, [1,1,{seq},{hd}]>(BLOBFILE(path=string(\"@model_path/weights/rope_cos.bin\"), offset=uint64(64)))];\n");
    m += &format!("        tensor<fp16, [1,1,{seq},{hd}]> rope_sin = const()[name=string(\"rs\"), val=tensor<fp16, [1,1,{seq},{hd}]>(BLOBFILE(path=string(\"@model_path/weights/rope_sin.bin\"), offset=uint64(64)))];\n");
    m += &format!("        tensor<int32, [4]> rp_sh = const()[name=string(\"rp_sh\"), val=tensor<int32, [4]>([1,{heads},{pairs_q},2])];\n");
    m += &format!("        tensor<int32, [4]> rp_s1 = const()[name=string(\"rp_s1\"), val=tensor<int32, [4]>([1,{heads},{pairs_q},1])];\n");
    m += "        tensor<int32, [4]> rp_b0 = const()[name=string(\"rp_b0\"), val=tensor<int32, [4]>([0,0,0,0])];\n";
    m += "        tensor<int32, [4]> rp_b1 = const()[name=string(\"rp_b1\"), val=tensor<int32, [4]>([0,0,0,1])];\n";
    m += "        fp16 neg1 = const()[name=string(\"neg1\"), val=fp16(-1)];\n";
    m += "        int32 rpax = const()[name=string(\"rpax\"), val=int32(3)];\n";
    m += "        bool rpil = const()[name=string(\"rpil\"), val=bool(false)];\n";
    m += &format!("        tensor<int32, [4]> rp_bk_q = const()[name=string(\"rp_bk_q\"), val=tensor<int32, [4]>([1,{heads},{seq},{hd}])];\n");

    // rotate_half(q)
    m += &format!("        tensor<fp16, [1,{heads},{pairs_q},2]> q_p = reshape(shape=rp_sh,x=q)[name=string(\"q_p\")];\n");
    m += &format!("        tensor<fp16, [1,{heads},{pairs_q},1]> q_e = slice_by_size(x=q_p,begin=rp_b0,size=rp_s1)[name=string(\"q_e\")];\n");
    m += &format!("        tensor<fp16, [1,{heads},{pairs_q},1]> q_o = slice_by_size(x=q_p,begin=rp_b1,size=rp_s1)[name=string(\"q_o\")];\n");
    m += &format!("        tensor<fp16, [1,{heads},{pairs_q},1]> nq = mul(x=q_o,y=neg1)[name=string(\"nq\")];\n");
    m += &format!("        tensor<fp16, [1,{heads},{pairs_q},2]> qrp = concat(axis=rpax,interleave=rpil,values=(nq,q_e))[name=string(\"qrp\")];\n");
    m += &format!("        tensor<fp16, [1,{heads},{seq},{hd}]> q_rot = reshape(shape=rp_bk_q,x=qrp)[name=string(\"q_rot\")];\n");
    m += &format!("        tensor<fp16, [1,{heads},{seq},{hd}]> qc = mul(x=q,y=rope_cos)[name=string(\"qc\")];\n");
    m += &format!("        tensor<fp16, [1,{heads},{seq},{hd}]> qrs = mul(x=q_rot,y=rope_sin)[name=string(\"qrs\")];\n");
    m += &format!("        tensor<fp16, [1,{heads},{seq},{hd}]> q_rope = add(x=qc,y=qrs)[name=string(\"q_rope\")];\n");

    // RoPE on K
    m += &format!("        tensor<int32, [4]> rp_sh_k = const()[name=string(\"rp_sh_k\"), val=tensor<int32, [4]>([1,{kv_heads},{pairs_k},2])];\n");
    m += &format!("        tensor<int32, [4]> rp_s1_k = const()[name=string(\"rp_s1_k\"), val=tensor<int32, [4]>([1,{kv_heads},{pairs_k},1])];\n");
    m += &format!("        tensor<int32, [4]> rp_bk_k = const()[name=string(\"rp_bk_k\"), val=tensor<int32, [4]>([1,{kv_heads},{seq},{hd}])];\n");
    m += &format!("        tensor<fp16, [1,{kv_heads},{pairs_k},2]> k_p = reshape(shape=rp_sh_k,x=k)[name=string(\"k_p\")];\n");
    m += &format!("        tensor<fp16, [1,{kv_heads},{pairs_k},1]> k_e = slice_by_size(x=k_p,begin=rp_b0,size=rp_s1_k)[name=string(\"k_e\")];\n");
    m += &format!("        tensor<fp16, [1,{kv_heads},{pairs_k},1]> k_o = slice_by_size(x=k_p,begin=rp_b1,size=rp_s1_k)[name=string(\"k_o\")];\n");
    m += &format!("        tensor<fp16, [1,{kv_heads},{pairs_k},1]> nk = mul(x=k_o,y=neg1)[name=string(\"nk\")];\n");
    m += &format!("        tensor<fp16, [1,{kv_heads},{pairs_k},2]> krp = concat(axis=rpax,interleave=rpil,values=(nk,k_e))[name=string(\"krp\")];\n");
    m += &format!("        tensor<fp16, [1,{kv_heads},{seq},{hd}]> k_rot = reshape(shape=rp_bk_k,x=krp)[name=string(\"k_rot\")];\n");
    m += &format!("        tensor<fp16, [1,{kv_heads},{seq},{hd}]> kc = mul(x=k,y=rope_cos)[name=string(\"kc\")];\n");
    m += &format!("        tensor<fp16, [1,{kv_heads},{seq},{hd}]> krs = mul(x=k_rot,y=rope_sin)[name=string(\"krs\")];\n");
    m += &format!("        tensor<fp16, [1,{kv_heads},{seq},{hd}]> k_rope = add(x=kc,y=krs)[name=string(\"k_rope\")];\n");

    // GQA: tile K,V from KV_HEADS to HEADS
    m += "        int32 cax = const()[name=string(\"cax\"), val=int32(1)];\n";
    m += "        bool cid = const()[name=string(\"cid\"), val=bool(false)];\n";
    let k_vals = (0..gqa).map(|_| "k_rope").collect::<Vec<_>>().join(",");
    let v_vals = (0..gqa).map(|_| "v").collect::<Vec<_>>().join(",");
    m += &format!("        tensor<fp16, [1,{heads},{seq},{hd}]> k_tiled = concat(axis=cax,interleave=cid,values=({k_vals}))[name=string(\"ktile\")];\n");
    m += &format!("        tensor<fp16, [1,{heads},{seq},{hd}]> v_tiled = concat(axis=cax,interleave=cid,values=({v_vals}))[name=string(\"vtile\")];\n");

    // Scaled attention: Q @ K^T
    m += &format!("        tensor<fp16, [1,{heads},{seq},{seq}]> sc1 = matmul(transpose_x=bF,transpose_y=bT,x=q_rope,y=k_tiled)[name=string(\"mm1\")];\n");
    m += &format!("        fp16 scv = const()[name=string(\"scv\"), val=fp16({sc})];\n");
    m += &format!("        tensor<fp16, [1,{heads},{seq},{seq}]> sc2 = mul(x=sc1,y=scv)[name=string(\"scl\")];\n");

    // Causal mask + softmax
    m += &format!("        tensor<fp16, [1,1,{seq},{seq}]> cm = const()[name=string(\"cm\"), val=tensor<fp16, [1,1,{seq},{seq}]>(BLOBFILE(path=string(\"@model_path/weights/mask.bin\"), offset=uint64(64)))];\n");
    m += &format!("        tensor<fp16, [1,{heads},{seq},{seq}]> ms = add(x=sc2,y=cm)[name=string(\"msk\")];\n");
    m += "        int32 sax = const()[name=string(\"sax\"), val=int32(-1)];\n";
    m += &format!("        tensor<fp16, [1,{heads},{seq},{seq}]> aw = softmax(axis=sax,x=ms)[name=string(\"sm\")];\n");

    // scores @ V_tiled
    m += &format!("        tensor<fp16, [1,{heads},{seq},{hd}]> a4 = matmul(transpose_x=bF,transpose_y=bF,x=aw,y=v_tiled)[name=string(\"mm2\")];\n");

    // Reshape attn_out to [1,Q_DIM,1,SEQ]
    m += &format!("        tensor<fp16, [1,{heads},{hd},{seq}]> at = transpose(perm=pm,x=a4)[name=string(\"ta\")];\n");
    m += &format!("        tensor<fp16, [1,{q_dim},1,{seq}]> af = reshape(shape=qsh,x=at)[name=string(\"ra\")];\n");

    // Convert RoPE'd Q,K back to flat layout for backward
    m += &format!("        tensor<fp16, [1,{heads},{hd},{seq}]> qrt = transpose(perm=pm,x=q_rope)[name=string(\"qrt\")];\n");
    m += &format!("        tensor<fp16, [1,{q_dim},1,{seq}]> qrf = reshape(shape=qsh,x=qrt)[name=string(\"qrf\")];\n");
    m += &format!("        tensor<fp16, [1,{kv_heads},{hd},{seq}]> krt = transpose(perm=pm,x=k_rope)[name=string(\"krt\")];\n");
    m += &format!("        tensor<fp16, [1,{kv_dim},1,{seq}]> krf = reshape(shape=kvsh,x=krt)[name=string(\"krf\")];\n");

    // Output: concat(attn_out, Q_rope, K_rope, V, xnorm)
    m += &format!("        tensor<fp16, [1,{out_ch},1,{seq}]> out = concat(axis=cax,interleave=cid,values=(af,qrf,krf,vf,xn))[name=string(\"cat\")];\n");
    m += &mil_footer("out");

    MilProgram {
        text: m,
        input_channels: dim,
        input_spatial: sp,
        output_channels: out_ch,
        output_spatial: seq,
    }
}

/// SDPA backward part 1: recompute attention + dV, dp
/// Input: [1, 4*Q_DIM, 1, SEQ] (Q, K_tiled, V_tiled, da)
/// Output: [1, Q_DIM+2*SCORE_CH, 1, SEQ] (dV_full, probs, dp)
pub fn sdpa_bwd1(cfg: &ModelConfig) -> MilProgram {
    let q_dim = cfg.q_dim;
    let heads = cfg.heads;
    let hd = cfg.hd;
    let seq = cfg.seq;
    let score_ch = cfg.score_ch();
    let sc = 1.0 / (hd as f32).sqrt();
    let in_ch = 4 * q_dim;
    let out_ch = q_dim + 2 * score_ch;

    let mut m = mil_header(in_ch, seq);

    // Slice Q, K_tiled, V_tiled, da (all [Q_DIM, SEQ], sliced along channel)
    m += &format!("        tensor<int32, [4]> sz = const()[name=string(\"sz\"), val=tensor<int32, [4]>([1,{q_dim},1,{seq}])];\n");
    m += "        tensor<int32, [4]> b0 = const()[name=string(\"b0\"), val=tensor<int32, [4]>([0,0,0,0])];\n";
    m += &format!("        tensor<fp16, [1,{q_dim},1,{seq}]> qf = slice_by_size(x=x,begin=b0,size=sz)[name=string(\"s0\")];\n");
    m += &format!("        tensor<int32, [4]> b1 = const()[name=string(\"b1\"), val=tensor<int32, [4]>([0,{q_dim},0,0])];\n");
    m += &format!("        tensor<fp16, [1,{q_dim},1,{seq}]> kf = slice_by_size(x=x,begin=b1,size=sz)[name=string(\"s1\")];\n");
    m += &format!("        tensor<int32, [4]> b2 = const()[name=string(\"b2\"), val=tensor<int32, [4]>([0,{},0,0])];\n", 2*q_dim);
    m += &format!("        tensor<fp16, [1,{q_dim},1,{seq}]> vf = slice_by_size(x=x,begin=b2,size=sz)[name=string(\"s2\")];\n");
    m += &format!("        tensor<int32, [4]> b3 = const()[name=string(\"b3\"), val=tensor<int32, [4]>([0,{},0,0])];\n", 3*q_dim);
    m += &format!("        tensor<fp16, [1,{q_dim},1,{seq}]> da = slice_by_size(x=x,begin=b3,size=sz)[name=string(\"s3\")];\n");

    // Reshape to heads
    m += &format!("        tensor<int32, [4]> rsh = const()[name=string(\"rsh\"), val=tensor<int32, [4]>([1,{heads},{hd},{seq}])];\n");
    m += "        tensor<int32, [4]> pm = const()[name=string(\"pm\"), val=tensor<int32, [4]>([0,1,3,2])];\n";
    for (src, dst_r, dst_t) in [
        ("qf", "qr", "q"),
        ("kf", "kr", "k"),
        ("vf", "vr", "v"),
        ("da", "dr", "dat"),
    ] {
        m += &format!("        tensor<fp16, [1,{heads},{hd},{seq}]> {dst_r} = reshape(shape=rsh,x={src})[name=string(\"r{dst_r}\")];\n");
        m += &format!("        tensor<fp16, [1,{heads},{seq},{hd}]> {dst_t} = transpose(perm=pm,x={dst_r})[name=string(\"t{dst_t}\")];\n");
    }

    // Recompute attention
    m += "        bool bF = const()[name=string(\"bF\"), val=bool(false)];\n";
    m += "        bool bT = const()[name=string(\"bT\"), val=bool(true)];\n";
    m += &format!("        tensor<fp16, [1,{heads},{seq},{seq}]> sc1 = matmul(transpose_x=bF,transpose_y=bT,x=q,y=k)[name=string(\"mm1\")];\n");
    m += &format!("        fp16 scv = const()[name=string(\"scv\"), val=fp16({sc})];\n");
    m += &format!("        tensor<fp16, [1,{heads},{seq},{seq}]> sc2 = mul(x=sc1,y=scv)[name=string(\"scl\")];\n");
    m += &format!("        tensor<fp16, [1,1,{seq},{seq}]> cm = const()[name=string(\"cm\"), val=tensor<fp16, [1,1,{seq},{seq}]>(BLOBFILE(path=string(\"@model_path/weights/mask.bin\"), offset=uint64(64)))];\n");
    m += &format!("        tensor<fp16, [1,{heads},{seq},{seq}]> ms = add(x=sc2,y=cm)[name=string(\"msk\")];\n");
    m += "        int32 sax = const()[name=string(\"sax\"), val=int32(-1)];\n";
    m += &format!("        tensor<fp16, [1,{heads},{seq},{seq}]> probs = softmax(axis=sax,x=ms)[name=string(\"sm\")];\n");

    // dV = probs^T @ da, dp = da @ V^T
    m += &format!("        tensor<fp16, [1,{heads},{seq},{hd}]> dv4 = matmul(transpose_x=bT,transpose_y=bF,x=probs,y=dat)[name=string(\"dv\")];\n");
    m += &format!("        tensor<fp16, [1,{heads},{seq},{seq}]> dp4 = matmul(transpose_x=bF,transpose_y=bT,x=dat,y=v)[name=string(\"dp\")];\n");

    // Reshape outputs
    m += &format!("        tensor<fp16, [1,{heads},{hd},{seq}]> dvt = transpose(perm=pm,x=dv4)[name=string(\"dvt\")];\n");
    m += &format!("        tensor<int32, [4]> dvs = const()[name=string(\"dvs\"), val=tensor<int32, [4]>([1,{q_dim},1,{seq}])];\n");
    m += &format!("        tensor<fp16, [1,{q_dim},1,{seq}]> dvf = reshape(shape=dvs,x=dvt)[name=string(\"dvf\")];\n");
    m += &format!("        tensor<int32, [4]> scs = const()[name=string(\"scs\"), val=tensor<int32, [4]>([1,{score_ch},1,{seq}])];\n");
    m += &format!("        tensor<fp16, [1,{score_ch},1,{seq}]> pf = reshape(shape=scs,x=probs)[name=string(\"pf\")];\n");
    m += &format!("        tensor<fp16, [1,{score_ch},1,{seq}]> dpf = reshape(shape=scs,x=dp4)[name=string(\"dpf\")];\n");

    m += "        int32 cax = const()[name=string(\"cax\"), val=int32(1)];\n";
    m += "        bool cid = const()[name=string(\"cid\"), val=bool(false)];\n";
    m += &format!("        tensor<fp16, [1,{out_ch},1,{seq}]> out = concat(axis=cax,interleave=cid,values=(dvf,pf,dpf))[name=string(\"cat\")];\n");
    m += &mil_footer("out");

    MilProgram {
        text: m,
        input_channels: in_ch,
        input_spatial: seq,
        output_channels: out_ch,
        output_spatial: seq,
    }
}

/// SDPA backward part 2: softmax backward + dQ, dK
/// Input: [1, 2*SCORE_CH + 2*Q_DIM, 1, SEQ]
/// Output: [1, 2*Q_DIM, 1, SEQ]
pub fn sdpa_bwd2(cfg: &ModelConfig) -> MilProgram {
    let q_dim = cfg.q_dim;
    let heads = cfg.heads;
    let hd = cfg.hd;
    let seq = cfg.seq;
    let score_ch = cfg.score_ch();
    let sc = 1.0 / (hd as f32).sqrt();
    let in_ch = 2 * score_ch + 2 * q_dim;
    let out_ch = 2 * q_dim;

    let mut m = mil_header(in_ch, seq);

    // Slice probs, dp, Q, K (along channel axis)
    m += &format!("        tensor<int32, [4]> sz_sc = const()[name=string(\"szsc\"), val=tensor<int32, [4]>([1,{score_ch},1,{seq}])];\n");
    m += "        tensor<int32, [4]> b0 = const()[name=string(\"b0\"), val=tensor<int32, [4]>([0,0,0,0])];\n";
    m += &format!("        tensor<fp16, [1,{score_ch},1,{seq}]> pf = slice_by_size(x=x,begin=b0,size=sz_sc)[name=string(\"s0\")];\n");
    m += &format!("        tensor<int32, [4]> b1 = const()[name=string(\"b1\"), val=tensor<int32, [4]>([0,{score_ch},0,0])];\n");
    m += &format!("        tensor<fp16, [1,{score_ch},1,{seq}]> dpf = slice_by_size(x=x,begin=b1,size=sz_sc)[name=string(\"s1\")];\n");
    m += &format!("        tensor<int32, [4]> sz_q = const()[name=string(\"szq\"), val=tensor<int32, [4]>([1,{q_dim},1,{seq}])];\n");
    m += &format!("        tensor<int32, [4]> b2 = const()[name=string(\"b2\"), val=tensor<int32, [4]>([0,{},0,0])];\n", 2*score_ch);
    m += &format!("        tensor<fp16, [1,{q_dim},1,{seq}]> qf = slice_by_size(x=x,begin=b2,size=sz_q)[name=string(\"s2\")];\n");
    m += &format!("        tensor<int32, [4]> b3 = const()[name=string(\"b3\"), val=tensor<int32, [4]>([0,{},0,0])];\n", 2*score_ch+q_dim);
    m += &format!("        tensor<fp16, [1,{q_dim},1,{seq}]> kf = slice_by_size(x=x,begin=b3,size=sz_q)[name=string(\"s3\")];\n");

    // Reshape to heads
    m += &format!("        tensor<int32, [4]> ssh = const()[name=string(\"ssh\"), val=tensor<int32, [4]>([1,{heads},{seq},{seq}])];\n");
    m += &format!("        tensor<fp16, [1,{heads},{seq},{seq}]> probs = reshape(shape=ssh,x=pf)[name=string(\"rp\")];\n");
    m += &format!("        tensor<fp16, [1,{heads},{seq},{seq}]> dp = reshape(shape=ssh,x=dpf)[name=string(\"rdp\")];\n");
    m += &format!("        tensor<int32, [4]> rsh = const()[name=string(\"rsh\"), val=tensor<int32, [4]>([1,{heads},{hd},{seq}])];\n");
    m += "        tensor<int32, [4]> pm = const()[name=string(\"pm\"), val=tensor<int32, [4]>([0,1,3,2])];\n";
    m += &format!("        tensor<fp16, [1,{heads},{hd},{seq}]> qr = reshape(shape=rsh,x=qf)[name=string(\"rq\")];\n");
    m += &format!("        tensor<fp16, [1,{heads},{seq},{hd}]> q = transpose(perm=pm,x=qr)[name=string(\"tq\")];\n");
    m += &format!("        tensor<fp16, [1,{heads},{hd},{seq}]> kr = reshape(shape=rsh,x=kf)[name=string(\"rk\")];\n");
    m += &format!("        tensor<fp16, [1,{heads},{seq},{hd}]> k = transpose(perm=pm,x=kr)[name=string(\"tk\")];\n");

    // Softmax backward: ds = (dp - sum(dp*probs)) * probs * scale
    m += &format!("        tensor<fp16, [1,{heads},{seq},{seq}]> pdp = mul(x=probs,y=dp)[name=string(\"pdp\")];\n");
    m += "        tensor<int32, [1]> rax = const()[name=string(\"rax\"), val=tensor<int32, [1]>([-1])];\n";
    m += "        bool kd = const()[name=string(\"kd\"), val=bool(true)];\n";
    m += &format!("        tensor<fp16, [1,{heads},{seq},1]> spdp = reduce_sum(x=pdp,axes=rax,keep_dims=kd)[name=string(\"rs\")];\n");
    m += &format!("        tensor<fp16, [1,{heads},{seq},{seq}]> dps = sub(x=dp,y=spdp)[name=string(\"dps\")];\n");
    m += &format!("        tensor<fp16, [1,{heads},{seq},{seq}]> ds0 = mul(x=probs,y=dps)[name=string(\"ds0\")];\n");
    m += &format!("        fp16 scv = const()[name=string(\"scv\"), val=fp16({sc})];\n");
    m += &format!("        tensor<fp16, [1,{heads},{seq},{seq}]> ds = mul(x=ds0,y=scv)[name=string(\"ds\")];\n");

    // dQ = ds @ K, dK = ds^T @ Q
    m += "        bool bF = const()[name=string(\"bF\"), val=bool(false)];\n";
    m += "        bool bT = const()[name=string(\"bT\"), val=bool(true)];\n";
    m += &format!("        tensor<fp16, [1,{heads},{seq},{hd}]> dq4 = matmul(transpose_x=bF,transpose_y=bF,x=ds,y=k)[name=string(\"dq\")];\n");
    m += &format!("        tensor<fp16, [1,{heads},{seq},{hd}]> dk4 = matmul(transpose_x=bT,transpose_y=bF,x=ds,y=q)[name=string(\"dk\")];\n");

    // Flatten back
    m += &format!("        tensor<fp16, [1,{heads},{hd},{seq}]> dqt = transpose(perm=pm,x=dq4)[name=string(\"dqt\")];\n");
    m += &format!("        tensor<fp16, [1,{heads},{hd},{seq}]> dkt = transpose(perm=pm,x=dk4)[name=string(\"dkt\")];\n");
    m += &format!("        tensor<int32, [4]> fs = const()[name=string(\"fs\"), val=tensor<int32, [4]>([1,{q_dim},1,{seq}])];\n");
    m += &format!("        tensor<fp16, [1,{q_dim},1,{seq}]> dqf = reshape(shape=fs,x=dqt)[name=string(\"dqf\")];\n");
    m += &format!("        tensor<fp16, [1,{q_dim},1,{seq}]> dkf = reshape(shape=fs,x=dkt)[name=string(\"dkf\")];\n");

    m += "        int32 cax = const()[name=string(\"cax\"), val=int32(1)];\n";
    m += "        bool cid = const()[name=string(\"cid\"), val=bool(false)];\n";
    m += &format!("        tensor<fp16, [1,{out_ch},1,{seq}]> out = concat(axis=cax,interleave=cid,values=(dqf,dkf))[name=string(\"cat\")];\n");
    m += &mil_footer("out");

    MilProgram {
        text: m,
        input_channels: in_ch,
        input_spatial: seq,
        output_channels: out_ch,
        output_spatial: seq,
    }
}
