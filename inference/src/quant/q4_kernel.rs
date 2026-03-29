//! Q4 VecMat cubecl kernel — zero-copy on burn's GPU device

use cubecl::prelude::*;

/// Q4 vecmat: dequant + multiply in single GPU kernel
/// Each invocation computes one output row
#[cube(launch_unchecked)]
pub fn q4_vecmat(
    activation: &Tensor<Line<f32>>,
    packed_weights: &Tensor<Line<u32>>,
    scales: &Tensor<Line<f32>>,
    output: &mut Tensor<Line<f32>>,
    k_buf: &Tensor<Line<u32>>,
    bs_buf: &Tensor<Line<u32>>,
) {
    let row = ABSOLUTE_POS;

    let n_val = u32::cast_from(k_buf[0]) as usize; // Actually contains K, but we use output length check
    // Bounds check — more threads than output elements
    if row >= output.len() {
        terminate!();
    }

    let k = u32::cast_from(k_buf[0]) as usize;
    let block_size = u32::cast_from(bs_buf[0]) as usize;
    let num_blocks = k / block_size;
    let half_bs = block_size / 2;
    let u32s_per_row = num_blocks * half_bs / 4;

    let mut sum = 0.0f32;

    let mut ui: usize = 0;
    while ui < u32s_per_row {
        let packed = u32::cast_from(packed_weights[row * u32s_per_row + ui]);

        let mut b: usize = 0;
        while b < 4 {
            let shift = (b * 8) as u32;
            let byte_val = (packed >> shift) & 0xFFu32;
            let byte_pos = ui * 4 + b;
            let blk = byte_pos / half_bs;
            let wb = byte_pos % half_bs;
            let col = blk * block_size + wb * 2;

            let scale = f32::cast_from(scales[row * num_blocks + blk]);

            if col < k {
                let lo = f32::cast_from(byte_val & 0xFu32) - 8.0;
                let a = f32::cast_from(activation[col]);
                sum += a * lo * scale;
            }
            if col + 1 < k {
                let hi = f32::cast_from((byte_val >> 4u32) & 0xFu32) - 8.0;
                let a2 = f32::cast_from(activation[col + 1]);
                sum += a2 * hi * scale;
            }

            b += 1;
        }

        ui += 1;
    }

    output[row] = Line::cast_from(sum);
}
