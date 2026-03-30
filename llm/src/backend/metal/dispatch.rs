//! Metal kernel dispatch — one function per MSL kernel
//!
//! Uses ComputeDispatcher (pre-resolved IMP) for hot-path dispatch.
//! All kernels use threadgroup-based indexing (dispatch_threadgroups).

use aruminium::MtlBuffer;
use super::pipelines::MetalPipelines;

/// matmul_f16: C[M,N] = A[M,K] @ B[K,N] — all fp16
/// Grid: (ceil(N/64), ceil(M/64)) threadgroups of 512 threads
pub fn matmul_f16(
    p: &MetalPipelines,
    a: &MtlBuffer,
    b: &MtlBuffer,
    c: &MtlBuffer,
    m: u32, n: u32, k: u32,
) {
    let params = [m, n, k];
    unsafe {
        p.dispatcher.dispatch_batch(|batch| {
            batch.set_pipeline(&p.matmul_f16);
            batch.set_buffer(a, 0, 0);
            batch.set_buffer(b, 0, 1);
            batch.set_buffer(c, 0, 2);
            batch.set_bytes(bytemuck::cast_slice(&params), 3);
            batch.dispatch_threadgroups(
                (div_ceil(n as usize, 64), div_ceil(m as usize, 64), 1),
                (512, 1, 1),
            );
        });
    }
}

/// matmul_q4: C[M,N] = A[M,K](fp16) @ B[K/32,N](q4_0) — prefill
/// Grid: (ceil(N/64), ceil(M/64)) threadgroups of 512 threads
pub fn matmul_q4(
    p: &MetalPipelines,
    a: &MtlBuffer,
    b: &MtlBuffer,
    c: &MtlBuffer,
    m: u32, n: u32, k: u32,
) {
    let params = [m, n, k];
    unsafe {
        p.dispatcher.dispatch_batch(|batch| {
            batch.set_pipeline(&p.matmul_q4);
            batch.set_buffer(a, 0, 0);
            batch.set_buffer(b, 0, 1);
            batch.set_buffer(c, 0, 2);
            batch.set_bytes(bytemuck::cast_slice(&params), 3);
            batch.dispatch_threadgroups(
                (div_ceil(n as usize, 64), div_ceil(m as usize, 64), 1),
                (512, 1, 1),
            );
        });
    }
}

/// matvec_q4: Y[N] = X[K](fp16) @ W[K/32,N](q4_0) — single decode
/// Grid: (ceil(N/256)) threadgroups of 256 threads
pub fn matvec_q4(
    p: &MetalPipelines,
    x: &MtlBuffer,
    w: &MtlBuffer,
    y: &MtlBuffer,
    n: u32, k: u32,
) {
    let params = [n, k];
    unsafe {
        p.dispatcher.dispatch_batch(|batch| {
            batch.set_pipeline(&p.matvec_q4);
            batch.set_buffer(x, 0, 0);
            batch.set_buffer(w, 0, 1);
            batch.set_buffer(y, 0, 2);
            batch.set_bytes(bytemuck::cast_slice(&params), 3);
            batch.dispatch_threadgroups(
                (div_ceil(n as usize, 256), 1, 1),
                (256, 1, 1),
            );
        });
    }
}

/// matvec_ternary: Y[N] = X[K](fp16) @ W[K/4,N](ternary) — single decode
/// Grid: (ceil(N/256)) threadgroups of 256 threads
pub fn matvec_ternary(
    p: &MetalPipelines,
    x: &MtlBuffer,
    w: &MtlBuffer,
    y: &MtlBuffer,
    n: u32, k: u32,
) {
    let params = [n, k];
    unsafe {
        p.dispatcher.dispatch_batch(|batch| {
            batch.set_pipeline(&p.matvec_ternary);
            batch.set_buffer(x, 0, 0);
            batch.set_buffer(w, 0, 1);
            batch.set_buffer(y, 0, 2);
            batch.set_bytes(bytemuck::cast_slice(&params), 3);
            batch.dispatch_threadgroups(
                (div_ceil(n as usize, 256), 1, 1),
                (256, 1, 1),
            );
        });
    }
}

