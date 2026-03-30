//! Projection MIL kernels: QKV, Wo, and backward passes
//! Ports gen_qkv_proj_dynamic, gen_wo_fwd_dynamic, gen_*_bwd from mil_dynamic.h

use ane::{gen_dyn_matmul, mil_footer, mil_header, MilProgram};
use crate::config::ModelConfig;

/// QKV projection: xnorm @ [Wq, Wk, Wv] → concat(Q, K, V)
/// Input: [1, DIM, 1, SEQ + Q_DIM + KV_DIM + KV_DIM]
/// Output: [1, Q_DIM+KV_DIM+KV_DIM, 1, SEQ]
pub fn qkv_proj(cfg: &ModelConfig) -> MilProgram {
    let sp = cfg.qkv_proj_sp();
    let out_ch = cfg.q_dim + cfg.kv_dim + cfg.kv_dim;
    let seq = cfg.seq;
    let dim = cfg.dim;
    let mut m = mil_header(dim, sp);

    // Three matmuls: Q, K, V
    gen_dyn_matmul(&mut m, "q", dim, cfg.q_dim, seq, 0, seq, "x");
    gen_dyn_matmul(&mut m, "k", dim, cfg.kv_dim, seq, 0, seq + cfg.q_dim, "x");
    gen_dyn_matmul(
        &mut m,
        "v",
        dim,
        cfg.kv_dim,
        seq,
        0,
        seq + cfg.q_dim + cfg.kv_dim,
        "x",
    );

    // Concat Q, K, V along channel axis
    m += "        int32 cax = const()[name=string(\"cax\"), val=int32(1)];\n";
    m += "        bool cid = const()[name=string(\"cid\"), val=bool(false)];\n";
    m += &format!("        tensor<fp16, [1,{out_ch},1,{seq}]> out = concat(axis=cax,interleave=cid,values=(q_y,k_y,v_y))[name=string(\"out\")];\n");

    m += &mil_footer("out");
    MilProgram {
        text: m,
        input_channels: dim,
        input_spatial: sp,
        output_channels: out_ch,
        output_spatial: seq,
    }
}

/// Wo forward: attn_out @ Wo^T → [1, DIM, 1, SEQ]
/// Input: [1, Q_DIM, 1, SEQ+DIM]
/// Output: [1, DIM, 1, SEQ]
pub fn wo_fwd(cfg: &ModelConfig) -> MilProgram {
    let sp = cfg.wo_fwd_sp();
    let mut m = mil_header(cfg.q_dim, sp);
    gen_dyn_matmul(&mut m, "wo", cfg.q_dim, cfg.dim, cfg.seq, 0, cfg.seq, "x");
    m += &mil_footer("wo_y");
    MilProgram {
        text: m,
        input_channels: cfg.q_dim,
        input_spatial: sp,
        output_channels: cfg.dim,
        output_spatial: cfg.seq,
    }
}

/// FFN backward W2^T: dffn @ W2^T → [1, HIDDEN, 1, SEQ]
/// Input: [1, DIM, 1, SEQ+HIDDEN]
/// Output: [1, HIDDEN, 1, SEQ]
pub fn ffn_bwd_w2t(cfg: &ModelConfig) -> MilProgram {
    let sp = cfg.ffn_bwd_w2t_sp();
    let mut m = mil_header(cfg.dim, sp);
    gen_dyn_matmul(&mut m, "bw2", cfg.dim, cfg.hidden, cfg.seq, 0, cfg.seq, "x");
    m += &mil_footer("bw2_y");
    MilProgram {
        text: m,
        input_channels: cfg.dim,
        input_spatial: sp,
        output_channels: cfg.hidden,
        output_spatial: cfg.seq,
    }
}

