// Q5_K VecMat — native K-quant dequant
//
// Q5_K superblock: 176 bytes = 256 values
// Layout: d(f16=2) + dmin(f16=2) + scales(12) + qh(32) + qs(128) = 176 bytes
//
// Like Q4_K but with a 5th bit per value stored in qh[32].
// qs[128]: low 4 bits of each value (same nibble layout as Q4_K)
// qh[32]: the 5th bit for each of the 256 values (256 bits = 32 bytes)
// scales[12]: same 6-bit packed format as Q4_K (get_scale_min_k4)
//
// Dequant for value at position p within superblock:
//   nibble = qs[p/2] low or high nib
//   high_bit = (qh[p/8] >> (p%8)) & 1
//   q = nibble | (high_bit << 4)
//   value = d * sc * q - dmin * m
//
// Weight buffer layout: row-major, each row has (K/256) superblocks.
// Total bytes per row = (K/256) * 176.

enable subgroups;

const WG_SIZE: u32 = 256u;
const NR: u32 = 4u;
const BLOCK_VALS: u32 = 256u;   // values per Q5_K superblock
const BLOCK_BYTES: u32 = 176u;  // bytes per Q5_K superblock

struct Params {
    n: u32,         // output dimension (rows)
    k: u32,         // input dimension (cols)
    blocks_per_row: u32,  // K / 256
    _pad: u32,
}

@group(0) @binding(0) var<storage, read> activation: array<f32>;
@group(0) @binding(1) var<storage, read> weights: array<u32>;  // raw Q5_K bytes as u32
@group(0) @binding(2) var<storage, read_write> output: array<f32>;
@group(0) @binding(3) var<uniform> params: Params;

var<workgroup> wg_partial: array<f32, 32>;

// Read a byte from the u32 weights array at a given byte offset
fn read_byte(byte_offset: u32) -> u32 {
    let u32_idx = byte_offset / 4u;
    let byte_in_u32 = byte_offset % 4u;
    return (weights[u32_idx] >> (byte_in_u32 * 8u)) & 0xFFu;
}

// Read f16 at a given byte offset
fn read_f16_at(byte_offset: u32) -> f32 {
    let lo = read_byte(byte_offset);
    let hi = read_byte(byte_offset + 1u);
    let bits = lo | (hi << 8u);
    return decode_f16(bits);
}

fn decode_f16(bits: u32) -> f32 {
    let sign = f32(bits >> 15u);
    let exp = (bits >> 10u) & 0x1Fu;
    let mant = bits & 0x3FFu;

    if (exp == 0u) {
        let val = f32(mant) / 1024.0 * (1.0 / 16384.0);
        return select(val, -val, sign > 0.5);
    }
    if (exp == 31u) {
        return 0.0;
    }
    let val = (1.0 + f32(mant) / 1024.0) * pow(2.0, f32(i32(exp) - 15));
    return select(val, -val, sign > 0.5);
}

// Unpack 6-bit scale and min for sub-block j (0..7), same as Q4_K
fn get_scale_min_k4(j: u32, base: u32) -> vec2<f32> {
    var b: array<u32, 12>;
    for (var i = 0u; i < 12u; i++) {
        b[i] = read_byte(base + i);
    }

    var sc: u32;
    var mn: u32;
    if (j < 4u) {
        sc = b[j] & 63u;
        mn = b[j + 4u] & 63u;
    } else {
        sc = (b[j + 4u] & 0xFu) | ((b[j - 4u] >> 6u) << 4u);
        mn = (b[j + 4u] >> 4u) | ((b[j] >> 6u) << 4u);
    }
    return vec2<f32>(f32(sc), f32(mn));
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
    let bytes_per_row = bpr * BLOCK_BYTES;

    // Each thread processes a subset of superblocks
    var blk_idx = tid;
    while (blk_idx < bpr) {
        let col_base = blk_idx * BLOCK_VALS;

        for (var r = 0u; r < NR; r++) {
            let row = base_row + r;
            if (row >= params.n) { break; }

            let blk_byte_base = row * bytes_per_row + blk_idx * BLOCK_BYTES;

            // Layout: d(2) + dmin(2) + scales(12) + qh(32) + qs(128) = 176
            let d = read_f16_at(blk_byte_base);
            let dmin = read_f16_at(blk_byte_base + 2u);
            let sc_base = blk_byte_base + 4u;
            let qh_base = blk_byte_base + 16u;
            let qs_base = blk_byte_base + 48u;

            // 4 groups of 64 values, each group = 2 sub-blocks of 32
            for (var grp = 0u; grp < 4u; grp++) {
                let sm1 = get_scale_min_k4(grp * 2u, sc_base);
                let sm2 = get_scale_min_k4(grp * 2u + 1u, sc_base);

                let d1 = d * sm1.x;
                let m1 = dmin * sm1.y;
                let d2 = d * sm2.x;
                let m2 = dmin * sm2.y;

                let qs_off = grp * 32u;
                let k_off = col_base + grp * 64u;

                var acc1: f32 = 0.0;
                var acc2: f32 = 0.0;
                var xsum1: f32 = 0.0;
                var xsum2: f32 = 0.0;

                for (var l = 0u; l < 32u; l++) {
                    let qs_byte = read_byte(qs_base + qs_off + l);
                    let lo_nib = qs_byte & 0xFu;
                    let hi_nib = qs_byte >> 4u;

                    // 5th bit for low position: value index = grp*64 + l
                    let lo_idx = grp * 64u + l;
                    let lo_qh = (read_byte(qh_base + lo_idx / 8u) >> (lo_idx % 8u)) & 1u;
                    let lo_q = lo_nib | (lo_qh << 4u);

                    // 5th bit for high position: value index = grp*64 + 32 + l
                    let hi_idx = grp * 64u + 32u + l;
                    let hi_qh = (read_byte(qh_base + hi_idx / 8u) >> (hi_idx % 8u)) & 1u;
                    let hi_q = hi_nib | (hi_qh << 4u);

                    let x0 = activation[k_off + l];
                    let x1 = activation[k_off + 32u + l];

                    acc1 += x0 * f32(lo_q);
                    xsum1 += x0;
                    acc2 += x1 * f32(hi_q);
                    xsum2 += x1;
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

    if (sg_idx == 0u) {
        if (sg_id < num_sgs) {
            for (var r = 0u; r < NR; r++) {
                sums[r] = wg_partial[sg_id * NR + r];
            }
        } else {
            for (var r = 0u; r < NR; r++) {
                sums[r] = 0.0;
            }
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
