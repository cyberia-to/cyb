//! Cross-entropy loss — Accelerate-optimized
//! Layout: logits[V, S] column-major

use crate::backend::cpu::accel::*;

/// Cross-entropy loss + softmax gradient — vDSP/vvexpf optimized
pub fn cross_entropy(
    dlogits: &mut [f32],
    logits: &[f32],
    targets: &[u16],
    v: usize,
    s: usize,
) -> f32 {
    let mut col = vec![0.0f32; v];
    let mut total_loss = 0.0f32;
    let inv_s = 1.0f32 / s as f32;

    for t in 0..s {
        // Gather column t (stride S)
        unsafe {
            cblas_scopy(
                v as i32,
                logits.as_ptr().add(t),
                s as i32,
                col.as_mut_ptr(),
                1,
            );
        }

        // Softmax with vDSP
        unsafe {
            let mut max_v: f32 = 0.0;
            vDSP_maxv(col.as_ptr(), 1, &mut max_v, v as u64);
            let neg_max = -max_v;
            vDSP_vsadd(col.as_ptr(), 1, &neg_max, col.as_mut_ptr(), 1, v as u64);
            let n = v as i32;
            vvexpf(col.as_mut_ptr(), col.as_ptr(), &n);
            let mut sum: f32 = 0.0;
            vDSP_sve(col.as_ptr(), 1, &mut sum, v as u64);
            let inv_sum = 1.0 / sum;
            vDSP_vsmul(col.as_ptr(), 1, &inv_sum, col.as_mut_ptr(), 1, v as u64);
        }

        // Loss + gradient
        let tgt = targets[t] as usize;
        total_loss -= (col[tgt] + 1e-10).ln();
        col[tgt] -= 1.0;
        unsafe {
            vDSP_vsmul(col.as_ptr(), 1, &inv_s, col.as_mut_ptr(), 1, v as u64);
        }

        // Scatter back
        unsafe {
            cblas_scopy(
                v as i32,
                col.as_ptr(),
                1,
                dlogits.as_mut_ptr().add(t),
                s as i32,
            );
        }
    }
    total_loss / s as f32
}
