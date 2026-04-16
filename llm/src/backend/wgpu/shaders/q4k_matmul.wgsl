// Q4_K VecMat — native K-quant dequant, no requant to Q4_0
//
// Q4_K superblock: 144 bytes = 256 values
// Layout: d(f16=2) + dmin(f16=2) + scales(12) + qs(128)
//
// The raw bytes are uploaded as array<u32> for GPU access.
// Each superblock = 36 u32s (144 bytes / 4).
//
// Weight buffer layout: row-major, each row has (K/256) superblocks.
// Total u32s per row = (K/256) * 36.

enable subgroups;

const WG_SIZE: u32 = 256u;
const NR: u32 = 4u;
const BLOCK_VALS: u32 = 256u;  // values per Q4_K superblock
const BLOCK_U32S: u32 = 36u;   // u32s per Q4_K superblock (144/4)

struct Params {
    n: u32,         // output dimension (rows)
    k: u32,         // input dimension (cols)
    blocks_per_row: u32,  // K / 256
    _pad: u32,
}

@group(0) @binding(0) var<storage, read> activation: array<f32>;
@group(0) @binding(1) var<storage, read> weights: array<u32>;  // raw Q4_K bytes as u32
@group(0) @binding(2) var<storage, read_write> output: array<f32>;
@group(0) @binding(3) var<uniform> params: Params;

var<workgroup> wg_partial: array<f32, 32>;

// Extract f16 from a u32 word: which=0 → low 16 bits, which=1 → high 16 bits
fn u32_to_f16(word: u32, which: u32) -> f32 {
    let bits = (word >> (which * 16u)) & 0xFFFFu;
    // IEEE 754 half-precision decode
    let sign = f32(bits >> 15u);
    let exp = (bits >> 10u) & 0x1Fu;
    let mant = bits & 0x3FFu;

    if (exp == 0u) {
        // Subnormal or zero
        let val = f32(mant) / 1024.0 * (1.0 / 16384.0);
        return select(val, -val, sign > 0.5);
    }
    if (exp == 31u) {
        // Inf/NaN — treat as 0 for safety
        return 0.0;
    }
    let val = (1.0 + f32(mant) / 1024.0) * pow(2.0, f32(i32(exp) - 15));
    return select(val, -val, sign > 0.5);
}

// Unpack 6-bit scale and min for sub-block j (0..7)
// scales_raw is 12 bytes = 3 u32s at offset `base` in the weights array
fn get_scale_min_k4(j: u32, s0: u32, s1: u32, s2: u32) -> vec2<f32> {
    // Reconstruct the 12 scale bytes from 3 u32s
    // s0 = bytes[0..3], s1 = bytes[4..7], s2 = bytes[8..11]
    var sc: u32;
    var mn: u32;

    // Extract individual bytes
    let b = array<u32, 12>(
        s0 & 0xFFu, (s0 >> 8u) & 0xFFu, (s0 >> 16u) & 0xFFu, (s0 >> 24u) & 0xFFu,
        s1 & 0xFFu, (s1 >> 8u) & 0xFFu, (s1 >> 16u) & 0xFFu, (s1 >> 24u) & 0xFFu,
        s2 & 0xFFu, (s2 >> 8u) & 0xFFu, (s2 >> 16u) & 0xFFu, (s2 >> 24u) & 0xFFu,
    );

    if (j < 4u) {
        sc = b[j] & 63u;
        mn = b[j + 4u] & 63u;
    } else {
        sc = (b[j + 4u] & 0xFu) | ((b[j - 4u] >> 6u) << 4u);
        mn = (b[j + 4u] >> 4u) | ((b[j] >> 6u) << 4u);
    }

    return vec2<f32>(f32(sc), f32(mn));
}

// Extract a nibble (4-bit value) from the qs region of a superblock
// qs starts at byte 16 within the superblock = u32 offset 4
// qs_byte_idx: byte index within the 128-byte qs region (0..127)
// nibble: 0 = low, 1 = high
fn get_qs_nibble(row_base: u32, blk_offset: u32, qs_byte_idx: u32, nibble: u32) -> u32 {
    let qs_u32_start = blk_offset + 4u; // skip d(2)+dmin(2)+scales(12) = 16 bytes = 4 u32s
    let u32_idx = qs_byte_idx / 4u;
    let byte_in_u32 = qs_byte_idx % 4u;
    let word = weights[row_base + qs_u32_start + u32_idx];
    let byte_val = (word >> (byte_in_u32 * 8u)) & 0xFFu;
    if (nibble == 0u) {
        return byte_val & 0xFu;
    } else {
        return byte_val >> 4u;
    }
}

