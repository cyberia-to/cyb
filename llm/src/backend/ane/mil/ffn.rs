//! FFN MIL kernel: fused SwiGLU forward with residual
//! Full port of gen_ffn_fused_dynamic_alpha from mil_dynamic.h

use rane::{mil_footer, mil_header, MilProgram};
use crate::config::ModelConfig;

/// Fused FFN forward: SwiGLU(x2norm @ W1, x2norm @ W3) @ W2 + x2 (residual)
/// Input: [1, DIM, 1, 2*SEQ + 3*HIDDEN]
///   sp[0:SEQ] = x2norm, sp[SEQ:2*SEQ] = x2 (residual)
///   sp[2*SEQ:] = W1^T, W3^T, W2
/// Output: [1, DIM+3*HIDDEN, 1, SEQ]
///   = concat(x_next, h1, h3, gate)
pub fn ffn_fused(cfg: &ModelConfig, alpha: f32) -> MilProgram {
    let seq = cfg.seq;
    let dim = cfg.dim;
    let hidden = cfg.hidden;
    let sp = cfg.ffn_fused_sp();
    let out_ch = dim + 3 * hidden;

    let mut m = mil_header(dim, sp);

    // Common constants
    m += "        tensor<int32, [4]> pm = const()[name=string(\"pm\"), val=tensor<int32, [4]>([0,1,3,2])];\n";
    m += "        bool bF = const()[name=string(\"bF\"), val=bool(false)];\n";

    // Slice x2norm [DIM, SEQ], x2 [DIM, SEQ], W1 [DIM, HIDDEN], W3 [DIM, HIDDEN], W2 [DIM, HIDDEN]
    m += "        tensor<int32, [4]> b_xn = const()[name=string(\"b_xn\"), val=tensor<int32, [4]>([0,0,0,0])];\n";
    m += &format!("        tensor<int32, [4]> s_ds = const()[name=string(\"s_ds\"), val=tensor<int32, [4]>([1,{dim},1,{seq}])];\n");
    m += &format!("        tensor<fp16, [1,{dim},1,{seq}]> x2norm = slice_by_size(x=x,begin=b_xn,size=s_ds)[name=string(\"x2norm\")];\n");
    m += &format!("        tensor<int32, [4]> b_x2 = const()[name=string(\"b_x2\"), val=tensor<int32, [4]>([0,0,0,{seq}])];\n");
    m += &format!("        tensor<fp16, [1,{dim},1,{seq}]> x2 = slice_by_size(x=x,begin=b_x2,size=s_ds)[name=string(\"x2\")];\n");
    m += &format!("        tensor<int32, [4]> b_w1 = const()[name=string(\"b_w1\"), val=tensor<int32, [4]>([0,0,0,{}])];\n", 2*seq);
    m += &format!("        tensor<int32, [4]> s_wh = const()[name=string(\"s_wh\"), val=tensor<int32, [4]>([1,{dim},1,{hidden}])];\n");
    m += &format!("        tensor<fp16, [1,{dim},1,{hidden}]> W1 = slice_by_size(x=x,begin=b_w1,size=s_wh)[name=string(\"W1\")];\n");
    m += &format!("        tensor<int32, [4]> b_w3 = const()[name=string(\"b_w3\"), val=tensor<int32, [4]>([0,0,0,{}])];\n", 2*seq+hidden);
    m += &format!("        tensor<fp16, [1,{dim},1,{hidden}]> W3 = slice_by_size(x=x,begin=b_w3,size=s_wh)[name=string(\"W3\")];\n");
    m += &format!("        tensor<int32, [4]> b_w2 = const()[name=string(\"b_w2\"), val=tensor<int32, [4]>([0,0,0,{}])];\n", 2*seq+2*hidden);
    m += &format!("        tensor<fp16, [1,{dim},1,{hidden}]> W2r = slice_by_size(x=x,begin=b_w2,size=s_wh)[name=string(\"W2r\")];\n");

    // xnorm matmul: reshape [1,DIM,1,SEQ] → [1,1,DIM,SEQ] → transpose → [1,1,SEQ,DIM]
    m += &format!("        tensor<int32, [4]> rd = const()[name=string(\"rd\"), val=tensor<int32, [4]>([1,1,{dim},{seq}])];\n");
    m += &format!("        tensor<fp16, [1,1,{dim},{seq}]> xn2 = reshape(shape=rd,x=x2norm)[name=string(\"xn2\")];\n");
    m += &format!("        tensor<fp16, [1,1,{seq},{dim}]> xnt = transpose(perm=pm,x=xn2)[name=string(\"xnt\")];\n");
    m += &format!("        tensor<int32, [4]> rw = const()[name=string(\"rw\"), val=tensor<int32, [4]>([1,1,{dim},{hidden}])];\n");
    m += &format!("        tensor<fp16, [1,1,{dim},{hidden}]> W12 = reshape(shape=rw,x=W1)[name=string(\"W12\")];\n");
    m += &format!("        tensor<fp16, [1,1,{dim},{hidden}]> W32 = reshape(shape=rw,x=W3)[name=string(\"W32\")];\n");

    // h1 = xnorm @ W1, h3 = xnorm @ W3
    m += &format!("        tensor<fp16, [1,1,{seq},{hidden}]> h1m = matmul(transpose_x=bF,transpose_y=bF,x=xnt,y=W12)[name=string(\"h1m\")];\n");
    m += &format!("        tensor<fp16, [1,1,{seq},{hidden}]> h3m = matmul(transpose_x=bF,transpose_y=bF,x=xnt,y=W32)[name=string(\"h3m\")];\n");

    // Reshape back to [1,HIDDEN,1,SEQ]
    m += &format!("        tensor<fp16, [1,1,{hidden},{seq}]> h1t = transpose(perm=pm,x=h1m)[name=string(\"h1t\")];\n");
    m += &format!("        tensor<fp16, [1,1,{hidden},{seq}]> h3t = transpose(perm=pm,x=h3m)[name=string(\"h3t\")];\n");
    m += &format!("        tensor<int32, [4]> rh = const()[name=string(\"rh\"), val=tensor<int32, [4]>([1,{hidden},1,{seq}])];\n");
    m += &format!("        tensor<fp16, [1,{hidden},1,{seq}]> h1 = reshape(shape=rh,x=h1t)[name=string(\"h1\")];\n");
    m += &format!("        tensor<fp16, [1,{hidden},1,{seq}]> h3 = reshape(shape=rh,x=h3t)[name=string(\"h3\")];\n");

    // SiLU + gate
    m += &format!(
        "        tensor<fp16, [1,{hidden},1,{seq}]> sig = sigmoid(x=h1)[name=string(\"sg\")];\n"
    );
    m += &format!(
        "        tensor<fp16, [1,{hidden},1,{seq}]> silu = mul(x=h1,y=sig)[name=string(\"si\")];\n"
    );
    m += &format!("        tensor<fp16, [1,{hidden},1,{seq}]> gate = mul(x=silu,y=h3)[name=string(\"gt\")];\n");

    // gate @ W2: reshape gate, transpose W2
    m += &format!("        tensor<int32, [4]> rg = const()[name=string(\"rg\"), val=tensor<int32, [4]>([1,1,{hidden},{seq}])];\n");
    m += &format!("        tensor<fp16, [1,1,{hidden},{seq}]> g2 = reshape(shape=rg,x=gate)[name=string(\"g2\")];\n");
    m += &format!("        tensor<fp16, [1,1,{seq},{hidden}]> gt = transpose(perm=pm,x=g2)[name=string(\"gtt\")];\n");
    m += &format!("        tensor<fp16, [1,1,{dim},{hidden}]> W22 = reshape(shape=rw,x=W2r)[name=string(\"W22\")];\n");
    m += &format!("        tensor<fp16, [1,1,{hidden},{dim}]> W2t = transpose(perm=pm,x=W22)[name=string(\"W2t\")];\n");
    m += &format!("        tensor<fp16, [1,1,{seq},{dim}]> fm = matmul(transpose_x=bF,transpose_y=bF,x=gt,y=W2t)[name=string(\"fm\")];\n");
    m += &format!("        tensor<fp16, [1,1,{dim},{seq}]> ft = transpose(perm=pm,x=fm)[name=string(\"ft\")];\n");
    m += &format!("        tensor<int32, [4]> rd2 = const()[name=string(\"rd2\"), val=tensor<int32, [4]>([1,{dim},1,{seq}])];\n");
    m += &format!("        tensor<fp16, [1,{dim},1,{seq}]> ffn_out = reshape(shape=rd2,x=ft)[name=string(\"ffn_out\")];\n");

    // Residual: x_next = x2 + alpha * ffn_out
    m += &format!(
        "        fp16 res_alpha = const()[name=string(\"res_alpha\"), val=fp16({alpha})];\n"
    );
    m += &format!("        tensor<fp16, [1,{dim},1,{seq}]> ffn_scaled = mul(x=ffn_out,y=res_alpha)[name=string(\"ffn_sc\")];\n");
    m += &format!("        tensor<fp16, [1,{dim},1,{seq}]> x_next = add(x=x2,y=ffn_scaled)[name=string(\"x_next\")];\n");

    // Output: concat(x_next, h1, h3, gate)
    m += "        int32 cax = const()[name=string(\"cax\"), val=int32(1)];\n";
    m += "        bool cid = const()[name=string(\"cid\"), val=bool(false)];\n";
    m += &format!("        tensor<fp16, [1,{out_ch},1,{seq}]> out = concat(axis=cax,interleave=cid,values=(x_next,h1,h3,gate))[name=string(\"cat\")];\n");
    m += &mil_footer("out");

    MilProgram {
        text: m,
        input_channels: dim,
        input_spatial: sp,
        output_channels: out_ch,
        output_spatial: seq,
    }
}
