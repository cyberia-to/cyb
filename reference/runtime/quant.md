# Quantization formats

Exact bit layouts for quantized weights. Matches llama.cpp GGUF
conventions so imports are byte-compatible.

## Overview

| Format | Bits/value | Block size | Bytes/block | Error |
|---|---|---|---|---|
| F32 | 32 | - | 4 | 0 |
| F16 | 16 | - | 2 | 6e-4 |
| BF16 | 16 | - | 2 | 4e-3 |
| Q8_0 | 8.5 | 32 | 34 | 6e-4 |
| Q4_0 | 4.5 | 32 | 18 | 2e-3 |
| Q2_K | 2.625 | 256 | 84 | 2e-2 |
| Q3_K | 3.4375 | 256 | 110 | 8e-3 |
| Q4_K | 4.5 | 256 | 144 | 2e-3 |
| Q5_K | 5.5 | 256 | 176 | 1e-3 |
| Q6_K | 6.5625 | 256 | 210 | 5e-4 |
| Ternary | 1.58 | - | (4 values / byte) | 3e-2 |

Error is RMSE vs F32 reference on Llama-2-7B weights, indicative.

## Endianness

All multi-byte integer and float fields are **little-endian**. Same
as GGUF, x86, ARM, WGSL.

## F32

IEEE 754 single-precision.

## F16

IEEE 754 half-precision: 1 sign bit, 5 exponent, 10 mantissa.
Range ±65504. Gradual underflow via denormals.

## BF16

"Brain floating point 16": 1 sign bit, 8 exponent (same as F32), 7
mantissa. Same dynamic range as F32, lower precision.

**Import rule:** BF16 weights convert to F16 at import time
([import.md](import.md)). Runtime does not carry BF16.

## Q8_0 — 8-bit blocks of 32

Symmetric quantization, per-block scale.

Block layout (34 bytes):

```
struct block_q8_0 {
    f16 d;            // 2 bytes — scale
    i8 qs[32];        // 32 bytes — quantized values [-128, 127]
}
```

Dequant:
```
x[i] = d * qs[i]
```

Quant (F32 → Q8_0):
```
amax = max(abs(x[i]))
d = amax / 127.0
qs[i] = round(x[i] / d)   # clip to [-128, 127]
```

## Q4_0 — 4-bit blocks of 32

Symmetric, two nibbles per byte.

Block layout (18 bytes):

```
struct block_q4_0 {
    f16 d;            // 2 bytes — scale
    u8 qs[16];        // 16 bytes — 32 nibbles
}
```

Nibble encoding: bottom nibble stores x[j] for j in 0..16, top nibble
for j in 16..32.

```
qs[j] = nibble + 8         # nibble in [-8, 7]
x[j] = d * (nibble_of(qs[j >= 16], j % 16) - 8)
```

Dequant:
```
for j in 0..16:
  x[j]      = d * ((qs[j] & 0x0F) - 8)
  x[j + 16] = d * ((qs[j] >> 4)   - 8)
```

**Note:** llama.cpp's Q4_0 interleaves lo/hi differently in some
older versions. Canonical layout above (lo → first half, hi → second
half) matches current GGUF Q4_0.

## K-quants (Q2_K, Q3_K, Q4_K, Q5_K, Q6_K)

Superblock of 256 values, divided into 8 or 16 sub-blocks with
6-bit scales and optional 6-bit mins. Higher precision than legacy
Q4_0 at similar size.

### Q4_K — 144 bytes per 256 values

```
struct block_q4_K {
    f16 d;              // 2 bytes — superblock scale
    f16 dmin;           // 2 bytes — superblock min offset
    u8 scales[12];      // 12 bytes — packed 6-bit: 8 scales + 8 mins
    u8 qs[128];         // 128 bytes — 256 nibbles
}
```

Sub-block structure: 8 sub-blocks of 32 values each. Each sub-block j
has a 6-bit scale `s[j]` and a 6-bit min `m[j]`.

Scale/min extraction from `scales[12]` for sub-block `j` in 0..8:

```
if j < 4:
    s[j] = scales[j] & 0x3F                        # low 6 bits
    m[j] = scales[j + 4] & 0x3F
else:
    s[j] = (scales[j + 4] & 0x0F) | ((scales[j - 4] >> 6) << 4)
    m[j] = (scales[j + 4] >> 4)   | ((scales[j]     >> 6) << 4)
```