/// FFN backward W1^T + W3^T: dh1@W1^T + dh3@W3^T → [1, DIM, 1, SEQ]
/// Input: [1, HIDDEN, 1, 2*SEQ+2*DIM]
/// Output: [1, DIM, 1, SEQ]
/// Port of gen_ffn_bwd_w13t_dynamic — manual slice+matmul+add (no gen_dyn_matmul)
pub fn ffn_bwd_w13t(cfg: &ModelConfig) -> MilProgram {
    let sp = cfg.ffn_bwd_w13t_sp();
    let seq = cfg.seq;
    let dim = cfg.dim;
    let hidden = cfg.hidden;
    let mut m = mil_header(hidden, sp);

    // Slice dh1, dh3, W1^T, W3^T
    m += &format!("        tensor<int32, [4]> sh = const()[name=string(\"sh\"), val=tensor<int32, [4]>([1,{hidden},1,{seq}])];\n");
    m += "        tensor<int32, [4]> b0 = const()[name=string(\"b0\"), val=tensor<int32, [4]>([0,0,0,0])];\n";
    m += &format!("        tensor<fp16, [1,{hidden},1,{seq}]> dh1 = slice_by_size(x=x,begin=b0,size=sh)[name=string(\"dh1\")];\n");
    m += &format!("        tensor<int32, [4]> b1 = const()[name=string(\"b1\"), val=tensor<int32, [4]>([0,0,0,{seq}])];\n");
    m += &format!("        tensor<fp16, [1,{hidden},1,{seq}]> dh3 = slice_by_size(x=x,begin=b1,size=sh)[name=string(\"dh3\")];\n");
    m += &format!("        tensor<int32, [4]> b2 = const()[name=string(\"b2\"), val=tensor<int32, [4]>([0,0,0,{}])];\n", 2*seq);
    m += &format!("        tensor<int32, [4]> sw = const()[name=string(\"sw\"), val=tensor<int32, [4]>([1,{hidden},1,{dim}])];\n");
    m += &format!("        tensor<fp16, [1,{hidden},1,{dim}]> W1t = slice_by_size(x=x,begin=b2,size=sw)[name=string(\"W1t\")];\n");
    m += &format!("        tensor<int32, [4]> b3 = const()[name=string(\"b3\"), val=tensor<int32, [4]>([0,0,0,{}])];\n", 2*seq+dim);
    m += &format!("        tensor<fp16, [1,{hidden},1,{dim}]> W3t = slice_by_size(x=x,begin=b3,size=sw)[name=string(\"W3t\")];\n");

    // Reshape + transpose for matmul
    m += "        tensor<int32, [4]> pm = const()[name=string(\"pm\"), val=tensor<int32, [4]>([0,1,3,2])];\n";
    m += &format!("        tensor<int32, [4]> ra = const()[name=string(\"ra\"), val=tensor<int32, [4]>([1,1,{hidden},{seq}])];\n");
    m += &format!("        tensor<fp16, [1,1,{hidden},{seq}]> dh12 = reshape(shape=ra,x=dh1)[name=string(\"dh12\")];\n");
    m += &format!("        tensor<fp16, [1,1,{seq},{hidden}]> dh1t = transpose(perm=pm,x=dh12)[name=string(\"dh1t\")];\n");
    m += &format!("        tensor<fp16, [1,1,{hidden},{seq}]> dh32 = reshape(shape=ra,x=dh3)[name=string(\"dh32\")];\n");
    m += &format!("        tensor<fp16, [1,1,{seq},{hidden}]> dh3t = transpose(perm=pm,x=dh32)[name=string(\"dh3t\")];\n");
    m += &format!("        tensor<int32, [4]> rw = const()[name=string(\"rw\"), val=tensor<int32, [4]>([1,1,{hidden},{dim}])];\n");
    m += &format!("        tensor<fp16, [1,1,{hidden},{dim}]> W1t2 = reshape(shape=rw,x=W1t)[name=string(\"W1t2\")];\n");
    m += &format!("        tensor<fp16, [1,1,{hidden},{dim}]> W3t2 = reshape(shape=rw,x=W3t)[name=string(\"W3t2\")];\n");

    // Matmuls + add
    m += "        bool bF = const()[name=string(\"bF\"), val=bool(false)];\n";
    m += &format!("        tensor<fp16, [1,1,{seq},{dim}]> dx1m = matmul(transpose_x=bF,transpose_y=bF,x=dh1t,y=W1t2)[name=string(\"dx1m\")];\n");
    m += &format!("        tensor<fp16, [1,1,{seq},{dim}]> dx3m = matmul(transpose_x=bF,transpose_y=bF,x=dh3t,y=W3t2)[name=string(\"dx3m\")];\n");
    m += &format!(
        "        tensor<fp16, [1,1,{seq},{dim}]> dxm = add(x=dx1m,y=dx3m)[name=string(\"dxm\")];\n"
    );
    m += &format!("        tensor<fp16, [1,1,{dim},{seq}]> dxt = transpose(perm=pm,x=dxm)[name=string(\"dxt\")];\n");
    m += &format!("        tensor<int32, [4]> ro = const()[name=string(\"ro\"), val=tensor<int32, [4]>([1,{dim},1,{seq}])];\n");
    m += &format!("        tensor<fp16, [1,{dim},1,{seq}]> dx = reshape(shape=ro,x=dxt)[name=string(\"dx\")];\n");
    m += &mil_footer("dx");

    MilProgram {
        text: m,
        input_channels: hidden,
        input_spatial: sp,
        output_channels: dim,
        output_spatial: seq,
    }
}

