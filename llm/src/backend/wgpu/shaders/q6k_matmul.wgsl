// Q6_K VecMat — native K-quant dequant
//
// Q6_K superblock: 210 bytes = 256 values
// Layout: ql(128B) + qh(64B) + scales(16B int8) + d(f16=2B) = 210 bytes
//
// Each value j is 6 bits:
//   ql_byte = ql[j/2], ql_nib = low nib if j even, high nib if j odd
//   qh_bit  = (qh[j/4] >> ((j%4)*2)) & 3
//   q = (qh_bit << 4) | ql_nib
//   scale = int8(scales[j/16])
//   value = d * scale * (q - 32)
//
// Weight buffer layout: row-major, each row has (K/256) superblocks.
// Total bytes per row = (K/256) * 210.

enable subgroups;

const WG_SIZE: u32 = 256u;
const NR: u32 = 4u;
const BLOCK_VALS: u32 = 256u;   // values per Q6_K superblock
const BLOCK_BYTES: u32 = 210u;  // bytes per Q6_K superblock

struct Params {
    n: u32,         // output dimension (rows)
    k: u32,         // input dimension (cols)
    blocks_per_row: u32,  // K / 256
    _pad: u32,
}

@group(0) @binding(0) var<storage, read> activation: array<f32>;
@group(0) @binding(1) var<storage, read> weights: array<u32>;  // raw Q6_K bytes as u32
@group(0) @binding(2) var<storage, read_write> output: array<f32>;
@group(0) @binding(3) var<uniform> params: Params;

var<workgroup> wg_partial: array<f32, 32>;

// Read a byte from the u32 weights array at a given byte offset
fn read_byte(byte_offset: u32) -> u32 {
    let u32_idx = byte_offset / 4u;
    let byte_in_u32 = byte_offset % 4u;
    return (weights[u32_idx] >> (byte_in_u32 * 8u)) & 0xFFu;
}

// Read f16 at a given byte offset (may straddle u32 boundary)
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

// Read int8 scale: interpret byte as signed int8 value
fn read_scale(byte_offset: u32) -> f32 {
    let raw = read_byte(byte_offset);
    if (raw >= 128u) {
        return f32(i32(raw) - 256);
    }
    return f32(raw);
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

            // d at bytes [208..209]
            let d = read_f16_at(blk_byte_base + 208u);
            // ql at [0..127], qh at [128..191], scales at [192..207]
            let ql_base = blk_byte_base;
            let qh_base = blk_byte_base + 128u;
            let sc_base = blk_byte_base + 192u;

            var acc = 0.0f;

            // Process 256 values using the linear j formula
            // j: ql_byte = ql[j/2], nib = low if j%2==0, high if j%2==1
            //    qh_bit = (qh[j/4] >> ((j%4)*2)) & 3
            //    scale = int8(scales[j/16])
            //    q = (qh_bit << 4) | ql_nib,  value = d * scale * (q - 32)

            // Process 4 values at a time sharing the same qh byte
            for (var j4 = 0u; j4 < 64u; j4++) {
                let qh_byte = read_byte(qh_base + j4);
                let j_base = j4 * 4u;

                // j = j_base + 0
                let j0 = j_base;
                let ql_byte0 = read_byte(ql_base + j0 / 2u);
                let ql_nib0 = select(ql_byte0 >> 4u, ql_byte0 & 0xFu, j0 % 2u == 0u);
                let qh_bit0 = (qh_byte >> 0u) & 3u;
                let q0 = i32((qh_bit0 << 4u) | ql_nib0) - 32;
                let sc0 = read_scale(sc_base + j0 / 16u);
                acc += d * sc0 * f32(q0) * activation[col_base + j0];

                // j = j_base + 1
                let j1 = j_base + 1u;
                let ql_byte1 = read_byte(ql_base + j1 / 2u);
                let ql_nib1 = select(ql_byte1 >> 4u, ql_byte1 & 0xFu, j1 % 2u == 0u);
                let qh_bit1 = (qh_byte >> 2u) & 3u;
                let q1 = i32((qh_bit1 << 4u) | ql_nib1) - 32;
                let sc1 = read_scale(sc_base + j1 / 16u);
                acc += d * sc1 * f32(q1) * activation[col_base + j1];

                // j = j_base + 2
                let j2 = j_base + 2u;
                let ql_byte2 = read_byte(ql_base + j2 / 2u);
                let ql_nib2 = select(ql_byte2 >> 4u, ql_byte2 & 0xFu, j2 % 2u == 0u);
                let qh_bit2 = (qh_byte >> 4u) & 3u;
                let q2 = i32((qh_bit2 << 4u) | ql_nib2) - 32;
                let sc2 = read_scale(sc_base + j2 / 16u);
                acc += d * sc2 * f32(q2) * activation[col_base + j2];

                // j = j_base + 3
                let j3 = j_base + 3u;
                let ql_byte3 = read_byte(ql_base + j3 / 2u);
                let ql_nib3 = select(ql_byte3 >> 4u, ql_byte3 & 0xFu, j3 % 2u == 0u);
                let qh_bit3 = (qh_byte >> 6u) & 3u;
                let q3 = i32((qh_bit3 << 4u) | ql_nib3) - 32;
                let sc3 = read_scale(sc_base + j3 / 16u);
                acc += d * sc3 * f32(q3) * activation[col_base + j3];
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
