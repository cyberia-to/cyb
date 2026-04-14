// Q2_K VecMat — native K-quant dequant
//
// Q2_K superblock: 84 bytes = 256 values
// Layout: scales(16) + qs(64) + d(f16=2) + dmin(f16=2) = 84 bytes
//
// qs[64]: 2 bits per value (256 * 2 bits = 512 bits = 64 bytes)
// scales[16]: 4-bit packed (16 sub-blocks of 16 values, each byte = scale|min)
//   For sub-block j: scale = scales[j] & 0xF, min = scales[j] >> 4
// d, dmin: f16 at offsets 80 and 82
//
// Dequant: val = d * sc * q2_value - dmin * m
//   where q2_value = (qs_byte >> (2*position_in_byte)) & 3
//
// Weight buffer layout: row-major, each row has (K/256) superblocks.

enable subgroups;

const WG_SIZE: u32 = 256u;
const NR: u32 = 4u;
const BLOCK_VALS: u32 = 256u;   // values per Q2_K superblock
const BLOCK_BYTES: u32 = 84u;   // bytes per Q2_K superblock

struct Params {
    n: u32,         // output dimension (rows)
    k: u32,         // input dimension (cols)
    blocks_per_row: u32,  // K / 256
    _pad: u32,
}

@group(0) @binding(0) var<storage, read> activation: array<f32>;
@group(0) @binding(1) var<storage, read> weights: array<u32>;  // raw Q2_K bytes as u32
@group(0) @binding(2) var<storage, read_write> output: array<f32>;
@group(0) @binding(3) var<uniform> params: Params;

var<workgroup> wg_partial: array<f32, 32>;

fn read_byte(byte_offset: u32) -> u32 {
    let u32_idx = byte_offset / 4u;
    let byte_in_u32 = byte_offset % 4u;
    return (weights[u32_idx] >> (byte_in_u32 * 8u)) & 0xFFu;
}

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

    var blk_idx = tid;
    while (blk_idx < bpr) {
        let col_base = blk_idx * BLOCK_VALS;

        for (var r = 0u; r < NR; r++) {
            let row = base_row + r;
            if (row >= params.n) { break; }

            let blk_byte_base = row * bytes_per_row + blk_idx * BLOCK_BYTES;

            // Layout: scales(16) + qs(64) + d(f16=2) + dmin(f16=2) = 84
            let sc_base = blk_byte_base;
            let qs_base = blk_byte_base + 16u;
            let d = read_f16_at(blk_byte_base + 80u);
            let dmin = read_f16_at(blk_byte_base + 82u);

            var acc = 0.0f;

            // 16 sub-blocks of 16 values each
            for (var sb = 0u; sb < 16u; sb++) {
                let sc_byte = read_byte(sc_base + sb);
                let sc = f32(sc_byte & 0xFu);
                let m = f32(sc_byte >> 4u);
                let ds = d * sc;
                let dm = dmin * m;
                let val_base = sb * 16u;

                for (var l = 0u; l < 16u; l++) {
                    let j = val_base + l;
                    // 4 values per byte, 2 bits each
                    let qs_byte_idx = j / 4u;
                    let qs_shift = (j % 4u) * 2u;
                    let q2 = (read_byte(qs_base + qs_byte_idx) >> qs_shift) & 3u;

                    acc += (ds * f32(q2) - dm) * activation[col_base + j];
                }
            }

            sums[r] += acc;
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