/// Wo^T backward: dy @ Wo → [1, Q_DIM, 1, SEQ]
/// Input: [1, DIM, 1, SEQ+Q_DIM]
/// Output: [1, Q_DIM, 1, SEQ]
pub fn wot_bwd(cfg: &ModelConfig) -> MilProgram {
    let sp = cfg.wot_bwd_sp();
    let mut m = mil_header(cfg.dim, sp);
    gen_dyn_matmul(&mut m, "wot", cfg.dim, cfg.q_dim, cfg.seq, 0, cfg.seq, "x");
    m += &mil_footer("wot_y");
    MilProgram {
        text: m,
        input_channels: cfg.dim,
        input_spatial: sp,
        output_channels: cfg.q_dim,
        output_spatial: cfg.seq,
    }
}

/// Q backward: dq @ Wq^T → [1, DIM, 1, SEQ]
/// Input: [1, Q_DIM, 1, SEQ+DIM]
/// Output: [1, DIM, 1, SEQ]
pub fn q_bwd(cfg: &ModelConfig) -> MilProgram {
    let sp = cfg.q_bwd_sp();
    let mut m = mil_header(cfg.q_dim, sp);
    gen_dyn_matmul(&mut m, "qb", cfg.q_dim, cfg.dim, cfg.seq, 0, cfg.seq, "x");
    m += &mil_footer("qb_y");
    MilProgram {
        text: m,
        input_channels: cfg.q_dim,
        input_spatial: sp,
        output_channels: cfg.dim,
        output_spatial: cfg.seq,
    }
}

