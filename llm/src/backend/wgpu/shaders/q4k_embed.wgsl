// Q4_K embed — dequant one row of a Q4_K embed table on GPU
//
// Instead of dequanting the entire vocab×hidden embed table to f32 (5.6GB for gemma),
// keep the raw Q4_K bytes on GPU and dequant one row per token on demand.
//
// Q4_K superblock: 144 bytes = 256 values
// Layout: d(f16=2) + dmin(f16=2) + scales(12) + qs(128)
//
// Each row has (hidden_size/256) superblocks.
// For token_id T, superblocks start at byte offset T * blocks_per_row * 144.

struct EmbedParams {
    token_id: u32,
    hidden_size: u32,
    blocks_per_row: u32,
    _pad: u32,
}

@group(0) @binding(0) var<storage, read> embed_table: array<u32>; // raw Q4_K bytes
@group(0) @binding(1) var<storage, read_write> output: array<f32>; // [hidden_size]
@group(0) @binding(2) var<uniform> params: EmbedParams;

const BLOCK_VALS: u32 = 256u;
const BLOCK_U32S: u32 = 36u; // 144 bytes / 4

// Extract f16 from a u32 word: which=0 → low 16 bits, which=1 → high 16 bits
fn u32_to_f16(word: u32, which: u32) -> f32 {
    let bits = (word >> (which * 16u)) & 0xFFFFu;
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

// Unpack 6-bit scale and min for sub-block j (0..7)
fn get_scale_min_k4(j: u32, s0: u32, s1: u32, s2: u32) -> vec2<f32> {
    let b = array<u32, 12>(
        s0 & 0xFFu, (s0 >> 8u) & 0xFFu, (s0 >> 16u) & 0xFFu, (s0 >> 24u) & 0xFFu,
        s1 & 0xFFu, (s1 >> 8u) & 0xFFu, (s1 >> 16u) & 0xFFu, (s1 >> 24u) & 0xFFu,
        s2 & 0xFFu, (s2 >> 8u) & 0xFFu, (s2 >> 16u) & 0xFFu, (s2 >> 24u) & 0xFFu,
    );

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
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= params.hidden_size) { return; }

    // Which superblock does this element belong to?
    let blk_idx = idx / BLOCK_VALS;
    let val_in_blk = idx % BLOCK_VALS;

    // Offset into embed_table (u32 array) for this token's row
    let row_base = params.token_id * params.blocks_per_row * BLOCK_U32S;
    let blk_base = row_base + blk_idx * BLOCK_U32S;

    // Read d and dmin
    let d_dmin_word = embed_table[blk_base];
    let d = u32_to_f16(d_dmin_word, 0u);
    let dmin = u32_to_f16(d_dmin_word, 1u);

    // Read scale bytes (3 u32s)
    let s0 = embed_table[blk_base + 1u];
    let s1 = embed_table[blk_base + 2u];
    let s2 = embed_table[blk_base + 3u];

    // Which group (0..3) and sub-position within group
    let grp = val_in_blk / 64u;
    let pos_in_grp = val_in_blk % 64u;

    // Two sub-blocks of 32 per group: low nibble (0..31), high nibble (32..63)
    let is_high = pos_in_grp / 32u;
    let pos_in_sub = pos_in_grp % 32u;

    let sm = get_scale_min_k4(grp * 2u + is_high, s0, s1, s2);
    let scale = d * sm.x;
    let min = dmin * sm.y;

    // Read the quantized nibble from qs region
    // qs starts at byte 16 within superblock = u32 offset 4
    let qs_byte_idx = grp * 32u + pos_in_sub;
    let qs_u32_start = blk_base + 4u;
    let u32_idx = qs_byte_idx / 4u;
    let byte_in_u32 = qs_byte_idx % 4u;
    let word = embed_table[qs_u32_start + u32_idx];
    let byte_val = (word >> (byte_in_u32 * 8u)) & 0xFFu;

    var nibble: u32;
    if (is_high == 0u) {
        nibble = byte_val & 0xFu;
    } else {
        nibble = byte_val >> 4u;
    }

    output[idx] = scale * f32(nibble) - min;
}
