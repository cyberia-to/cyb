// KV Cache management — replaces ~48 buffer copies per layer with 2 dispatches
//
// kv_append: copies past KV + appends new KV into full buffer
// Layout: [heads, seq, dim] per head
//
// Also handles GQA head expansion: repeat kv_heads to match num_heads

struct Params {
    num_heads: u32,      // kv_num_heads for append, num_heads for expand
    head_dim: u32,
    past_seq: u32,
    new_seq: u32,        // typically 1 for decode
    total_seq: u32,      // past_seq + new_seq
    kv_heads: u32,       // for expansion
    _pad0: u32,
    _pad1: u32,
}

// Append: past_kv[heads, past_seq, dim] + new_kv[new_seq * heads * dim] → full_kv[heads, total_seq, dim]
@group(0) @binding(0) var<storage, read> past_kv: array<f32>;
@group(0) @binding(1) var<storage, read> new_kv: array<f32>;
@group(0) @binding(2) var<storage, read_write> full_kv: array<f32>;
@group(0) @binding(3) var<uniform> params: Params;

@compute @workgroup_size(256)
fn kv_append(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    let total_elements = params.num_heads * params.total_seq * params.head_dim;
    if (idx >= total_elements) { return; }

    // Determine which head, seq position, and dim this element belongs to
    let dim_idx = idx % params.head_dim;
    let remaining = idx / params.head_dim;
    let seq_idx = remaining % params.total_seq;
    let head_idx = remaining / params.total_seq;

    if (seq_idx < params.past_seq) {
        // From past cache: past_kv[head, seq, dim]
        let past_idx = head_idx * params.past_seq * params.head_dim
                     + seq_idx * params.head_dim
                     + dim_idx;
        full_kv[idx] = past_kv[past_idx];
    } else {
        // From new KV: new_kv layout is [new_seq, heads, dim] (interleaved)
        let new_seq_pos = seq_idx - params.past_seq;
        let new_idx = (new_seq_pos * params.num_heads + head_idx) * params.head_dim + dim_idx;
        full_kv[idx] = new_kv[new_idx];
    }
}

// Expand: kv[kv_heads, seq, dim] → expanded[num_heads, seq, dim] with head repetition
@group(0) @binding(0) var<storage, read> kv_src: array<f32>;
@group(0) @binding(1) var<storage, read_write> kv_dst: array<f32>;
// reuse binding 3 for params
@group(0) @binding(2) var<uniform> expand_params: Params;

@compute @workgroup_size(256)
fn kv_expand(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    let total_elements = expand_params.num_heads * expand_params.total_seq * expand_params.head_dim;
    if (idx >= total_elements) { return; }

    let dim_idx = idx % expand_params.head_dim;
    let remaining = idx / expand_params.head_dim;
    let seq_idx = remaining % expand_params.total_seq;
    let head_idx = remaining / expand_params.total_seq;

    // Map expanded head back to KV head
    let repeats = expand_params.num_heads / expand_params.kv_heads;
    let kv_head = head_idx / repeats;

    let src_idx = kv_head * expand_params.total_seq * expand_params.head_dim
                + seq_idx * expand_params.head_dim
                + dim_idx;
    kv_dst[idx] = kv_src[src_idx];
}
