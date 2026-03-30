//! Rotary Position Embedding (RoPE)
//! Layout: [dim, seq] channel-first

/// RoPE forward in-place on [dim, seq]
pub fn forward(x: &mut [f32], seq: usize, dim: usize, hd: usize) {
    let nheads = dim / hd;
    for h in 0..nheads {
        for i in 0..hd / 2 {
            let freq = 1.0 / 10000.0f32.powf(2.0 * i as f32 / hd as f32);
            for p in 0..seq {
                let theta = p as f32 * freq;
                let (sin_t, cos_t) = theta.sin_cos();
                let idx0 = (h * hd + 2 * i) * seq + p;
                let idx1 = (h * hd + 2 * i + 1) * seq + p;
                let v0 = x[idx0];
                let v1 = x[idx1];
                x[idx0] = v0 * cos_t - v1 * sin_t;
                x[idx1] = v0 * sin_t + v1 * cos_t;
            }
        }
    }
}

/// RoPE backward in-place (inverse rotation)
pub fn backward(dx: &mut [f32], seq: usize, dim: usize, hd: usize) {
    let nheads = dim / hd;
    for h in 0..nheads {
        for i in 0..hd / 2 {
            let freq = 1.0 / 10000.0f32.powf(2.0 * i as f32 / hd as f32);
            for p in 0..seq {
                let theta = p as f32 * freq;
                let (sin_t, cos_t) = theta.sin_cos();
                let idx0 = (h * hd + 2 * i) * seq + p;
                let idx1 = (h * hd + 2 * i + 1) * seq + p;
                let v0 = dx[idx0];
                let v1 = dx[idx1];
                dx[idx0] = v0 * cos_t + v1 * sin_t;
                dx[idx1] = -v0 * sin_t + v1 * cos_t;
            }
        }
    }
}

/// Single-token RoPE (for decode)
pub fn forward_single(x: &mut [f32], pos: usize, dim: usize, hd: usize) {
    let nheads = dim / hd;
    for h in 0..nheads {
        for i in 0..hd / 2 {
            let freq = 1.0 / 10000.0f32.powf(2.0 * i as f32 / hd as f32);
            let theta = pos as f32 * freq;
            let (sin_t, cos_t) = theta.sin_cos();
            let idx0 = h * hd + 2 * i;
            let idx1 = h * hd + 2 * i + 1;
            let v0 = x[idx0];
            let v1 = x[idx1];
            x[idx0] = v0 * cos_t - v1 * sin_t;
            x[idx1] = v0 * sin_t + v1 * cos_t;
        }
    }
}