/// matvec_q4k: Y[N] = X[K](fp16) @ W[K/256,N](q4_K) — single decode
/// Grid: (ceil(N/256)) threadgroups of 256 threads
pub fn matvec_q4k(
    p: &MetalPipelines,
    x: &MtlBuffer,
    w: &MtlBuffer,
    y: &MtlBuffer,
    n: u32, k: u32,
) {
    let params = [n, k];
    unsafe {
        p.dispatcher.dispatch_batch(|batch| {
            batch.set_pipeline(&p.matvec_q4k);
            batch.set_buffer(x, 0, 0);
            batch.set_buffer(w, 0, 1);
            batch.set_buffer(y, 0, 2);
            batch.set_bytes(bytemuck::cast_slice(&params), 3);
            batch.dispatch_threadgroups(
                (div_ceil(n as usize, 256), 1, 1),
                (256, 1, 1),
            );
        });
    }
}

/// matvec_q4_batch: Y[BATCH,N] = X[BATCH,K](fp16) @ W[K/32,N](q4_0) — batched decode
/// Grid: (ceil(N/256)) threadgroups of 256 threads
pub fn matvec_q4_batch(
    p: &MetalPipelines,
    x: &MtlBuffer,
    w: &MtlBuffer,
    y: &MtlBuffer,
    n: u32, k: u32,
) {
    let params = [n, k];
    unsafe {
        p.dispatcher.dispatch_batch(|batch| {
            batch.set_pipeline(&p.matvec_q4_batch);
            batch.set_buffer(x, 0, 0);
            batch.set_buffer(w, 0, 1);
            batch.set_buffer(y, 0, 2);
            batch.set_bytes(bytemuck::cast_slice(&params), 3);
            batch.dispatch_threadgroups(
                (div_ceil(n as usize, 256), 1, 1),
                (256, 1, 1),
            );
        });
    }
}

/// matvec_ternary_batch: Y[BATCH,N] = X[BATCH,K](fp16) @ W[K/4,N](ternary) — batched decode
/// Grid: (ceil(N/256)) threadgroups of 256 threads
pub fn matvec_ternary_batch(
    p: &MetalPipelines,
    x: &MtlBuffer,
    w: &MtlBuffer,
    y: &MtlBuffer,
    n: u32, k: u32,
) {
    let params = [n, k];
    unsafe {
        p.dispatcher.dispatch_batch(|batch| {
            batch.set_pipeline(&p.matvec_ternary_batch);
            batch.set_buffer(x, 0, 0);
            batch.set_buffer(w, 0, 1);
            batch.set_buffer(y, 0, 2);
            batch.set_bytes(bytemuck::cast_slice(&params), 3);
            batch.dispatch_threadgroups(
                (div_ceil(n as usize, 256), 1, 1),
                (256, 1, 1),
            );
        });
    }
}

/// Timed dispatch — uses safe command buffer API for GPU timestamps.
/// Returns GPU execution time in seconds.
pub fn timed_dispatch(
    p: &MetalPipelines,
    pipeline: &aruminium::MtlComputePipeline,
    buffers: &[(&MtlBuffer, usize)],
    params: &[u8],
    params_index: usize,
    groups: (usize, usize, usize),
    threads: (usize, usize, usize),
) -> f64 {
    let cmd = p.queue.command_buffer().expect("command buffer");
    let enc = cmd.compute_encoder().expect("encoder");
    enc.set_pipeline(pipeline);
    for &(buf, idx) in buffers {
        enc.set_buffer(buf, 0, idx);
    }
    enc.set_bytes(params, params_index);
    enc.dispatch_threadgroups(groups, threads);
    enc.end_encoding();
    cmd.commit();
    cmd.wait_until_completed();
    cmd.gpu_time()
}

#[inline]
fn div_ceil(a: usize, b: usize) -> usize {
    (a + b - 1) / b
}