/// KV backward: dk@Wk^T + dv@Wv^T → [1, DIM, 1, SEQ]
/// Input: [1, KV_DIM, 1, 2*SEQ+2*DIM]
/// Output: [1, DIM, 1, SEQ]
/// Port of gen_kv_bwd_dynamic — manual slice+matmul+add
pub fn kv_bwd(cfg: &ModelConfig) -> MilProgram {
    let sp = cfg.kv_bwd_sp();
    let seq = cfg.seq;
    let dim = cfg.dim;
    let kv_dim = cfg.kv_dim;
    let mut m = mil_header(kv_dim, sp);

    m += &format!("        tensor<int32, [4]> sh = const()[name=string(\"sh\"), val=tensor<int32, [4]>([1,{kv_dim},1,{seq}])];\n");
    m += "        tensor<int32, [4]> b0 = const()[name=string(\"b0\"), val=tensor<int32, [4]>([0,0,0,0])];\n";
    m += &format!("        tensor<fp16, [1,{kv_dim},1,{seq}]> dk = slice_by_size(x=x,begin=b0,size=sh)[name=string(\"dk\")];\n");
    m += &format!("        tensor<int32, [4]> b1 = const()[name=string(\"b1\"), val=tensor<int32, [4]>([0,0,0,{seq}])];\n");
    m += &format!("        tensor<fp16, [1,{kv_dim},1,{seq}]> dv = slice_by_size(x=x,begin=b1,size=sh)[name=string(\"dv\")];\n");
    m += &format!("        tensor<int32, [4]> b2 = const()[name=string(\"b2\"), val=tensor<int32, [4]>([0,0,0,{}])];\n", 2*seq);
    m += &format!("        tensor<int32, [4]> sw = const()[name=string(\"sw\"), val=tensor<int32, [4]>([1,{kv_dim},1,{dim}])];\n");
    m += &format!("        tensor<fp16, [1,{kv_dim},1,{dim}]> Wkt = slice_by_size(x=x,begin=b2,size=sw)[name=string(\"Wkt\")];\n");
    m += &format!("        tensor<int32, [4]> b3 = const()[name=string(\"b3\"), val=tensor<int32, [4]>([0,0,0,{}])];\n", 2*seq+dim);
    m += &format!("        tensor<fp16, [1,{kv_dim},1,{dim}]> Wvt = slice_by_size(x=x,begin=b3,size=sw)[name=string(\"Wvt\")];\n");

    m += "        tensor<int32, [4]> pm = const()[name=string(\"pm\"), val=tensor<int32, [4]>([0,1,3,2])];\n";
    m += &format!("        tensor<int32, [4]> ra = const()[name=string(\"ra\"), val=tensor<int32, [4]>([1,1,{kv_dim},{seq}])];\n");
    m += &format!("        tensor<fp16, [1,1,{kv_dim},{seq}]> dk2 = reshape(shape=ra,x=dk)[name=string(\"dk2\")];\n");
    m += &format!("        tensor<fp16, [1,1,{seq},{kv_dim}]> dkt = transpose(perm=pm,x=dk2)[name=string(\"dkt\")];\n");
    m += &format!("        tensor<fp16, [1,1,{kv_dim},{seq}]> dv2 = reshape(shape=ra,x=dv)[name=string(\"dv2\")];\n");
    m += &format!("        tensor<fp16, [1,1,{seq},{kv_dim}]> dvt = transpose(perm=pm,x=dv2)[name=string(\"dvt\")];\n");
    m += &format!("        tensor<int32, [4]> rw = const()[name=string(\"rw\"), val=tensor<int32, [4]>([1,1,{kv_dim},{dim}])];\n");
    m += &format!("        tensor<fp16, [1,1,{kv_dim},{dim}]> Wkt2 = reshape(shape=rw,x=Wkt)[name=string(\"Wkt2\")];\n");
    m += &format!("        tensor<fp16, [1,1,{kv_dim},{dim}]> Wvt2 = reshape(shape=rw,x=Wvt)[name=string(\"Wvt2\")];\n");

    m += "        bool bF = const()[name=string(\"bF\"), val=bool(false)];\n";
    m += &format!("        tensor<fp16, [1,1,{seq},{dim}]> dxk = matmul(transpose_x=bF,transpose_y=bF,x=dkt,y=Wkt2)[name=string(\"dxk\")];\n");
    m += &format!("        tensor<fp16, [1,1,{seq},{dim}]> dxv = matmul(transpose_x=bF,transpose_y=bF,x=dvt,y=Wvt2)[name=string(\"dxv\")];\n");
    m += &format!(
        "        tensor<fp16, [1,1,{seq},{dim}]> dxm = add(x=dxk,y=dxv)[name=string(\"dxm\")];\n"
    );
    m += &format!("        tensor<fp16, [1,1,{dim},{seq}]> dxt = transpose(perm=pm,x=dxm)[name=string(\"dxt\")];\n");
    m += &format!("        tensor<int32, [4]> ro = const()[name=string(\"ro\"), val=tensor<int32, [4]>([1,{dim},1,{seq}])];\n");
    m += &format!("        tensor<fp16, [1,{dim},1,{seq}]> dx = reshape(shape=ro,x=dxt)[name=string(\"dx\")];\n");
    m += &mil_footer("dx");

    MilProgram {
        text: m,
        input_channels: kv_dim,
        input_spatial: sp,
        output_channels: dim,
        output_spatial: seq,
    }
}
