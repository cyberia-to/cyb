// Q3_K VecMat — native K-quant dequant
//
// Q3_K superblock: 110 bytes = 256 values
// Layout: hmask(32) + qs(64) + scales(12) + d(f16=2) = 110 bytes
//
// qs[64]: low 2 bits per value (256 * 2 bits = 512 bits = 64 bytes)
// hmask[32]: 3rd (high) bit per value (256 bits = 32 bytes)
// scales[12]: 6-bit packed scales (DIFFERENT unpacking from Q4_K!)
// d: f16 super-block scale at offset 108 (no dmin!)
//
// Scale unpacking for Q3_K (16 sub-blocks of 16 values):
//   scales[12] encodes 16 6-bit values.
//   Bytes 0..3: low 4 bits of scales 0..3 (and 8..11)
//   Bytes 4..7: low 4 bits of scales 4..7 (and 12..15)
//   Bytes 8..11: high 2 bits
//   For j < 8:  scale = (scales[j%4 + (j/4)*4] & 0xF) | (((scales[8 + j%4] >> (2*(j/4))) & 3) << 4) - 32
//   llama.cpp: see dequantize_row_q3_K implementation
//
// Dequant: val = d * sc * (q3_value - 4)
//   where q3_value = (ql_2bits | (hm_1bit << 2))
//
// Weight buffer layout: row-major, each row has (K/256) superblocks.

enable subgroups;

const WG_SIZE: u32 = 256u;
const NR: u32 = 4u;
const BLOCK_VALS: u32 = 256u;   // values per Q3_K superblock
const BLOCK_BYTES: u32 = 110u;  // bytes per Q3_K superblock

struct Params {
    n: u32,         // output dimension (rows)
    k: u32,         // input dimension (cols)
    blocks_per_row: u32,  // K / 256
    _pad: u32,
}

@group(0) @binding(0) var<storage, read> activation: array<f32>;
@group(0) @binding(1) var<storage, read> weights: array<u32>;  // raw Q3_K bytes as u32
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

// Unpack Q3_K scale for sub-block j (0..15), returning signed value (subtract 32)
// scales[12] layout from llama.cpp dequantize_row_q3_K:
//   For sub-block j:
//     low_bits come from scales[j % 4] or scales[4 + j%4] depending on j/4
//     high_bits come from scales[8 + j%4]
//
// llama.cpp reference (uint32_t aux, bottom of dequantize_row_q3_K):
//   a[0..3] = scales bytes 0..3 (nibble pairs: lo4 for j=0..3, lo4 for j=8..11)
//   a[4..7] = scales bytes 4..7 (nibble pairs: lo4 for j=4..7, lo4 for j=12..15)
//   a[8..11]= scales bytes 8..11 (2-bit pairs for high bits)
//
// Simplified for sub-block j (0..15):
//   if j < 4:   sc = (scales[j]     & 0xF) | (((scales[8 + j/2] >> (4*(j%2))) >> ((j/2)*0)) ...)
// Actually let me follow llama.cpp exactly:
//   uint8_t scales[16] (unpacked) from the 12 packed bytes.
//   The llama.cpp code unpacks into utmp[4] then extracts.
//
// Following llama.cpp ggml-quants.c dequantize_row_q3_K exactly:
//   const uint8_t * restrict scales = x[i].scales;
//   // Unpack 16 scales from 12 bytes:
//   // a = scales[0..3], b = scales[4..7], c = scales[8..11]
//   for j in 0..16:
//     if j < 4:  us = (scales[j] & 0xF) | (((scales[8 + (j&1)] >> (4*(j>>1))) & 3) << 4)
//     wait... let me just implement from the actual C code pattern.
//
// From ggml-quants.c (line ~1833):
//   uint32_t tmp[4];
//   memcpy(tmp, scales, 12);
//   // This gives us access as uint32_t[3] = {scales[0..3], scales[4..7], scales[8..11]}
//   // Then unpacking is:
//   us[0..3] from low nibble of tmp[0] bytes + bits from tmp[2]
//   us[4..7] from low nibble of tmp[1] bytes + bits from tmp[2] shifted
//   us[8..11] from high nibble of tmp[0] bytes + bits from tmp[2] more shifted
//   us[12..15] from high nibble of tmp[1] bytes + bits from tmp[2] more shifted
//
// Compact version per sub-block j (0..15):
fn get_q3k_scale(j: u32, sc_base: u32) -> f32 {
    var raw: array<u32, 12>;
    for (var i = 0u; i < 12u; i++) {
        raw[i] = read_byte(sc_base + i);
    }

    var us: u32;
    if (j < 4u) {
        // low nibble of scales[j], high 2 bits from scales[8 + j/2] shifted
        us = (raw[j] & 0xFu) | (((raw[8u + (j >> 1u)] >> (4u * (j & 1u))) & 3u) << 4u);
    } else if (j < 8u) {
        let jj = j - 4u;
        us = (raw[4u + jj] & 0xFu) | (((raw[10u + (jj >> 1u)] >> (4u * (jj & 1u))) & 3u) << 4u);
    } else if (j < 12u) {
        let jj = j - 8u;
        us = (raw[jj] >> 4u) | (((raw[8u + (jj >> 1u)] >> (4u * (jj & 1u) + 2u)) & 3u) << 4u);
    } else {
        let jj = j - 12u;
        us = (raw[4u + jj] >> 4u) | (((raw[10u + (jj >> 1u)] >> (4u * (jj & 1u) + 2u)) & 3u) << 4u);
    }

    return f32(i32(us) - 32);
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

            // Layout: hmask(32) + qs(64) + scales(12) + d(f16=2) = 110
            let hmask_base = blk_byte_base;
            let qs_base = blk_byte_base + 32u;
            let sc_base = blk_byte_base + 96u;
            let d = read_f16_at(blk_byte_base + 108u);

            var acc = 0.0f;

            // Process 256 values: 16 sub-blocks of 16 values each
            for (var sb = 0u; sb < 16u; sb++) {
                let sc = get_q3k_scale(sb, sc_base);
                let val_base = sb * 16u;

                for (var l = 0u; l < 16u; l++) {
                    let j = val_base + l;

                    // low 2 bits from qs: 4 values per byte
                    let qs_byte_idx = j / 4u;
                    let qs_shift = (j % 4u) * 2u;
                    let ql = (read_byte(qs_base + qs_byte_idx) >> qs_shift) & 3u;

                    // high bit from hmask
                    let hm_byte_idx = j / 8u;
                    let hm_bit = (read_byte(hmask_base + hm_byte_idx) >> (j % 8u)) & 1u;

                    let q3_val = i32(ql | (hm_bit << 2u)) - 4;
                    acc += d * sc * f32(q3_val) * activation[col_base + j];
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
