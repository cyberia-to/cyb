//! AdamW optimizer

/// Adam optimizer state for one parameter group
pub struct AdamState {
    pub m: Vec<f32>, // first moment
    pub v: Vec<f32>, // second moment
}

impl AdamState {
    pub fn new(n: usize) -> Self {
        AdamState {
            m: vec![0.0; n],
            v: vec![0.0; n],
        }
    }
}

/// AdamW update step
pub fn update(
    w: &mut [f32],
    g: &[f32],
    state: &mut AdamState,
    t: usize,
    lr: f32,
    b1: f32,
    b2: f32,
    eps: f32,
    wd: f32,
) {
    let bc1 = 1.0 - b1.powi(t as i32);
    let bc2 = 1.0 - b2.powi(t as i32);
    for i in 0..w.len() {
        state.m[i] = b1 * state.m[i] + (1.0 - b1) * g[i];
        state.v[i] = b2 * state.v[i] + (1.0 - b2) * g[i] * g[i];
        let mh = state.m[i] / bc1;
        let vh = state.v[i] / bc2;
        w[i] -= lr * (mh / (vh.sqrt() + eps) + wd * w[i]);
    }
}