@compute @workgroup_size(256)
fn main(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(num_workgroups) num_wg: vec3<u32>,
    @builtin(subgroup_invocation_id) sg_id: u32,
    @builtin(subgroup_size) sg_size: u32,
) {
    let wg_idx = wg_id.y * num_wg.x + wg_id.x;
    let base_row = wg_idx * NR;
    let tid = local_id.x;
    let sg_idx = tid / sg_size;
    let num_sgs = WG_SIZE / sg_size;

    var sums: array<f32, 4>;
    sums[0] = 0.0; sums[1] = 0.0; sums[2] = 0.0; sums[3] = 0.0;

    let bpr = params.blocks_per_row;
    let u32s_per_row = bpr * BLOCK_U32S;

    // Each thread processes a subset of superblocks
    var blk_idx = tid;
    while (blk_idx < bpr) {
        let col_base = blk_idx * BLOCK_VALS;
        let blk_u32_offset = blk_idx * BLOCK_U32S;

        for (var r = 0u; r < NR; r++) {
            let row = base_row + r;
            if (row >= params.n) { break; }

            let row_base = row * u32s_per_row;
            let blk_base = row_base + blk_u32_offset;

            // Read d and dmin (first u32 = d_low16 | dmin_high16)
            let d_dmin_word = weights[blk_base];
            let d = u32_to_f16(d_dmin_word, 0u);
            let dmin = u32_to_f16(d_dmin_word, 1u);

            // Read scale bytes (3 u32s at offset 1..3)
            let s0 = weights[blk_base + 1u];
            let s1 = weights[blk_base + 2u];
            let s2 = weights[blk_base + 3u];

            // 4 groups of 64 values, each group = 2 sub-blocks of 32
            for (var grp = 0u; grp < 4u; grp++) {
                let sm1 = get_scale_min_k4(grp * 2u, s0, s1, s2);
                let sm2 = get_scale_min_k4(grp * 2u + 1u, s0, s1, s2);

                let d1 = d * sm1.x;
                let m1 = dmin * sm1.y;
                let d2 = d * sm2.x;
                let m2 = dmin * sm2.y;

                let qs_byte_off = grp * 32u;
                let k_off = col_base + grp * 64u;

                var acc1: f32 = 0.0;
                var acc2: f32 = 0.0;
                var xsum1: f32 = 0.0;
                var xsum2: f32 = 0.0;

                // Process 32 byte pairs (low nibble → sub-block 1, high → sub-block 2)
                // Read 8 u32s = 32 bytes from qs region
                let qs_u32_start = blk_base + 4u + (qs_byte_off / 4u);

                for (var qw = 0u; qw < 8u; qw++) {
                    let word = weights[qs_u32_start + qw];

                    // Each u32 = 4 qs bytes
                    for (var bi = 0u; bi < 4u; bi++) {
                        let l = qw * 4u + bi;
                        let byte_val = (word >> (bi * 8u)) & 0xFFu;
                        let lo = byte_val & 0xFu;
                        let hi = byte_val >> 4u;

                        let x0 = activation[k_off + l];
                        let x1 = activation[k_off + 32u + l];

                        acc1 += x0 * f32(lo);
                        xsum1 += x0;
                        acc2 += x1 * f32(hi);
                        xsum2 += x1;
                    }
                }

                sums[r] += d1 * acc1 - m1 * xsum1 + d2 * acc2 - m2 * xsum2;
            }
        }

        blk_idx += WG_SIZE;
    }

    // Subgroup + cross-subgroup reduction
    for (var r = 0u; r < NR; r++) {
        sums[r] = subgroupAdd(sums[r]);
    }

    if (sg_id == 0u) {
        for (var r = 0u; r < NR; r++) {
            wg_partial[sg_idx * NR + r] = sums[r];
        }
    }
    workgroupBarrier();

    if (sg_idx == 0u && sg_id < num_sgs) {
        for (var r = 0u; r < NR; r++) {
            sums[r] = wg_partial[sg_id * NR + r];
        }
        for (var r = 0u; r < NR; r++) {
            sums[r] = subgroupAdd(sums[r]);
        }
        if (sg_id == 0u) {
            for (var r = 0u; r < NR; r++) {
                let row = base_row + r;
                if (row < params.n) {
                    output[row] = sums[r];
                }
            }
        }
    }
}
