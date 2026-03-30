// Adapter operations: LoRA apply, Kronecker product

// === LoRA Apply ===
// Low-rank adaptation: output = x + alpha * (up @ down @ x)
// down: [rank, in_dim] — projects input to low-rank space
// up: [out_dim, rank] — projects back to full space
// For efficiency, this fused kernel does the full LoRA forward pass:
//   1. low = down @ x   [rank]
//   2. high = up @ low   [out_dim]
//   3. output = x + alpha/rank * high
//
// This version handles the common case where x is a single vector [in_dim]
// and output is [out_dim] (out_dim == in_dim for most LoRA applications)
struct LoraParams {
    in_dim: u32,
    out_dim: u32,
    rank: u32,
    alpha: f32,
}

@group(0) @binding(0) var<storage, read> lora_x: array<f32>;
@group(0) @binding(1) var<storage, read> lora_down: array<f32>;  // [rank, in_dim]
@group(0) @binding(2) var<storage, read> lora_up: array<f32>;    // [out_dim, rank]
@group(0) @binding(3) var<storage, read_write> lora_output: array<f32>;
@group(0) @binding(4) var<uniform> lora_params: LoraParams;

// Shared memory for the low-rank intermediate result
var<workgroup> lora_low: array<f32, 256>;  // max rank 256

@compute @workgroup_size(256)
fn lora_apply_kernel(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(workgroup_id) wg_id: vec3<u32>,
) {
    let tid = lid.x;
    let out_idx = gid.x;

    // Step 1: compute low = down @ x  (done cooperatively by the workgroup)
    if (tid < lora_params.rank) {
        var dot: f32 = 0.0;
        for (var i = 0u; i < lora_params.in_dim; i++) {
            dot += lora_down[tid * lora_params.in_dim + i] * lora_x[i];
        }
        lora_low[tid] = dot;
    }
    workgroupBarrier();

    // Step 2: compute high = up @ low, and output = x + alpha/rank * high
    if (out_idx < lora_params.out_dim) {
        var high: f32 = 0.0;
        for (var r = 0u; r < lora_params.rank; r++) {
            high += lora_up[out_idx * lora_params.rank + r] * lora_low[r];
        }
        let scale = lora_params.alpha / f32(lora_params.rank);
        lora_output[out_idx] = lora_x[out_idx] + scale * high;
    }
}

// === Kronecker Product ===
// C = A kron B
// If A is [m, n] and B is [p, q], then C is [m*p, n*q]
// C[i*p + s, j*q + t] = A[i, j] * B[s, t]
// Each thread computes one element of C
struct KronParams {
    m: u32,
    n: u32,
    p: u32,
    q: u32,
    total_output: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(0) var<storage, read> kron_a: array<f32>;
@group(0) @binding(1) var<storage, read> kron_b: array<f32>;
@group(0) @binding(2) var<storage, read_write> kron_output: array<f32>;
@group(0) @binding(3) var<uniform> kron_params: KronParams;

@compute @workgroup_size(256)
fn kron_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= kron_params.total_output) { return; }

    let out_cols = kron_params.n * kron_params.q;
    let row = idx / out_cols;
    let col = idx % out_cols;

    // Decompose into A and B indices
    let i = row / kron_params.p;
    let s = row % kron_params.p;
    let j = col / kron_params.q;
    let t = col % kron_params.q;

    let a_val = kron_a[i * kron_params.n + j];
    let b_val = kron_b[s * kron_params.q + t];

    kron_output[idx] = a_val * b_val;
}
