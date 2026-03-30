//! CPU attention — Accelerate-optimized with cblas_sgemm
//! Layout: Q[Q_DIM, SEQ], K[KV_DIM, SEQ], V[KV_DIM, SEQ]

use crate::backend::cpu::accel::*;

/// Full-sequence causal attention with GQA — cblas_sgemm optimized
/// Q: [q_dim, seq], K: [kv_dim, seq], V: [kv_dim, seq] → out: [q_dim, seq]
pub fn cpu_attention(
    out: &mut [f32],
    q: &[f32],
    k: &[f32],
    v: &[f32],
    heads: usize,
    kv_heads: usize,
    hd: usize,
    seq: usize,
) {
    let scale = 1.0 / (hd as f32).sqrt();
    let gqa_ratio = heads / kv_heads;
    let mut scores = vec![0.0f32; seq * seq];

    for h in 0..heads {
        let kv_h = h / gqa_ratio;
        let q_h = &q[h * hd * seq..];
        let k_h = &k[kv_h * hd * seq..];
        let v_h = &v[kv_h * hd * seq..];
        let out_h = &mut out[h * hd * seq..];

        // scores[seq, seq] = Q_h^T[seq, hd] @ K_h[hd, seq] * scale
        // Q_h is [hd, seq] row-major → Q_h^T is CblasTrans
        sgemm(
            true,
            false,
            seq,
            seq,
            hd,
            scale,
            q_h,
            seq,
            k_h,
            seq,
            0.0,
            &mut scores,
            seq,
        );

        // Causal mask + softmax per row
        for i in 0..seq {
            for j in (i + 1)..seq {
                scores[i * seq + j] = -1e9;
            }
            // Softmax using vDSP
            unsafe {
                let row = scores.as_mut_ptr().add(i * seq);
                let mut max_v: f32 = 0.0;
                vDSP_maxv(row, 1, &mut max_v, (i + 1) as u64);
                let neg_max = -max_v;
                vDSP_vsadd(row, 1, &neg_max, row, 1, (i + 1) as u64);
                let ni = (i + 1) as i32;
                vvexpf(row, row as *const f32, &ni);
                let mut sum: f32 = 0.0;
                vDSP_sve(row, 1, &mut sum, (i + 1) as u64);
                let inv = 1.0 / sum;
                vDSP_vsmul(row, 1, &inv, row, 1, (i + 1) as u64);
            }
            for j in (i + 1)..seq {
                scores[i * seq + j] = 0.0;
            }
        }

        // out_h[hd, seq] = V_h[hd, seq] @ scores^T[seq, seq]
        sgemm(
            false, true, hd, seq, seq, 1.0, v_h, seq, &scores, seq, 0.0, out_h, seq,
        );
    }
}

/// Single-query attention against KV cache — cblas_sgemv optimized
pub fn cpu_attention_cached(
    out: &mut [f32],
    q_single: &[f32],
    k_cache: &[f32],
    v_cache: &[f32],
    heads: usize,
    kv_heads: usize,
    hd: usize,
    cache_len: usize,
    cache_stride: usize,
) {
    let scale = 1.0 / (hd as f32).sqrt();
    let gqa = heads / kv_heads;
    let mut scores = vec![0.0f32; cache_len];

    for h in 0..heads {
        let kv_h = h / gqa;
        let q_off = h * hd;
        let k_off = kv_h * hd * cache_stride;
        let v_off = kv_h * hd * cache_stride;
        let o_off = h * hd;

        // scores[j] = sum_d Q[d] * K[d, j] * scale
        scores.iter_mut().for_each(|s| *s = 0.0);
        for d in 0..hd {
            let qv = q_single[q_off + d] * scale;
            unsafe {
                vDSP_vsma(
                    k_cache.as_ptr().add(k_off + d * cache_stride),
                    1,
                    &qv,
                    scores.as_ptr(),
                    1,
                    scores.as_mut_ptr(),
                    1,
                    cache_len as u64,
                );
            }
        }

        // Softmax
        unsafe {
            let mut max_v: f32 = 0.0;
            vDSP_maxv(scores.as_ptr(), 1, &mut max_v, cache_len as u64);
            let neg_max = -max_v;
            vDSP_vsadd(
                scores.as_ptr(),
                1,
                &neg_max,
                scores.as_mut_ptr(),
                1,
                cache_len as u64,
            );
            let n = cache_len as i32;
            vvexpf(scores.as_mut_ptr(), scores.as_ptr(), &n);
            let mut sum: f32 = 0.0;
            vDSP_sve(scores.as_ptr(), 1, &mut sum, cache_len as u64);
            let inv = 1.0 / sum;
            vDSP_vsmul(
                scores.as_ptr(),
                1,
                &inv,
                scores.as_mut_ptr(),
                1,
                cache_len as u64,
            );
        }

        // out[d] = sum_j V[d, j] * scores[j]
        for d in 0..hd {
            unsafe {
                let mut dot: f32 = 0.0;
                vDSP_dotpr(
                    v_cache.as_ptr().add(v_off + d * cache_stride),
                    1,
                    scores.as_ptr(),
                    1,
                    &mut dot,
                    cache_len as u64,
                );
                out[o_off + d] = dot;
            }
        }
    }
}

// vDSP_dotpr FFI
unsafe extern "C" {
    fn vDSP_dotpr(A: *const f32, IA: i64, B: *const f32, IB: i64, C: *mut f32, N: u64);
}
