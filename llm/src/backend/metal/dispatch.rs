//! Metal kernel dispatch — one function per MSL kernel
//!
//! Uses aruminium::Dispatch (pre-resolved IMP) for hot-path dispatch.

use aruminium::Buffer;
use super::pipelines::MetalPipelines;

/// matmul_f16: C[M,N] = A[M,K] @ B[K,N] — all fp16
pub fn matmul_f16(
    p: &MetalPipelines,
    a: &Buffer, b: &Buffer, c: &Buffer,
    m: u32, n: u32, k: u32,
) {
    let params = [m, n, k];
    unsafe {
        p.dispatcher.batch(|batch| {
            batch.bind(&p.matmul_f16);
            batch.bind_buffer(a, 0, 0);
            batch.bind_buffer(b, 0, 1);
            batch.bind_buffer(c, 0, 2);
            batch.push(bytemuck::cast_slice(&params), 3);
            batch.launch_groups(
                (div_ceil(n as usize, 64), div_ceil(m as usize, 64), 1),
                (512, 1, 1),
            );
        });
    }
}

/// matmul_q4: C[M,N] = A[M,K](fp16) @ B[K/32,N](q4_0) — prefill
pub fn matmul_q4(
    p: &MetalPipelines,
    a: &Buffer, b: &Buffer, c: &Buffer,
    m: u32, n: u32, k: u32,
) {
    let params = [m, n, k];
    unsafe {
        p.dispatcher.batch(|batch| {
            batch.bind(&p.matmul_q4);
            batch.bind_buffer(a, 0, 0);
            batch.bind_buffer(b, 0, 1);
            batch.bind_buffer(c, 0, 2);
            batch.push(bytemuck::cast_slice(&params), 3);
            batch.launch_groups(
                (div_ceil(n as usize, 64), div_ceil(m as usize, 64), 1),
                (512, 1, 1),
            );
        });
    }
}

/// matvec_q4: Y[N] = X[K](fp16) @ W[K/32,N](q4_0) — single decode
pub fn matvec_q4(
    p: &MetalPipelines,
    x: &Buffer, w: &Buffer, y: &Buffer,
    n: u32, k: u32,
) {
    let params = [n, k];
    unsafe {
        p.dispatcher.batch(|batch| {
            batch.bind(&p.matvec_q4);
            batch.bind_buffer(x, 0, 0);
            batch.bind_buffer(w, 0, 1);
            batch.bind_buffer(y, 0, 2);
            batch.push(bytemuck::cast_slice(&params), 3);
            batch.launch_groups((div_ceil(n as usize, 256), 1, 1), (256, 1, 1));
        });
    }
}

/// matvec_ternary: Y[N] = X[K](fp16) @ W[K/4,N](ternary) — single decode
pub fn matvec_ternary(
    p: &MetalPipelines,
    x: &Buffer, w: &Buffer, y: &Buffer,
    n: u32, k: u32,
) {
    let params = [n, k];
    unsafe {
        p.dispatcher.batch(|batch| {
            batch.bind(&p.matvec_ternary);
            batch.bind_buffer(x, 0, 0);
            batch.bind_buffer(w, 0, 1);
            batch.bind_buffer(y, 0, 2);
            batch.push(bytemuck::cast_slice(&params), 3);
            batch.launch_groups((div_ceil(n as usize, 256), 1, 1), (256, 1, 1));
        });
    }
}

/// matvec_q4k: Y[N] = X[K](fp16) @ W[K/256,N](q4_K) — single decode
pub fn matvec_q4k(
    p: &MetalPipelines,
    x: &Buffer, w: &Buffer, y: &Buffer,
    n: u32, k: u32,
) {
    let params = [n, k];
    unsafe {
        p.dispatcher.batch(|batch| {
            batch.bind(&p.matvec_q4k);
            batch.bind_buffer(x, 0, 0);
            batch.bind_buffer(w, 0, 1);
            batch.bind_buffer(y, 0, 2);
            batch.push(bytemuck::cast_slice(&params), 3);
            batch.launch_groups((div_ceil(n as usize, 256), 1, 1), (256, 1, 1));
        });
    }
}

/// matvec_q4_batch: Y[BATCH,N] = X[BATCH,K](fp16) @ W[K/32,N](q4_0) — batched decode
pub fn matvec_q4_batch(
    p: &MetalPipelines,
    x: &Buffer, w: &Buffer, y: &Buffer,
    n: u32, k: u32,
) {
    let params = [n, k];
    unsafe {
        p.dispatcher.batch(|batch| {
            batch.bind(&p.matvec_q4_batch);
            batch.bind_buffer(x, 0, 0);
            batch.bind_buffer(w, 0, 1);
            batch.bind_buffer(y, 0, 2);
            batch.push(bytemuck::cast_slice(&params), 3);
            batch.launch_groups((div_ceil(n as usize, 256), 1, 1), (256, 1, 1));
        });
    }
}

/// matvec_ternary_batch: Y[BATCH,N] = X[BATCH,K](fp16) @ W[K/4,N](ternary) — batched decode
pub fn matvec_ternary_batch(
    p: &MetalPipelines,
    x: &Buffer, w: &Buffer, y: &Buffer,
    n: u32, k: u32,
) {
    let params = [n, k];
    unsafe {
        p.dispatcher.batch(|batch| {
            batch.bind(&p.matvec_ternary_batch);
            batch.bind_buffer(x, 0, 0);
            batch.bind_buffer(w, 0, 1);
            batch.bind_buffer(y, 0, 2);
            batch.push(bytemuck::cast_slice(&params), 3);
            batch.launch_groups((div_ceil(n as usize, 256), 1, 1), (256, 1, 1));
        });
    }
}

/// Timed dispatch — uses safe command buffer API for GPU timestamps.
pub fn timed_dispatch(
    p: &MetalPipelines,
    pipeline: &aruminium::Pipeline,
    buffers: &[(&Buffer, usize)],
    params: &[u8],
    params_index: usize,
    groups: (usize, usize, usize),
    threads: (usize, usize, usize),
) -> f64 {
    let cmd = p.queue.commands().expect("command buffer");
    let enc = cmd.encoder().expect("encoder");
    enc.bind(pipeline);
    for &(buf, idx) in buffers {
        enc.bind_buffer(buf, 0, idx);
    }
    enc.push(params, params_index);
    enc.launch_groups(groups, threads);
    enc.finish();
    cmd.submit();
    cmd.wait();
    cmd.gpu_time()
}

#[inline]
fn div_ceil(a: usize, b: usize) -> usize {
    (a + b - 1) / b
}
