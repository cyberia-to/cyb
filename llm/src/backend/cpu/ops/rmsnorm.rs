//! RMSNorm forward and backward — Accelerate-optimized
//! Layout: [dim, seq] channel-first, x[d*S+t]

use crate::backend::cpu::accel::*;

/// RMSNorm forward: out[d,t] = x[d,t] * w[d] / sqrt(mean(x[:,t]^2) + eps)
/// Uses vDSP for vectorized operations — matches ObjC performance.
pub fn forward(out: &mut [f32], x: &[f32], w: &[f32], d: usize, s: usize) {
    unsafe {
        let mut ss = vec![0.0f32; s];
        let mut tmp = vec![0.0f32; s];

        // ss[t] = sum_d x[d*S+t]^2
        for i in 0..d {
            vDSP_vmul(
                x.as_ptr().add(i * s),
                1,
                x.as_ptr().add(i * s),
                1,
                tmp.as_mut_ptr(),
                1,
                s as u64,
            );
            vDSP_vadd(
                tmp.as_ptr(),
                1,
                ss.as_ptr(),
                1,
                ss.as_mut_ptr(),
                1,
                s as u64,
            );
        }

        // ss = 1/sqrt(ss/d + eps)
        let inv_d = 1.0f32 / d as f32;
        let eps = 1e-5f32;
        vDSP_vsmsa(ss.as_ptr(), 1, &inv_d, &eps, ss.as_mut_ptr(), 1, s as u64);
        let n = s as i32;
        vvrsqrtf(ss.as_mut_ptr(), ss.as_ptr(), &n);

        // out[d,t] = x[d,t] * ss[t] * w[d]
        for i in 0..d {
            vDSP_vmul(
                x.as_ptr().add(i * s),
                1,
                ss.as_ptr(),
                1,
                out.as_mut_ptr().add(i * s),
                1,
                s as u64,
            );
            vDSP_vsmul(
                out.as_ptr().add(i * s),
                1,
                &w[i],
                out.as_mut_ptr().add(i * s),
                1,
                s as u64,
            );
        }
    }
}

/// RMSNorm backward — Accelerate-optimized
pub fn backward(
    dx: &mut [f32],
    dw: &mut [f32],
    dy: &[f32],
    x: &[f32],
    w: &[f32],
    d: usize,
    s: usize,
) {
    unsafe {
        let mut tmp = vec![0.0f32; s];

        // Compute rrms = 1/sqrt(ss/d + eps)
        let mut ss = vec![0.0f32; s];
        for i in 0..d {
            vDSP_vmul(
                x.as_ptr().add(i * s),
                1,
                x.as_ptr().add(i * s),
                1,
                tmp.as_mut_ptr(),
                1,
                s as u64,
            );
            vDSP_vadd(
                tmp.as_ptr(),
                1,
                ss.as_ptr(),
                1,
                ss.as_mut_ptr(),
                1,
                s as u64,
            );
        }
        let inv_d = 1.0f32 / d as f32;
        let eps = 1e-5f32;
        vDSP_vsmsa(ss.as_ptr(), 1, &inv_d, &eps, ss.as_mut_ptr(), 1, s as u64);
        let mut rrms = vec![0.0f32; s];
        let n = s as i32;
        vvrsqrtf(rrms.as_mut_ptr(), ss.as_ptr(), &n);

        // dot[t] = sum_d dy[d,t] * x[d,t] * w[d]
        let mut dot = vec![0.0f32; s];
        for i in 0..d {
            vDSP_vmul(
                dy.as_ptr().add(i * s),
                1,
                x.as_ptr().add(i * s),
                1,
                tmp.as_mut_ptr(),
                1,
                s as u64,
            );
            vDSP_vsma(
                tmp.as_ptr(),
                1,
                &w[i],
                dot.as_ptr(),
                1,
                dot.as_mut_ptr(),
                1,
                s as u64,
            );
        }

        // coeff = dot * rrms^2 / d
        vDSP_vmul(
            rrms.as_ptr(),
            1,
            rrms.as_ptr(),
            1,
            ss.as_mut_ptr(),
            1,
            s as u64,
        );
        vDSP_vsmul(ss.as_ptr(), 1, &inv_d, ss.as_mut_ptr(), 1, s as u64);
        vDSP_vmul(
            dot.as_ptr(),
            1,
            ss.as_ptr(),
            1,
            dot.as_mut_ptr(),
            1,
            s as u64,
        );

        // dx[d,t] = (dy[d,t] - x[d,t]*coeff[t]) * rrms[t] * w[d]
        // dw[d] += sum_t dy[d,t] * x[d,t] * rrms[t]
        for i in 0..d {
            vDSP_vmul(
                x.as_ptr().add(i * s),
                1,
                dot.as_ptr(),
                1,
                tmp.as_mut_ptr(),
                1,
                s as u64,
            );
            vDSP_vsub(
                tmp.as_ptr(),
                1,
                dy.as_ptr().add(i * s),
                1,
                tmp.as_mut_ptr(),
                1,
                s as u64,
            );
            vDSP_vmul(
                tmp.as_ptr(),
                1,
                rrms.as_ptr(),
                1,
                tmp.as_mut_ptr(),
                1,
                s as u64,
            );
            vDSP_vsmul(
                tmp.as_ptr(),
                1,
                &w[i],
                dx.as_mut_ptr().add(i * s),
                1,
                s as u64,
            );

            vDSP_vmul(
                dy.as_ptr().add(i * s),
                1,
                x.as_ptr().add(i * s),
                1,
                tmp.as_mut_ptr(),
                1,
                s as u64,
            );
            vDSP_vmul(
                tmp.as_ptr(),
                1,
                rrms.as_ptr(),
                1,
                tmp.as_mut_ptr(),
                1,
                s as u64,
            );
            let mut sv: f32 = 0.0;
            vDSP_sve(tmp.as_ptr(), 1, &mut sv, s as u64);
            dw[i] += sv;
        }
    }
}

/// Single-token RMSNorm (for decode) — scalar, fast enough for dim=1024
pub fn forward_single(out: &mut [f32], x: &[f32], w: &[f32], d: usize) {
    let mut ss = 0.0f32;
    for i in 0..d {
        ss += x[i] * x[i];
    }
    let inv_rms = 1.0 / (ss / d as f32 + 1e-5).sqrt();
    for i in 0..d {
        out[i] = x[i] * inv_rms * w[i];
    }
}

/// Per-head RMSNorm (QK-norm) — scalar per-head, vectorized would be overkill
pub fn qk_norm(x: &mut [f32], w: &[f32], dim: usize, hd: usize, seq: usize) {
    let nheads = dim / hd;
    for h in 0..nheads {
        for s in 0..seq {
            let mut ss = 0.0f32;
            for d in 0..hd {
                let v = x[(h * hd + d) * seq + s];
                ss += v * v;
            }
            let inv_rms = 1.0 / (ss / hd as f32 + 1e-6).sqrt();
            for d in 0..hd {
                x[(h * hd + d) * seq + s] *= inv_rms * w[d];
            }
        }
    }
}

/// Single-token QK-norm
pub fn qk_norm_single(x: &mut [f32], w: &[f32], dim: usize, hd: usize) {
    let nheads = dim / hd;
    for h in 0..nheads {
        let mut ss = 0.0f32;
        for d in 0..hd {
            let v = x[h * hd + d];
            ss += v * v;
        }
        let inv_rms = 1.0 / (ss / hd as f32 + 1e-6).sqrt();
        for d in 0..hd {
            x[h * hd + d] *= inv_rms * w[d];
        }
    }
}