Nibbles: `qs[128]` holds 256 nibbles. For sub-block j (0..8), values
are at positions `[j*32, j*32+32)`. Two nibbles per byte:

```
qs_byte_off = (j / 2) * 32          # byte range for a pair of sub-blocks
is_upper = j % 2                    # low nibble (0) or high nibble (1)

for l in 0..32:
    byte = qs[qs_byte_off + l]
    nibble = (byte >> (is_upper * 4)) & 0x0F
    x[j*32 + l] = d * s[j] * nibble - dmin * m[j]
```

**Critical:** dequant subtracts `dmin * m[j]`, not adds. This is the
asymmetric min offset. Common bug: forgetting the min term entirely
(matches Q4_0 behavior but produces wrong Q4_K values).

### Q5_K — 176 bytes per 256 values

Same superblock structure as Q4_K, plus 1 extra high bit per nibble
packed in 32 extra bytes:

```
struct block_q5_K {
    f16 d;
    f16 dmin;
    u8 scales[12];
    u8 qh[32];          // high bits (1 bit per value)
    u8 qs[128];         // low 4 bits
}
```

5-bit quant value: `q = (qh_bit << 4) | qs_nibble`, where `qh_bit`
is the bit for this value pulled from `qh[32]` (32 bytes = 256 bits).

### Q6_K — 210 bytes per 256 values

8 sub-blocks of 32, 6-bit values:

```
struct block_q6_K {
    u8 ql[128];         // low 4 bits of each value
    u8 qh[64];          // high 2 bits of each value
    i8 scales[16];      // 16 sub-block scales (i8)
    f16 d;              // superblock scale
}
```

Note: sub-blocks of 16 values here, not 32. Scale array is length 16.

Dequant:
```
for j in 0..16:
    s = scales[j]
    for l in 0..16:
        idx = j * 16 + l
        q = ((ql[bit position] & 0x0F) | ((qh[bit position] & 0x03) << 4)) - 32
        x[idx] = d * s * q
```

Full bit-packing details follow GGUF's `dequantize_row_q6_K`.

### Q3_K — 110 bytes per 256 values

3-bit with 6-bit scales:

```
struct block_q3_K {
    u8 hmask[32];       // high bits (1 per value)
    u8 qs[64];          // low 2 bits
    u8 scales[12];      // packed 6-bit scales
    f16 d;
}
```

### Q2_K — 84 bytes per 256 values

2-bit with 4-bit scales + 4-bit mins:

```
struct block_q2_K {
    u8 scales[16];      // 4-bit scale + 4-bit min per sub-block of 16
    u8 qs[64];          // 256 values × 2 bits
    f16 d;
    f16 dmin;
}
```

## Ternary (BitNet 1.58-bit)

Four ternary values per byte, 2 bits each.

Encoding:
```
00 → -1
01 →  0
10 → +1
11 →  0  (unused)
```

Dequant:
```
for each byte:
    for i in 0..4:
        bits = (byte >> (i * 2)) & 0x03
        v = {-1, 0, +1, 0}[bits]
        x[4*block_idx + i] = scale * v
```

Scale is a per-tensor f32 separately stored.

## Format dispatch

A matmul with quantized weights runs a format-specific kernel. The
backend (wgpu, honeycrisp) has one kernel per supported format. Missing
format → error at dispatch ("backend does not implement Q3_K matmul").

Conversion between formats (e.g. Q4_0 → Q4_K for storage normalization)
happens only at import time ([import.md](import.md)), not at runtime.

## Tolerance

Dequant tolerance is the error bound of the format itself:

| Format | Max abs error / max abs value |
|---|---|
| Q8_0 | 1/256 ≈ 4e-3 |
| Q4_0 | 1/32 ≈ 3e-2 |
| Q4_K | 1/64 ≈ 1.5e-2 |
| Q5_K | 1/128 ≈ 8e-3 |
| Q6_K | 1/256 ≈ 4e-3 |
| Ternary | 1.0 (coarse) |

These are per-weight, not per-output. Output error of a matmul
depends on accumulation length (√K scaling). Expect 10-100× the
per-weight bound on final activations.
