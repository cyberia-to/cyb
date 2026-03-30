// f16 Vector x Matrix multiply
// activation: [K] f32  (activations stay f32 for precision)
// weight: [N, K] f16   (stored as u32 pairs: two f16 values per u32)
// output: [N] f32
//
// Weights are stored as packed f16 in u32 array:
//   weight_packed[row * (K/2) + i] contains two f16 values at positions 2*i and 2*i+1
// Each workgroup computes one output element using subgroup reduction

enable subgroups;

const WORKGROUP_SIZE: u32 = 256u;

struct Params {
    n: u32,
    k: u32,
    k_half: u32,  // k / 2 (number of u32s per row)
    _pad: u32,
}

@group(0) @binding(0) var<storage, read> f16m_activation: array<f32>;
@group(0) @binding(1) var<storage, read> f16m_weight_packed: array<u32>;  // packed f16 pairs
@group(0) @binding(2) var<storage, read_write> f16m_output: array<f32>;
@group(0) @binding(3) var<uniform> f16m_params: Params;

var<workgroup> f16m_wg_partial: array<f32, 8>;

// Unpack lower f16 from u32 to f32
fn unpack_f16_low(packed: u32) -> f32 {
    let bits = packed & 0xFFFFu;
    let sign = (bits >> 15u) & 1u;
    let exponent = (bits >> 10u) & 0x1Fu;
    let mantissa = bits & 0x3FFu;

    if (exponent == 0u) {
        // Subnormal or zero
        if (mantissa == 0u) {
            if (sign == 1u) { return -0.0; } else { return 0.0; }
        }
        let val = f32(mantissa) / 1024.0 * exp2(-14.0);
        if (sign == 1u) { return -val; } else { return val; }
    } else if (exponent == 31u) {
        // Inf or NaN
        if (mantissa == 0u) {
            if (sign == 1u) { return -3.402823e+38f; } else { return 3.402823e+38f; }
        }
        return 0.0f; // NaN → 0 (WGSL doesn't support NaN literals)
    }

    let val = (1.0 + f32(mantissa) / 1024.0) * exp2(f32(exponent) - 15.0);
    if (sign == 1u) { return -val; } else { return val; }
}

// Unpack upper f16 from u32 to f32
fn unpack_f16_high(packed: u32) -> f32 {
    return unpack_f16_low(packed >> 16u);
}

@compute @workgroup_size(256)
fn main(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(num_workgroups) num_wg: vec3<u32>,
    @builtin(subgroup_invocation_id) sg_id: u32,
    @builtin(subgroup_size) sg_size: u32,
) {
    let row = wg_id.y * num_wg.x + wg_id.x;
    let tid = local_id.x;
    let sg_idx = tid / sg_size;
    let num_sgs = WORKGROUP_SIZE / sg_size;

    if (row >= f16m_params.n) { return; }

    var partial_sum: f32 = 0.0;
    let base = row * f16m_params.k_half;

    // Process two f16 weights per u32
    var i = tid;
    while (i < f16m_params.k_half) {
        let packed = f16m_weight_packed[base + i];
        let w0 = unpack_f16_low(packed);
        let w1 = unpack_f16_high(packed);
        let k_idx = i * 2u;
        partial_sum += f16m_activation[k_idx] * w0;
        if (k_idx + 1u < f16m_params.k) {
            partial_sum += f16m_activation[k_idx + 1u] * w1;
        }
        i += WORKGROUP_SIZE;
    }

    // Subgroup reduction
    partial_sum = subgroupAdd(partial_sum);

    if (sg_id == 0u) {
        f16m_wg_partial[sg_idx] = partial_sum;
    }
    workgroupBarrier();

    if (sg_idx == 0u && sg_id < num_sgs) {
        partial_sum = f16m_wg_partial[sg_id];
        partial_sum = subgroupAdd(partial_sum);
    }

    if (tid == 0u) {
        f16m_output[row] = partial_sum;
    }
}
