//! honeycrisp — Apple Silicon turbo backend via aruminium.
//!
//! Stack: Metal (GPU) + ANE + AMX + NEON + unimem zero-copy.
//! Currently implements Metal compute kernels for hot f32 ops and
//! fused Q4_K/Q6_K dequant+matmul. ANE/AMX integration is future.
//!
//! Spec: specs/architecture.md#honeycrisp

#![cfg(target_os = "macos")]

use crate::backend::{Backend, BackendError, BackendKind};
use crate::backend::cpu::CpuBackend;
use crate::core::dtype::DType;
use crate::core::op::Op;
use crate::core::tensor::{BackendData, Tensor, TensorData};
use std::any::Any;
use std::sync::Arc;

mod device;
mod kernels;

use device::HoneycrispDevice;

struct HcBuffer {
    buffer: aruminium::Buffer,
    /// Logical byte length of valid data inside `buffer` (may be ≤ buffer.size()
    /// if the buffer was over-allocated).
    bytes: usize,
}

/// Send-marked wrapper for Buffer. Metal buffers are thread-safe per Apple
/// docs; Rust marks the underlying raw pointer as !Send conservatively.
struct PooledBuf(aruminium::Buffer);
unsafe impl Send for PooledBuf {}

/// GPU KV cache + attention pipelines, lazily constructed for a specific
/// (num_heads, kv_heads, head_dim, max_seq) tuple — needed because the
/// attention kernel bakes those as MSL constants (avoids the rope-style
/// Metal scheduler regression).
struct AttnState {
    num_heads: u32,
    kv_heads: u32,
    head_dim: u32,
    max_seq: u32,
    pipe_attn: HcPipeline,
    pipe_kv_append: HcPipeline,
    pipe_kv_append_both: HcPipeline,
    /// Fused RoPE + KV-append + SDPA in one dispatch.
    pipe_rope_kv_attn: HcPipeline,
    /// QK-norm + RoPE fused (parameterized for this geometry's head_dim).
    pipe_qk_norm_rope: HcPipeline,
    /// Per-layer K and V buffers, indexed by compact slot (see layer_slot).
    /// Compact slots avoid allocating empty entries for interleaved layers:
    /// e.g. Gemma-4 full layers at global indices 5,11,..59 get slots 0..9.
    k_caches: Vec<aruminium::Buffer>,
    v_caches: Vec<aruminium::Buffer>,
    /// Maps global layer_idx → k_caches/v_caches slot index.
    layer_slot: std::collections::HashMap<usize, usize>,
    /// Precomputed RoPE tables: [max_seq, rope_half] f32 each.
    /// Eliminates per-token powf/cos/sin + buffer_with_data calls.
    rope_half: u32,
    rope_cos: aruminium::Buffer,
    rope_sin: aruminium::Buffer,
}
unsafe impl Send for AttnState {}
unsafe impl Sync for AttnState {}

/// Tiny LIFO pool of Metal buffers keyed by a power-of-two size class.
/// Avoids `newBufferWithLength` syscalls in the hot path.
struct BufferPool {
    by_class: std::sync::Mutex<std::collections::HashMap<usize, Vec<PooledBuf>>>,
}

impl BufferPool {
    fn new() -> Self {
        Self { by_class: std::sync::Mutex::new(std::collections::HashMap::new()) }
    }
    fn class_for(size: usize) -> usize {
        // Round up to next power of two, min 4 KB.
        let s = size.max(4096);
        s.next_power_of_two()
    }
    fn pop(&self, size: usize) -> Option<aruminium::Buffer> {
        let cls = Self::class_for(size);
        self.by_class.lock().ok()?.get_mut(&cls)?.pop().map(|p| p.0)
    }
    fn push(&self, buf: aruminium::Buffer) {
        let cls = Self::class_for(buf.size());
        if let Ok(mut map) = self.by_class.lock() {
            map.entry(cls).or_insert_with(Vec::new).push(PooledBuf(buf));
        }
    }
}

/// Owned-or-borrowed Metal buffer reference for kernel dispatch.
enum BufRef<'a> {
    Owned(aruminium::Buffer),
    Borrowed(&'a aruminium::Buffer),
}

impl<'a> BufRef<'a> {
    fn as_buffer(&self) -> &aruminium::Buffer {
        match self {
            BufRef::Owned(b) => b,
            BufRef::Borrowed(b) => b,
        }
    }
}

unsafe impl Send for HcBuffer {}
unsafe impl Sync for HcBuffer {}

impl BackendData for HcBuffer {
    fn backend_name(&self) -> &'static str { "honeycrisp" }
    fn as_any(&self) -> &dyn Any { self }
    fn try_as_host_bytes(&self) -> Option<&[u8]> {
        if !self.buffer.is_shared() { return None; }
        // Shared storage: contents pointer is CPU-readable. Only valid AFTER
        // the dispatch wait for the command buffer that wrote it. Our backend
        // always waits inside batch_raw before returning, so this is safe.
        Some(&self.buffer.as_bytes()[..self.bytes])
    }
}

/// Wrapper to make aruminium::Pipeline Send+Sync.
/// Metal objects are thread-safe per Apple's documentation.
struct HcPipeline(aruminium::Pipeline);
unsafe impl Send for HcPipeline {}
unsafe impl Sync for HcPipeline {}

pub struct HoneycrispBackend {
    device: Arc<HoneycrispDevice>,
    cpu: CpuBackend,
    pipe_matmul: HcPipeline,
    pipe_rmsnorm: HcPipeline,
    pipe_silu: HcPipeline,
    pipe_scale: HcPipeline,
    pipe_q4: HcPipeline,
    pipe_q4k: HcPipeline,
    pipe_q6k: HcPipeline,
    pipe_q8: HcPipeline,
    pipe_add: HcPipeline,
    pipe_silu_mul: HcPipeline,
    pipe_rope: HcPipeline,
    pipe_q4_dual: HcPipeline,
    pipe_q8_dual: HcPipeline,
    pipe_q4_gus: HcPipeline,
    pipe_q8_gus: HcPipeline,
    pipe_qk_rope: HcPipeline,
    pipe_qk_norm: HcPipeline,
    pipe_qk_norm_rope: HcPipeline,
    pipe_q4_qkv: HcPipeline,
    pipe_q8_qkv: HcPipeline,
    pipe_q4_res: HcPipeline,
    pipe_q8_res: HcPipeline,
    // Sized-MAX_BLOCKS variants for forward_decode_fused_layers (better TG occupancy).
    pipe_q4_mb32:      HcPipeline,  // basic, k_in=1024 (32 blocks)
    pipe_q8_mb32:      HcPipeline,
    pipe_q4_dual_mb32: HcPipeline,  // KV dual, k_in=1024
    pipe_q8_dual_mb32: HcPipeline,
    pipe_q4_gus_mb32:  HcPipeline,  // gate+up+silu, k_in=1024
    pipe_q8_gus_mb32:  HcPipeline,
    pipe_q4_res_mb64:  HcPipeline,  // matmul+residual, k_in=2048 (o_proj)
    pipe_q8_res_mb64:  HcPipeline,
    pipe_q4_res_mb96:  HcPipeline,  // matmul+residual, k_in=3072 (down_proj)
    pipe_q8_res_mb96:  HcPipeline,
    pipe_q4_gus_nrm_mb32: HcPipeline,  // post_norm+gate+up+silu fused, k_in=1024
    pipe_q8_gus_nrm_mb32: HcPipeline,
    pipe_q4_nrm_mb32:      HcPipeline,  // input_norm+Q matmul fused, k_in=1024
    pipe_q8_nrm_mb32:      HcPipeline,
    pipe_q4_dual_nrm_mb32: HcPipeline,  // input_norm+KV dual fused, k_in=1024
    pipe_q8_dual_nrm_mb32: HcPipeline,
    /// No-threadgroup-cache Q8 pipeline: correct for any n_blocks (large k).
    /// Used when n_blocks > TG_MAX_BLOCKS to avoid threadgroup memory overflow.
    pipe_q8_large: HcPipeline,
    /// Large-geometry fused-path kernels: no TG cache, work for any n_blocks.
    pipe_q8_large_nrm:     HcPipeline,  // inline RMSnorm + Q8 matmul
    pipe_q8_large_gus_nrm: HcPipeline,  // inline RMSnorm + gate+up+silu
    pipe_q8_large_res:     HcPipeline,  // Q8 matmul + residual add
    /// TG-cached nrm variants for intermediate geometry (MAX_BLOCKS=64, k_in≤2048).
    /// Covers qwen2.5-coder style (hidden=1536, n_blk=48) without falling back to
    /// the no-cache LARGE path, which causes excessive L2 traffic at high n_rows.
    pipe_q8_nrm_mb64:        HcPipeline,
    pipe_q8_dual_nrm_mb64:   HcPipeline,
    pipe_q8_gus_nrm_mb64:    HcPipeline,
    /// MB48 gate+up+silu+norm: 6 KB TG memory vs 8 KB → 5 TGs/core (vs 4) → 200 concurrent TGs.
    /// Only safe when n_blk_kd ≤ 48 (e.g. qwen2.5-coder hidden=1536, 48 blocks).
    pipe_q8_gus_nrm_mb48:    HcPipeline,
    /// Q4 equivalents for the MB64/MB48 path (models with n_blk_kd 33-64, e.g. qwen2.5-coder Q4).
    pipe_q4_nrm_mb64:        HcPipeline,
    pipe_q4_dual_nrm_mb64:   HcPipeline,
    pipe_q4_gus_nrm_mb64:    HcPipeline,
    pipe_q4_gus_nrm_mb48:    HcPipeline,
    /// Q4 matmul, no TG cache: for large n_rows (e.g. lm_head) where L2 beats TG cache.
    pipe_q4_large:           HcPipeline,
    /// Q4 matmul+residual, no TG cache: for down_proj when n_blk_inter > 96.
    pipe_q4_large_res:       HcPipeline,
    /// 4-rows-per-SIMD gate+up+silu+norm with TG x-cache: same 8 KB TG memory as mb64
    /// but 4× fewer threadgroups (140 vs 560 for 8960 rows) → fits in one GPU wave.
    pipe_q8_gus_nrm_mb64_r4: HcPipeline,
    /// LARGE4 variants: 4 output rows per SIMD group, 4× fewer threadgroups.
    /// Reduces GPU wave serialization for large n_rows (e.g. gate+up at 8960 rows).
    pipe_q8_large4_nrm:          HcPipeline,
    pipe_q8_large4_gus_nrm:      HcPipeline,
    pipe_q8_large4_gus_nrm_gelu: HcPipeline,  // GeluTanh variant (Gemma-4)
    pipe_q8_large2_res:     HcPipeline,
    pipe_q8_large4_res:     HcPipeline,
    pipe_q8_large8_res:     HcPipeline,
    pipe_q8_mbx16_res:      HcPipeline,
    pipe_q8_large4t_res:    HcPipeline,
    pipe_q8_large8t_res:    HcPipeline,
    pipe_q8_large16t_res:   HcPipeline,
    /// TG-cached Q8 matmul with MAX_BLOCKS=192 (24 KB TG memory).
    /// Covers models with k_dim up to 192*32=6144 (e.g. Gemma-4 n_blk_kd=168).
    /// Between TG_MAX_BLOCKS=128 and 192 → full TG x-cache without overflow.
    pipe_q8_mb192: HcPipeline,
    /// Set to true after the first `forward_decode_fused_layers` call.
    /// The first call runs 3 warmup CB iterations to ramp the Metal GPU from
    /// its post-upload idle/low-clock state to full decode speed before
    /// returning the first real token. Eliminates the 3-token JIT warmup period.
    warmed_up: std::sync::atomic::AtomicBool,
    /// Recyclable scratch buffer pool — avoid `newBufferWithLength` per call
    /// inside fused chains.
    scratch: BufferPool,
    /// Lazily-built attention states, one per geometry (num_heads, kv_heads, head_dim, max_seq).
    /// Multiple entries support models with mixed-geometry layers (e.g. Gemma-4 sliding+full).
    attn: std::sync::Mutex<Vec<AttnState>>,
    /// Lazy transposed down_proj weight cache for the fused forward path.
    /// Key = original Metal buffer address (stable, page-aligned).
    /// Value = transposed Q8 buffer [n_blocks, n_rows, 34] on GPU.
    transposed_down_w: std::sync::Mutex<TransposedCache>,
}

struct TransposedCache(std::collections::HashMap<usize, Box<aruminium::Buffer>>);
// SAFETY: aruminium::Buffer wraps an Objective-C Metal object which is thread-safe
// when accessed through the Mutex guard. We only ever read/write via &mut from the lock.
unsafe impl Send for TransposedCache {}

/// Transpose Q8_0 weight blocks from [n_rows, n_blocks, 34] to [n_blocks, n_rows, 34].
/// With the transposed layout, consecutive rows at the same block index are 34 bytes apart,
/// greatly improving GPU cache line utilization (from ~27% to ~71%).
fn transpose_q8_blocks(src: &[u8], n_rows: usize, n_blk: usize) -> Vec<u8> {
    const BLOCK_BYTES: usize = 34;
    let mut dst = vec![0u8; n_rows * n_blk * BLOCK_BYTES];
    for row in 0..n_rows {
        for blk in 0..n_blk {
            let src_off = (row * n_blk + blk) * BLOCK_BYTES;
            let dst_off = (blk * n_rows + row) * BLOCK_BYTES;
            dst[dst_off..dst_off + BLOCK_BYTES]
                .copy_from_slice(&src[src_off..src_off + BLOCK_BYTES]);
        }
    }
    dst
}

impl HoneycrispBackend {
    pub fn new() -> Result<Self, BackendError> {
        let device = Arc::new(HoneycrispDevice::new()?);
        let pipe_matmul = HcPipeline(device.pipeline(kernels::matmul::MSL)?);
        let pipe_rmsnorm = HcPipeline(device.pipeline(kernels::rmsnorm::MSL)?);
        let pipe_silu = HcPipeline(device.pipeline(kernels::silu::MSL)?);
        let pipe_scale = HcPipeline(device.pipeline(kernels::silu::MSL_SCALE)?);
        let pipe_q4k = HcPipeline(device.pipeline(kernels::q4k_matmul::MSL)?);
        let pipe_q6k = HcPipeline(device.pipeline(kernels::q6k_matmul::MSL)?);
        let pipe_q8 = HcPipeline(device.pipeline(&kernels::q8_matmul::msl())?);
        let pipe_q4 = HcPipeline(device.pipeline(&kernels::q4_matmul::msl())?);
        let pipe_add = HcPipeline(device.pipeline(kernels::elementwise::ADD_MSL)?);
        let pipe_silu_mul = HcPipeline(device.pipeline(kernels::elementwise::SILU_MUL_MSL)?);
        let pipe_rope = HcPipeline(device.pipeline(kernels::rope::MSL)?);
        let pipe_q4_dual = HcPipeline(device.pipeline(&kernels::q4_matmul::msl_dual())?);
        let pipe_q8_dual = HcPipeline(device.pipeline(&kernels::q8_matmul::msl_dual())?);
        let pipe_q4_gus  = HcPipeline(device.pipeline(&kernels::q4_matmul::msl_gus())?);
        let pipe_q8_gus  = HcPipeline(device.pipeline(&kernels::q8_matmul::msl_gus())?);
        let pipe_qk_rope = HcPipeline(device.pipeline(kernels::rope::MSL_QK)?);
        let pipe_qk_norm = HcPipeline(device.pipeline(kernels::rmsnorm::MSL_QK)?);
        let pipe_qk_norm_rope = HcPipeline(device.pipeline(kernels::rope::MSL_QK_NORM_ROPE)?);
        let pipe_q4_qkv  = HcPipeline(device.pipeline(kernels::q4_matmul::MSL_QKV)?);
        let pipe_q8_qkv  = HcPipeline(device.pipeline(kernels::q8_matmul::MSL_QKV)?);
        let pipe_q4_res  = HcPipeline(device.pipeline(&kernels::q4_matmul::msl_res())?);
        let pipe_q8_res  = HcPipeline(device.pipeline(&kernels::q8_matmul::msl_res())?);
        // Optimized fused-path pipelines with exact MAX_BLOCKS for model geometry.
        let pipe_q4_mb32      = HcPipeline(device.pipeline(&kernels::q4_matmul::msl_mb(32))?);
        let pipe_q8_mb32      = HcPipeline(device.pipeline(&kernels::q8_matmul::msl_mb(32))?);
        let pipe_q4_dual_mb32 = HcPipeline(device.pipeline(&kernels::q4_matmul::msl_dual_mb(32))?);
        let pipe_q8_dual_mb32 = HcPipeline(device.pipeline(&kernels::q8_matmul::msl_dual_mb(32))?);
        let pipe_q4_gus_mb32  = HcPipeline(device.pipeline(&kernels::q4_matmul::msl_gus_mb(32))?);
        let pipe_q8_gus_mb32  = HcPipeline(device.pipeline(&kernels::q8_matmul::msl_gus_mb(32))?);
        let pipe_q4_res_mb64  = HcPipeline(device.pipeline(&kernels::q4_matmul::msl_res_mb(64))?);
        let pipe_q8_res_mb64  = HcPipeline(device.pipeline(&kernels::q8_matmul::msl_res_mb(64))?);
        let pipe_q4_res_mb96  = HcPipeline(device.pipeline(&kernels::q4_matmul::msl_res_mb(96))?);
        let pipe_q8_res_mb96  = HcPipeline(device.pipeline(&kernels::q8_matmul::msl_res_mb(96))?);
        let pipe_q4_gus_nrm_mb32 = HcPipeline(device.pipeline(&kernels::q4_matmul::msl_gus_nrm_mb(32))?);
        let pipe_q8_gus_nrm_mb32 = HcPipeline(device.pipeline(&kernels::q8_matmul::msl_gus_nrm_mb(32))?);
        let pipe_q4_nrm_mb32      = HcPipeline(device.pipeline(&kernels::q4_matmul::msl_nrm_mb(32))?);
        let pipe_q8_nrm_mb32      = HcPipeline(device.pipeline(&kernels::q8_matmul::msl_nrm_mb(32))?);
        let pipe_q4_dual_nrm_mb32 = HcPipeline(device.pipeline(&kernels::q4_matmul::msl_dual_nrm_mb(32))?);
        let pipe_q8_dual_nrm_mb32 = HcPipeline(device.pipeline(&kernels::q8_matmul::msl_dual_nrm_mb(32))?);
        let pipe_q8_large = HcPipeline(device.pipeline(kernels::q8_matmul::MSL_LARGE)?);
        let pipe_q8_large_nrm     = HcPipeline(device.pipeline(kernels::q8_matmul::MSL_LARGE_NRM)?);
        let pipe_q8_large_gus_nrm = HcPipeline(device.pipeline(kernels::q8_matmul::MSL_LARGE_GUS_NRM)?);
        let pipe_q8_large_res     = HcPipeline(device.pipeline(kernels::q8_matmul::MSL_LARGE_RES)?);
        let pipe_q8_nrm_mb64        = HcPipeline(device.pipeline(&kernels::q8_matmul::msl_nrm_mb(64))?);
        let pipe_q8_dual_nrm_mb64   = HcPipeline(device.pipeline(&kernels::q8_matmul::msl_dual_nrm_mb(64))?);
        let pipe_q8_gus_nrm_mb64    = HcPipeline(device.pipeline(&kernels::q8_matmul::msl_gus_nrm_mb(64))?);
        let pipe_q8_gus_nrm_mb48    = HcPipeline(device.pipeline(&kernels::q8_matmul::msl_gus_nrm_mb(48))?);
        let pipe_q8_gus_nrm_mb64_r4 = HcPipeline(device.pipeline(kernels::q8_matmul::MSL_GUS_NRM_MB64_R4)?);
        let pipe_q4_nrm_mb64        = HcPipeline(device.pipeline(&kernels::q4_matmul::msl_nrm_mb(64))?);
        let pipe_q4_dual_nrm_mb64   = HcPipeline(device.pipeline(&kernels::q4_matmul::msl_dual_nrm_mb(64))?);
        let pipe_q4_gus_nrm_mb64    = HcPipeline(device.pipeline(&kernels::q4_matmul::msl_gus_nrm_mb(64))?);
        let pipe_q4_gus_nrm_mb48    = HcPipeline(device.pipeline(&kernels::q4_matmul::msl_gus_nrm_mb(48))?);
        let pipe_q4_large           = HcPipeline(device.pipeline(kernels::q4_matmul::MSL_LARGE)?);
        let pipe_q4_large_res       = HcPipeline(device.pipeline(kernels::q4_matmul::MSL_LARGE_RES)?);
        let pipe_q8_large4_nrm          = HcPipeline(device.pipeline(kernels::q8_matmul::MSL_LARGE4_NRM)?);
        let pipe_q8_large4_gus_nrm      = HcPipeline(device.pipeline(kernels::q8_matmul::MSL_LARGE4_GUS_NRM)?);
        let pipe_q8_large4_gus_nrm_gelu = HcPipeline(device.pipeline(kernels::q8_matmul::MSL_LARGE4_GUS_NRM_GELU)?);
        let pipe_q8_large2_res     = HcPipeline(device.pipeline(kernels::q8_matmul::MSL_LARGE2_RES)?);
        let pipe_q8_large4_res     = HcPipeline(device.pipeline(kernels::q8_matmul::MSL_LARGE4_RES)?);
        let pipe_q8_large8_res     = HcPipeline(device.pipeline(kernels::q8_matmul::MSL_LARGE8_RES)?);
        let pipe_q8_mbx16_res      = HcPipeline(device.pipeline(kernels::q8_matmul::MSL_MBX16_RES)?);
        let pipe_q8_large4t_res    = HcPipeline(device.pipeline(kernels::q8_matmul::MSL_LARGE4T_RES)?);
        let pipe_q8_large8t_res    = HcPipeline(device.pipeline(kernels::q8_matmul::MSL_LARGE8T_RES)?);
        let pipe_q8_large16t_res   = HcPipeline(device.pipeline(kernels::q8_matmul::MSL_LARGE16T_RES)?);
        let pipe_q8_mb192          = HcPipeline(device.pipeline(&kernels::q8_matmul::msl_mb(192))?);
        Ok(Self {
            device,
            cpu: CpuBackend::new(),
            pipe_matmul,
            pipe_rmsnorm,
            pipe_silu,
            pipe_scale,
            pipe_q4,
            pipe_q4k,
            pipe_q6k,
            pipe_q8,
            pipe_add,
            pipe_silu_mul,
            pipe_rope,
            pipe_q4_dual,
            pipe_q8_dual,
            pipe_q4_gus,
            pipe_q8_gus,
            pipe_qk_rope,
            pipe_qk_norm,
            pipe_qk_norm_rope,
            pipe_q4_qkv,
            pipe_q8_qkv,
            pipe_q4_res,
            pipe_q8_res,
            pipe_q4_mb32,
            pipe_q8_mb32,
            pipe_q4_dual_mb32,
            pipe_q8_dual_mb32,
            pipe_q4_gus_mb32,
            pipe_q8_gus_mb32,
            pipe_q4_res_mb64,
            pipe_q8_res_mb64,
            pipe_q4_res_mb96,
            pipe_q8_res_mb96,
            pipe_q4_gus_nrm_mb32,
            pipe_q8_gus_nrm_mb32,
            pipe_q4_nrm_mb32,
            pipe_q8_nrm_mb32,
            pipe_q4_dual_nrm_mb32,
            pipe_q8_dual_nrm_mb32,
            pipe_q8_large,
            pipe_q8_large_nrm,
            pipe_q8_large_gus_nrm,
            pipe_q8_large_res,
            pipe_q8_nrm_mb64,
            pipe_q8_dual_nrm_mb64,
            pipe_q8_gus_nrm_mb64,
            pipe_q8_gus_nrm_mb48,
            pipe_q8_gus_nrm_mb64_r4,
            pipe_q4_nrm_mb64,
            pipe_q4_dual_nrm_mb64,
            pipe_q4_gus_nrm_mb64,
            pipe_q4_gus_nrm_mb48,
            pipe_q4_large,
            pipe_q4_large_res,
            pipe_q8_large4_nrm,
            pipe_q8_large4_gus_nrm,
            pipe_q8_large4_gus_nrm_gelu,
            pipe_q8_large2_res,
            pipe_q8_large4_res,
            pipe_q8_large8_res,
            pipe_q8_mbx16_res,
            pipe_q8_large4t_res,
            pipe_q8_large8t_res,
            pipe_q8_large16t_res,
            pipe_q8_mb192,
            warmed_up: std::sync::atomic::AtomicBool::new(false),
            scratch: BufferPool::new(),
            attn: std::sync::Mutex::new(Vec::new()),
            transposed_down_w: std::sync::Mutex::new(TransposedCache(std::collections::HashMap::new())),
        })
    }

    /// Lazily build attention state. Geometry change → full reset (drops cache).
    /// Same geometry but more layers needed → APPEND new cache buffers without
    /// touching existing ones (so per-layer state survives forward()).
    /// Build or grow an AttnState for the given geometry + layer indices.
    /// Uses compact slot allocation: each distinct layer_idx gets one slot,
    /// avoiding huge gaps when layers alternate geometry (e.g. Gemma-4
    /// full-attention layers at global indices 5,11,...59 → slots 0..9).
    fn attn_state(
        &self,
        num_heads: u32,
        kv_heads: u32,
        head_dim: u32,
        max_seq: u32,
        layer_indices: &[usize],
        rope_dim: u32,
        rope_theta: f32,
    ) -> Result<std::sync::MutexGuard<Vec<AttnState>>, BackendError> {
        let mut guard = self.attn.lock().unwrap();
        let cache_bytes =
            (kv_heads as usize) * (max_seq as usize) * (head_dim as usize) * 4;
        let rope_half = rope_dim / 2;

        // Find existing state matching this geometry (keyed by the 4-tuple).
        let match_idx = guard.iter().position(|s|
            s.num_heads == num_heads
                && s.kv_heads == kv_heads
                && s.head_dim == head_dim
                && s.max_seq == max_seq
                && (rope_half == 0 || s.rope_half == 0 || s.rope_half == rope_half)
        );

        if let Some(i) = match_idx {
            // Matching geometry — add new slots only for unseen layer_indices.
            let st = &mut guard[i];
            for &li in layer_indices {
                if !st.layer_slot.contains_key(&li) {
                    let slot = st.k_caches.len();
                    st.k_caches.push(self.device.alloc(cache_bytes)?);
                    st.v_caches.push(self.device.alloc(cache_bytes)?);
                    st.layer_slot.insert(li, slot);
                }
            }
            return Ok(guard);
        }

        // No existing state for this geometry — build a new one and append.
        let attn_msl = kernels::attention::msl_for(
            num_heads as usize, kv_heads as usize, head_dim as usize, max_seq as usize,
        );
        let kv_msl = kernels::attention::kv_append_msl_for(
            kv_heads as usize, head_dim as usize, max_seq as usize,
        );
        let kv_both_msl = kernels::attention::kv_append_both_msl_for(
            kv_heads as usize, head_dim as usize, max_seq as usize,
        );
        let fused_msl = kernels::attention::fused_rope_kv_attn_msl_for(
            num_heads as usize, kv_heads as usize, max_seq as usize,
        );
        let pipe_attn = HcPipeline(self.device.pipeline(&attn_msl)?);
        let pipe_kv_append = HcPipeline(self.device.pipeline(&kv_msl)?);
        let pipe_kv_append_both = HcPipeline(self.device.pipeline(&kv_both_msl)?);
        let pipe_rope_kv_attn = HcPipeline(self.device.pipeline(&fused_msl)?);
        let qk_norm_rope_msl = kernels::rope::msl_qk_norm_rope_for(head_dim as usize);
        let pipe_qk_norm_rope = HcPipeline(self.device.pipeline(&qk_norm_rope_msl)?);

        let mut k_caches = Vec::with_capacity(layer_indices.len());
        let mut v_caches = Vec::with_capacity(layer_indices.len());
        let mut layer_slot = std::collections::HashMap::new();
        for &li in layer_indices {
            if !layer_slot.contains_key(&li) {
                let slot = k_caches.len();
                k_caches.push(self.device.alloc(cache_bytes)?);
                v_caches.push(self.device.alloc(cache_bytes)?);
                layer_slot.insert(li, slot);
            }
        }

        // Precompute RoPE tables [max_seq, rope_half] f32. Eliminates per-token
        // powf/cos/sin + buffer_with_data allocations from the decode hot path.
        // When rope_dim==0 the caller doesn't use the tables; build tiny placeholders.
        let (rope_cos, rope_sin) = if rope_half > 0 {
            let hd = head_dim as f32;
            let mut cos_data = vec![0f32; max_seq as usize * rope_half as usize];
            let mut sin_data = vec![0f32; max_seq as usize * rope_half as usize];
            for pos in 0..max_seq as usize {
                for j in 0..rope_half as usize {
                    let theta = (pos as f32) / rope_theta.powf(2.0 * j as f32 / hd);
                    cos_data[pos * rope_half as usize + j] = theta.cos();
                    sin_data[pos * rope_half as usize + j] = theta.sin();
                }
            }
            let rc = self.device.gpu.buffer_with_data(bytemuck::cast_slice(&cos_data))
                .map_err(|e| BackendError::Internal(format!("rope_cos: {e}")))?;
            let rs = self.device.gpu.buffer_with_data(bytemuck::cast_slice(&sin_data))
                .map_err(|e| BackendError::Internal(format!("rope_sin: {e}")))?;
            (rc, rs)
        } else {
            let rc = self.device.alloc(4)?;
            let rs = self.device.alloc(4)?;
            (rc, rs)
        };

        guard.push(AttnState {
            num_heads, kv_heads, head_dim, max_seq,
            pipe_attn, pipe_kv_append, pipe_kv_append_both, pipe_rope_kv_attn,
            pipe_qk_norm_rope,
            k_caches, v_caches, layer_slot,
            rope_half, rope_cos, rope_sin,
        });
        Ok(guard)
    }

    /// Get a scratch buffer of at least `size` bytes — pulled from the pool
    /// or freshly allocated. Returned to the pool by `release_scratch`.
    fn take_scratch(&self, size: usize) -> Result<aruminium::Buffer, BackendError> {
        if let Some(b) = self.scratch.pop(size) {
            return Ok(b);
        }
        self.device.alloc(BufferPool::class_for(size))
    }
    fn release_scratch(&self, buf: aruminium::Buffer) {
        self.scratch.push(buf);
    }

    /// Owned-or-borrowed Metal buffer reference. Returns Borrowed for tensors
    /// already GPU-resident (zero copy) and Owned for host tensors (one upload).
    fn buf_ref<'a>(&self, t: &'a Tensor) -> Result<BufRef<'a>, BackendError> {
        match &t.data {
            TensorData::Host(bytes) => {
                let buf = self
                    .device
                    .gpu
                    .buffer_with_data(bytes.as_slice())
                    .map_err(|e| BackendError::Internal(format!("buffer_with_data: {e}")))?;
                Ok(BufRef::Owned(buf))
            }
            TensorData::Backend(b) => {
                let h = b.as_any().downcast_ref::<HcBuffer>().ok_or_else(|| {
                    BackendError::Internal("honeycrisp: tensor on a different backend".into())
                })?;
                Ok(BufRef::Borrowed(&h.buffer))
            }
        }
    }

    /// Get (or lazily create + cache) a transposed Q8 down_proj weight buffer.
    /// The transposed layout is [n_blocks, n_rows, 34] vs original [n_rows, n_blocks, 34].
    /// Returns a raw pointer into the cache (stable — Box<Buffer> in HashMap never moves).
    fn get_or_transpose_down_w(
        &self,
        orig: &aruminium::Buffer,
        n_rows: usize,
        n_blk: usize,
    ) -> *const aruminium::Buffer {
        let key = orig.as_bytes().as_ptr() as usize;
        let mut guard = self.transposed_down_w.lock().unwrap();
        let cache = &mut guard.0;
        if let Some(b) = cache.get(&key) {
            return b.as_ref() as *const aruminium::Buffer;
        }
        let src = orig.as_bytes();
        let total = n_rows * n_blk * 34;
        let transposed = transpose_q8_blocks(&src[..total], n_rows, n_blk);
        let gpu_buf = self.device.gpu.buffer_with_data(&transposed)
            .expect("transposed down_w alloc");
        let boxed = Box::new(gpu_buf);
        let ptr = boxed.as_ref() as *const aruminium::Buffer;
        cache.insert(key, boxed);
        ptr
    }

    /// Legacy upload helper: always returns an owned buffer (used where the
    /// dispatch path requires an owned buffer to extend its lifetime).
    /// Prefer `buf_ref` for new code.
    fn upload_tensor(&self, t: &Tensor) -> Result<aruminium::Buffer, BackendError> {
        match &t.data {
            TensorData::Host(bytes) => self
                .device
                .gpu
                .buffer_with_data(bytes.as_slice())
                .map_err(|e| BackendError::Internal(format!("buffer_with_data: {e}"))),
            TensorData::Backend(b) => {
                if let Some(h) = b.as_any().downcast_ref::<HcBuffer>() {
                    let bytes = &h.buffer.as_bytes()[..h.bytes];
                    self.device
                        .gpu
                        .buffer_with_data(bytes)
                        .map_err(|e| BackendError::Internal(format!("re-upload: {e}")))
                } else {
                    Err(BackendError::Internal(
                        "honeycrisp: tensor on a different backend".into(),
                    ))
                }
            }
        }
    }

    fn read_f32(&self, buf: &aruminium::Buffer, n: usize) -> Vec<f32> {
        buf.read(|bytes| {
            let needed = n * 4;
            bytemuck::cast_slice::<u8, f32>(&bytes[..needed]).to_vec()
        })
    }

    /// Wrap a GPU output buffer as a Backend Tensor. Caller must guarantee the
    /// buffer has been written by a completed dispatch (waited inside batch_raw).
    fn wrap_output(&self, buf: aruminium::Buffer, shape: Vec<usize>, dtype: DType) -> Tensor {
        let bytes = dtype.bytes_for(crate::core::tensor::numel(&shape));
        let handle = HcBuffer { buffer: buf, bytes };
        Tensor { shape, dtype, data: TensorData::Backend(Arc::new(handle)) }
    }

    /// CPU fallback for quant_matmul. If w is GPU-resident (pre-uploaded),
    /// reads the raw bytes back from the Metal buffer first.
    fn cpu_quant_matmul(&self, x: &Tensor, w: &Tensor) -> Result<Tensor, BackendError> {
        match &w.data {
            TensorData::Host(_) => self.cpu.quant_matmul(x, w),
            TensorData::Backend(b) => {
                let h = b.as_any().downcast_ref::<HcBuffer>().ok_or_else(|| {
                    BackendError::Internal("honeycrisp cpu_quant_matmul: unknown tensor".into())
                })?;
                let bytes: Arc<Vec<u8>> = Arc::new(h.buffer.read(|b| b.to_vec()));
                let w_host = Tensor {
                    shape: w.shape.clone(),
                    dtype: w.dtype,
                    data: TensorData::Host(bytes),
                };
                self.cpu.quant_matmul(x, &w_host)
            }
        }
    }

    fn download_tensor(&self, t: &Tensor) -> Result<Tensor, BackendError> {
        match &t.data {
            TensorData::Host(_) => Ok(t.clone()),
            TensorData::Backend(b) => {
                let h = b.as_any().downcast_ref::<HcBuffer>().ok_or_else(|| {
                    BackendError::Internal("honeycrisp: unknown backend tensor".into())
                })?;
                if t.dtype != DType::F32 {
                    return Err(BackendError::Internal(
                        "honeycrisp: non-F32 GPU download not implemented".into(),
                    ));
                }
                Ok(Tensor::from_f32(t.shape.clone(), self.read_f32(&h.buffer, t.numel())))
            }
        }
    }
}

impl Backend for HoneycrispBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Honeycrisp
    }

    fn uploads_quant_weights(&self) -> bool { true }

    fn supports_gpu_attention(&self) -> bool { true }

    fn reset_gpu_kv_cache(&self) {
        // Drop all AttnStates — next call rebuilds with zeroed caches.
        // Cheap because allocations are pooled in the Metal driver's free-list.
        self.attn.lock().unwrap().clear();
    }

    fn fused_attn_oproj_residual(
        &self,
        q: &Tensor,
        k: &Tensor,
        v: &Tensor,
        hidden_in: &Tensor,
        o_proj_w: &Tensor,
        layer_idx: usize,
        position: usize,
        num_heads: u32,
        kv_heads: u32,
        head_dim: u32,
        max_seq: u32,
        scale: f32,
        window: u32,
    ) -> Result<Tensor, BackendError> {
        // o_proj must be Q8 or Q4, GPU-resident. Otherwise fall back.
        let q8 = matches!(o_proj_w.dtype, DType::Q8);
        let q4 = matches!(o_proj_w.dtype, DType::Q4);
        let on_gpu = matches!(o_proj_w.data, TensorData::Backend(_));
        if !(q8 || q4) || !on_gpu || hidden_in.dtype != DType::F32 {
            return Backend::fused_attn_oproj_residual(
                self, q, k, v, hidden_in, o_proj_w,
                layer_idx, position,
                num_heads, kv_heads, head_dim, max_seq, scale, window,
            );
        }
        // Ensure attention state has a slot for layer_idx.
        {
            let guard = self.attn.lock().unwrap();
            let needs_init = !guard.iter().any(|s|
                s.num_heads == num_heads && s.kv_heads == kv_heads
                && s.head_dim == head_dim && s.max_seq == max_seq
                && s.layer_slot.contains_key(&layer_idx));
            if needs_init {
                drop(guard);
                let g = self.attn_state(num_heads, kv_heads, head_dim, max_seq, &[layer_idx], 0, 0.0)?;
                drop(g);
            }
        }
        let guard = self.attn.lock().unwrap();
        let st = guard.iter().find(|s|
            s.num_heads == num_heads && s.kv_heads == kv_heads
            && s.head_dim == head_dim && s.max_seq == max_seq
        ).expect("attn_state ensured above");
        let kv_slot = *st.layer_slot.get(&layer_idx).expect("slot ensured above");

        let q_buf = self.buf_ref(q)?;
        let k_buf = self.buf_ref(k)?;
        let v_buf = self.buf_ref(v)?;
        let hidden_buf = self.buf_ref(hidden_in)?;
        let o_w_h = match &o_proj_w.data {
            TensorData::Backend(b) => b.as_any().downcast_ref::<HcBuffer>().unwrap(),
            _ => unreachable!(),
        };

        let nh_hd = (num_heads * head_dim) as usize;
        let attn_buf = self.take_scratch(nh_hd * 4)?;
        let oproj_buf = self.take_scratch(nh_hd * 4)?;
        let out_buf = self.device.alloc(nh_hd * 4)?;

        let total_seq = (position + 1) as u32;

        // Q8/Q4 dispatch params
        let block_size = if q8 {
            kernels::q8_matmul::BLOCK_SIZE
        } else {
            kernels::q4_matmul::BLOCK_SIZE
        };
        let n_blocks_oproj = (o_proj_w.shape[1] / block_size) as u32;
        let oproj_n = o_proj_w.shape[0] as u32;
        let oproj_pipe = if q8 { &self.pipe_q8.0 } else { &self.pipe_q4.0 };
        let oproj_simds = if q8 {
            kernels::q8_matmul::SIMDS_PER_GROUP
        } else {
            kernels::q4_matmul::SIMDS_PER_GROUP
        };
        let oproj_n_dst = if q8 {
            kernels::q8_matmul::N_DST
        } else {
            kernels::q4_matmul::N_DST
        };

        unsafe {
            aruminium::autorelease_pool(|| {
                self.device.dispatch.batch_raw(|enc| {
                    // 1) kv_append k
                    {
                        #[repr(C)]
                        #[derive(Clone, Copy)]
                        struct P { position: u32, p0: u32, p1: u32, p2: u32 }
                        let p = P { position: position as u32, p0: 0, p1: 0, p2: 0 };
                        enc.bind(&st.pipe_kv_append.0);
                        enc.bind_buffer(k_buf.as_buffer(), 0, 0);
                        enc.bind_buffer(&st.k_caches[kv_slot], 0, 1);
                        let bytes = std::slice::from_raw_parts(
                            &p as *const P as *const u8, std::mem::size_of::<P>(),
                        );
                        enc.push(bytes, 2);
                        enc.launch_groups(
                            (1, kv_heads as usize, 1), (head_dim as usize, 1, 1),
                        );
                    }
                    // 2) kv_append v
                    {
                        #[repr(C)]
                        #[derive(Clone, Copy)]
                        struct P { position: u32, p0: u32, p1: u32, p2: u32 }
                        let p = P { position: position as u32, p0: 0, p1: 0, p2: 0 };
                        enc.bind(&st.pipe_kv_append.0);
                        enc.bind_buffer(v_buf.as_buffer(), 0, 0);
                        enc.bind_buffer(&st.v_caches[kv_slot], 0, 1);
                        let bytes = std::slice::from_raw_parts(
                            &p as *const P as *const u8, std::mem::size_of::<P>(),
                        );
                        enc.push(bytes, 2);
                        enc.launch_groups(
                            (1, kv_heads as usize, 1), (head_dim as usize, 1, 1),
                        );
                    }
                    // 3) attention → attn_buf
                    {
                        #[repr(C)]
                        #[derive(Clone, Copy)]
                        struct P { total_seq: u32, window: u32, scale: f32, pad: u32 }
                        let p = P { total_seq, window, scale, pad: 0 };
                        enc.bind(&st.pipe_attn.0);
                        enc.bind_buffer(q_buf.as_buffer(), 0, 0);
                        enc.bind_buffer(&st.k_caches[kv_slot], 0, 1);
                        enc.bind_buffer(&st.v_caches[kv_slot], 0, 2);
                        enc.bind_buffer(&attn_buf, 0, 3);
                        let bytes = std::slice::from_raw_parts(
                            &p as *const P as *const u8, std::mem::size_of::<P>(),
                        );
                        enc.push(bytes, 4);
                        enc.launch_groups((num_heads as usize, 1, 1), (32, 1, 1));
                    }
                    // 4) o_proj quant matmul (attn_buf → oproj_buf)
                    {
                        let rows_per_tg = oproj_simds * oproj_n_dst;
                        let groups_x = (oproj_n + rows_per_tg - 1) / rows_per_tg;
                        let threads_per_group = oproj_simds * 32;
                        #[repr(C)]
                        #[derive(Clone, Copy)]
                        struct Dims { batch: u32, n_rows: u32, n_blocks: u32, pad: u32 }
                        let dims = Dims { batch: 1, n_rows: oproj_n, n_blocks: n_blocks_oproj, pad: 0 };
                        enc.bind(oproj_pipe);
                        enc.bind_buffer(&attn_buf, 0, 0);
                        enc.bind_buffer(&o_w_h.buffer, 0, 1);
                        enc.bind_buffer(&oproj_buf, 0, 2);
                        let bytes = std::slice::from_raw_parts(
                            &dims as *const Dims as *const u8, std::mem::size_of::<Dims>(),
                        );
                        enc.push(bytes, 3);
                        enc.launch_groups(
                            (groups_x as usize, 1, 1),
                            (threads_per_group as usize, 1, 1),
                        );
                    }
                    // 5) residual add: out = hidden_in + oproj_buf
                    {
                        let n = nh_hd as u32;
                        #[repr(C)]
                        #[derive(Clone, Copy)]
                        struct P { n: u32, a_len: u32, b_len: u32, pad: u32 }
                        let p = P { n, a_len: n, b_len: n, pad: 0 };
                        enc.bind(&self.pipe_add.0);
                        enc.bind_buffer(hidden_buf.as_buffer(), 0, 0);
                        enc.bind_buffer(&oproj_buf, 0, 1);
                        enc.bind_buffer(&out_buf, 0, 2);
                        let bytes = std::slice::from_raw_parts(
                            &p as *const P as *const u8, std::mem::size_of::<P>(),
                        );
                        enc.push(bytes, 3);
                        enc.launch_groups((((n as usize) + 63) / 64, 1, 1), (64, 1, 1));
                    }
                });
            });
        }

        self.release_scratch(attn_buf);
        self.release_scratch(oproj_buf);
        Ok(self.wrap_output(out_buf, hidden_in.shape.clone(), DType::F32))
    }

    fn gpu_attention(
        &self,
        q: &Tensor,
        k: &Tensor,
        v: &Tensor,
        layer_idx: usize,
        position: usize,
        num_heads: u32,
        kv_heads: u32,
        head_dim: u32,
        max_seq: u32,
        scale: f32,
        window: u32,
    ) -> Result<Tensor, BackendError> {
        // n_layers is unknown here — caller (forward) will pre-init via a
        // dummy call OR we infer from layer_idx + 1. We grow on demand.
        // Ensure attention state has a slot for layer_idx.
        {
            let guard = self.attn.lock().unwrap();
            let needs_init = !guard.iter().any(|s|
                s.num_heads == num_heads && s.kv_heads == kv_heads
                && s.head_dim == head_dim && s.max_seq == max_seq
                && s.layer_slot.contains_key(&layer_idx));
            if needs_init {
                drop(guard);
                let g = self.attn_state(num_heads, kv_heads, head_dim, max_seq, &[layer_idx], 0, 0.0)?;
                drop(g);
            }
        }

        let guard = self.attn.lock().unwrap();
        let st = guard.iter().find(|s|
            s.num_heads == num_heads && s.kv_heads == kv_heads
            && s.head_dim == head_dim && s.max_seq == max_seq
        ).expect("attn_state ensured above");
        let kv_slot = *st.layer_slot.get(&layer_idx).expect("slot ensured above");

        let q_buf = self.buf_ref(q)?;
        let k_buf = self.buf_ref(k)?;
        let v_buf = self.buf_ref(v)?;
        let out_buf = self.device.alloc((num_heads as usize * head_dim as usize * 4).max(4))?;

        let total_seq = (position + 1) as u32;

        unsafe {
            aruminium::autorelease_pool(|| {
                // All 3 dispatches in ONE Metal command buffer + ONE wait.
                // Metal handles the read-after-write hazard on the cache buffer
                // automatically because dispatches share an encoder.
                self.device.dispatch.batch_raw(|enc| {
                    // 1) Append k → k_cache[layer_idx] at position
                    {
                        #[repr(C)]
                        #[derive(Clone, Copy)]
                        struct P { position: u32, p0: u32, p1: u32, p2: u32 }
                        let p = P { position: position as u32, p0: 0, p1: 0, p2: 0 };
                        enc.bind(&st.pipe_kv_append.0);
                        enc.bind_buffer(k_buf.as_buffer(), 0, 0);
                        enc.bind_buffer(&st.k_caches[kv_slot], 0, 1);
                        let bytes = std::slice::from_raw_parts(
                            &p as *const P as *const u8,
                            std::mem::size_of::<P>(),
                        );
                        enc.push(bytes, 2);
                        enc.launch_groups(
                            (head_dim as usize, kv_heads as usize, 1),
                            (1, 1, 1),
                        );
                    }
                    // 2) Append v → v_cache[layer_idx] at position
                    {
                        #[repr(C)]
                        #[derive(Clone, Copy)]
                        struct P { position: u32, p0: u32, p1: u32, p2: u32 }
                        let p = P { position: position as u32, p0: 0, p1: 0, p2: 0 };
                        enc.bind(&st.pipe_kv_append.0);
                        enc.bind_buffer(v_buf.as_buffer(), 0, 0);
                        enc.bind_buffer(&st.v_caches[kv_slot], 0, 1);
                        let bytes = std::slice::from_raw_parts(
                            &p as *const P as *const u8,
                            std::mem::size_of::<P>(),
                        );
                        enc.push(bytes, 2);
                        enc.launch_groups(
                            (head_dim as usize, kv_heads as usize, 1),
                            (1, 1, 1),
                        );
                    }
                    // 3) Attention: scores = q @ k_cache^T → softmax → @ v_cache
                    {
                        #[repr(C)]
                        #[derive(Clone, Copy)]
                        struct P { total_seq: u32, window: u32, scale: f32, pad: u32 }
                        let p = P { total_seq, window, scale, pad: 0 };
                        enc.bind(&st.pipe_attn.0);
                        enc.bind_buffer(q_buf.as_buffer(), 0, 0);
                        enc.bind_buffer(&st.k_caches[kv_slot], 0, 1);
                        enc.bind_buffer(&st.v_caches[kv_slot], 0, 2);
                        enc.bind_buffer(&out_buf, 0, 3);
                        let bytes = std::slice::from_raw_parts(
                            &p as *const P as *const u8,
                            std::mem::size_of::<P>(),
                        );
                        enc.push(bytes, 4);
                        enc.launch_groups((num_heads as usize, 1, 1), (32, 1, 1));
                    }
                });
            });
        }

        let result = self.wrap_output(out_buf, vec![1, (num_heads * head_dim) as usize], DType::F32);

        Ok(result)
    }

    fn supports(&self, op: &Op, inputs: &[&Tensor]) -> bool {
        match op {
            // Only ops large enough to amortize per-dispatch wait cost.
            // RmsNorm uses parallel reduction over D; even tiny ones beat CPU
            // upload-roundtrip for f32. Add/Silu are too cheap and lose to CPU.
            Op::Matmul | Op::RmsNorm { .. } | Op::Silu
                if inputs.iter().all(|t| t.dtype == DType::F32) =>
            {
                true
            }
            _ => false,
        }
    }

    fn execute(&self, op: &Op, inputs: &[&Tensor]) -> Result<Vec<Tensor>, BackendError> {
        if !self.supports(op, inputs) {
            let materialized: Result<Vec<Tensor>, BackendError> =
                inputs.iter().map(|t| self.download_tensor(t)).collect();
            let materialized = materialized?;
            let refs: Vec<&Tensor> = materialized.iter().collect();
            return self.cpu.execute(op, &refs);
        }

        match op {
            Op::Matmul => {
                if inputs.len() != 2 {
                    return Err(BackendError::InvalidInput {
                        op: "Matmul",
                        reason: format!("expected 2 inputs, got {}", inputs.len()),
                    });
                }
                let x = inputs[0];
                let w = inputs[1];
                if w.rank() != 2 || x.shape.last() != Some(&w.shape[1]) {
                    return Err(BackendError::ShapeMismatch {
                        op: "Matmul",
                        expected: vec![0, w.shape[1]],
                        got: x.shape.clone(),
                    });
                }
                let batch: u32 = x.shape[..x.shape.len() - 1].iter().product::<usize>() as u32;
                let n = w.shape[0] as u32;
                let k = w.shape[1] as u32;
                let x_buf = self.buf_ref(x)?;
                let w_buf = self.buf_ref(w)?;
                let out_buf = kernels::matmul::dispatch(
                    &self.device, &self.pipe_matmul.0,
                    x_buf.as_buffer(), w_buf.as_buffer(), batch, n, k,
                )?;
                let mut out_shape = x.shape.clone();
                *out_shape.last_mut().unwrap() = n as usize;
                Ok(vec![self.wrap_output(out_buf, out_shape, DType::F32)])
            }
            Op::RmsNorm { eps } => {
                let x = inputs[0];
                let g = inputs[1];
                let d = g.shape[0] as u32;
                let batch: u32 = x.shape[..x.shape.len() - 1].iter().product::<usize>() as u32;
                let x_buf = self.buf_ref(x)?;
                let g_buf = self.buf_ref(g)?;
                let out_buf = kernels::rmsnorm::dispatch(
                    &self.device, &self.pipe_rmsnorm.0,
                    x_buf.as_buffer(), g_buf.as_buffer(), batch, d, *eps,
                )?;
                Ok(vec![self.wrap_output(out_buf, x.shape.clone(), DType::F32)])
            }
            Op::Silu => {
                let x = inputs[0];
                let n = x.numel() as u32;
                let x_buf = self.buf_ref(x)?;
                let out_buf = kernels::silu::dispatch(&self.device, &self.pipe_silu.0, x_buf.as_buffer(), n)?;
                Ok(vec![self.wrap_output(out_buf, x.shape.clone(), DType::F32)])
            }
            Op::Add => {
                if inputs.len() != 2 {
                    return Err(BackendError::InvalidInput {
                        op: "Add",
                        reason: format!("expected 2 inputs, got {}", inputs.len()),
                    });
                }
                let a = inputs[0];
                let b = inputs[1];
                // Output shape = max(a, b) per dim. We support: same-shape, or
                // b broadcast along leading dims (e.g. bias [D] added to [B, D]).
                let n = a.numel().max(b.numel()) as u32;
                let a_len = a.numel() as u32;
                let b_len = b.numel() as u32;
                let a_buf = self.buf_ref(a)?;
                let b_buf = self.buf_ref(b)?;
                let out_buf = kernels::elementwise::dispatch_add(
                    &self.device, &self.pipe_add.0,
                    a_buf.as_buffer(), b_buf.as_buffer(), n, a_len, b_len,
                )?;
                let out_shape = if a.numel() >= b.numel() { a.shape.clone() } else { b.shape.clone() };
                Ok(vec![self.wrap_output(out_buf, out_shape, DType::F32)])
            }
            _ => self.cpu.execute(op, inputs),
        }
    }

    fn upload(
        &self,
        bytes: &[u8],
        shape: Vec<usize>,
        dtype: DType,
    ) -> Result<Tensor, BackendError> {
        let n_bytes = bytes.len();
        let buffer = self
            .device
            .gpu
            .buffer_with_data(bytes)
            .map_err(|e| BackendError::Internal(format!("buffer_with_data: {e}")))?;
        let handle = HcBuffer { buffer, bytes: n_bytes };
        Ok(Tensor {
            shape,
            dtype,
            data: TensorData::Backend(Arc::new(handle)),
        })
    }

    fn download_f32(&self, t: &Tensor) -> Result<Vec<f32>, BackendError> {
        match &t.data {
            TensorData::Host(_) => self.cpu.download_f32(t),
            TensorData::Backend(b) => {
                let h = b.as_any().downcast_ref::<HcBuffer>().ok_or_else(|| {
                    BackendError::Internal("honeycrisp: unknown tensor".into())
                })?;
                Ok(self.read_f32(&h.buffer, t.numel()))
            }
        }
    }

    fn quant_matmul(&self, x: &Tensor, w: &Tensor) -> Result<Tensor, BackendError> {
        let n = w.shape[0];
        let k = w.shape[1];

        // QuantKind selects the GPU dispatch path; everything else falls back to CPU.
        enum QuantKind { Q4K, Q6K, Q8, Q4 }
        let kind = match w.dtype {
            DType::Q4_K if k % 256 == 0 => QuantKind::Q4K,
            DType::Q6_K if k % 256 == 0 => QuantKind::Q6K,
            DType::Q8   if k % kernels::q8_matmul::BLOCK_SIZE == 0 => QuantKind::Q8,
            DType::Q4   if k % kernels::q4_matmul::BLOCK_SIZE == 0 => QuantKind::Q4,
            _ => return self.cpu_quant_matmul(x, w),
        };
        let n_blocks: usize = match kind {
            QuantKind::Q4K | QuantKind::Q6K => k / 256,
            QuantKind::Q8 => k / kernels::q8_matmul::BLOCK_SIZE,
            QuantKind::Q4 => k / kernels::q4_matmul::BLOCK_SIZE,
        };

        let batch = x.shape[..x.shape.len() - 1].iter().product::<usize>() as u32;
        let x_buf = self.buf_ref(x)?;
        let w_buf = self.buf_ref(w)?;

        let out_buf = match kind {
            QuantKind::Q4K => kernels::q4k_matmul::dispatch(
                &self.device, &self.pipe_q4k.0,
                x_buf.as_buffer(), w_buf.as_buffer(), batch, n as u32, n_blocks as u32,
            )?,
            QuantKind::Q6K => kernels::q6k_matmul::dispatch(
                &self.device, &self.pipe_q6k.0,
                x_buf.as_buffer(), w_buf.as_buffer(), batch, n as u32, n_blocks as u32,
            )?,
            QuantKind::Q8 => {
                // Select kernel based on n_blocks and n_rows.
                // mb192: TG-cached with MAX_BLOCKS=192 (24 KB TG) covers 128<n_blk≤192
                // (e.g. Gemma-4 n_blk_kd=168). For larger n_blocks or many output rows
                // (lm_head), LARGE has better L2 reuse across threadgroups.
                let n_rows = n as usize;
                let use_large = n_blocks > 192
                    || n_rows > (kernels::q8_matmul::SIMDS_PER_GROUP as usize) * 64;
                let pipe = if use_large {
                    &self.pipe_q8_large.0
                } else if n_blocks > kernels::q8_matmul::TG_MAX_BLOCKS {
                    &self.pipe_q8_mb192.0
                } else {
                    &self.pipe_q8.0
                };
                kernels::q8_matmul::dispatch(
                    &self.device, pipe,
                    x_buf.as_buffer(), w_buf.as_buffer(), batch, n as u32, n_blocks as u32,
                )?
            }
            QuantKind::Q4 => {
                // Mirror Q8 logic: for large output matrices (e.g. lm_head with 151936 rows)
                // the no-cache LARGE kernel lets GPU L2 cache serve the shared x vector
                // across all TGs — far more efficient than reloading per-TG.
                let use_large = n_blocks > kernels::q4_matmul::TG_MAX_BLOCKS
                    || n as usize > (kernels::q4_matmul::SIMDS_PER_GROUP as usize) * 64;
                let pipe = if use_large { &self.pipe_q4_large.0 } else { &self.pipe_q4.0 };
                kernels::q4_matmul::dispatch(
                    &self.device, pipe,
                    x_buf.as_buffer(), w_buf.as_buffer(), batch, n as u32, n_blocks as u32,
                )?
            }
        };

        let mut out_shape = x.shape.clone();
        *out_shape.last_mut().unwrap() = n;
        Ok(self.wrap_output(out_buf, out_shape, DType::F32))
    }

    /// Batched: encode all `ws` matmuls into ONE command buffer, single wait.
    /// Saves the fixed per-dispatch submit + wait overhead (~50–100 µs each).
    fn quant_matmul_multi(
        &self,
        x: &Tensor,
        ws: &[&Tensor],
    ) -> Result<Vec<Tensor>, BackendError> {
        if ws.is_empty() { return Ok(Vec::new()); }

        // Homogeneous Q8/Q4 batch with same K. Otherwise per-call routing.
        let k = ws[0].shape[1];
        let kind0 = ws[0].dtype;
        let supported = ws.iter().all(|w| {
            w.dtype == kind0
                && w.shape[1] == k
                && (matches!(w.dtype, DType::Q8) && w.shape[1] % kernels::q8_matmul::BLOCK_SIZE == 0
                    || matches!(w.dtype, DType::Q4) && w.shape[1] % kernels::q4_matmul::BLOCK_SIZE == 0)
        });
        if !supported {
            return ws.iter().map(|w| self.quant_matmul(x, w)).collect();
        }
        let all_backend = ws.iter().all(|w| matches!(w.data, TensorData::Backend(_)));
        if !all_backend {
            return ws.iter().map(|w| self.quant_matmul(x, w)).collect();
        }

        let batch = x.shape[..x.shape.len() - 1].iter().product::<usize>() as u32;
        let block_size = match kind0 {
            DType::Q8 => kernels::q8_matmul::BLOCK_SIZE,
            DType::Q4 => kernels::q4_matmul::BLOCK_SIZE,
            _ => unreachable!(),
        };
        let pipe = match kind0 {
            DType::Q8 => &self.pipe_q8.0,
            DType::Q4 => &self.pipe_q4.0,
            _ => unreachable!(),
        };
        let simds_per_group = match kind0 {
            DType::Q8 => kernels::q8_matmul::SIMDS_PER_GROUP,
            DType::Q4 => kernels::q4_matmul::SIMDS_PER_GROUP,
            _ => unreachable!(),
        };
        let n_dst_q0 = match kind0 {
            DType::Q8 => kernels::q8_matmul::N_DST,
            DType::Q4 => kernels::q4_matmul::N_DST,
            _ => unreachable!(),
        };
        let n_blocks = (k / block_size) as u32;

        // Upload x once (or borrow if already on GPU).
        let x_buf = self.buf_ref(x)?;

        // Borrow weight buffers in place — no copy.
        let mut weight_refs: Vec<&aruminium::Buffer> = Vec::with_capacity(ws.len());
        let mut ns: Vec<usize> = Vec::with_capacity(ws.len());
        for w in ws {
            let h = match &w.data {
                TensorData::Backend(b) => b.as_any().downcast_ref::<HcBuffer>().ok_or_else(|| {
                    BackendError::Internal("quant_matmul_multi: wrong backend tensor".into())
                })?,
                _ => unreachable!("all_backend guard above"),
            };
            weight_refs.push(&h.buffer);
            ns.push(w.shape[0]);
        }
        let mut out_bufs: Vec<aruminium::Buffer> = Vec::with_capacity(ws.len());
        for &n in &ns {
            out_bufs.push(self.device.alloc((batch as usize * n * 4).max(4))?);
        }

        // ONE command buffer, encode all ws dispatches, ONE wait.
        unsafe {
            aruminium::autorelease_pool(|| {
                self.device.dispatch.batch_raw(|enc| {
                    for i in 0..ws.len() {
                        let n = ns[i] as u32;
                        let total_rows = batch * n;
                        let rows_per_tg = simds_per_group * n_dst_q0;
                        let groups_x = (total_rows + rows_per_tg - 1) / rows_per_tg;
                        let threads_per_group = simds_per_group * 32;

                        #[repr(C)]
                        #[derive(Clone, Copy)]
                        struct Dims { batch: u32, n_rows: u32, n_blocks: u32, pad: u32 }
                        let dims = Dims { batch, n_rows: n, n_blocks, pad: 0 };

                        enc.bind(pipe);
                        enc.bind_buffer(x_buf.as_buffer(), 0, 0);
                        enc.bind_buffer(weight_refs[i], 0, 1);
                        enc.bind_buffer(&out_bufs[i], 0, 2);
                        let bytes = std::slice::from_raw_parts(
                            &dims as *const Dims as *const u8,
                            std::mem::size_of::<Dims>(),
                        );
                        enc.push(bytes, 3);
                        enc.launch_groups(
                            (groups_x as usize, 1, 1),
                            (threads_per_group as usize, 1, 1),
                        );
                    }
                });
            });
        }

        // Wrap outputs as backend tensors (shared memory — readable after wait).
        let mut results = Vec::with_capacity(ws.len());
        for (out_buf, n) in out_bufs.into_iter().zip(ns.into_iter()) {
            let mut out_shape = x.shape.clone();
            *out_shape.last_mut().unwrap() = n;
            results.push(self.wrap_output(out_buf, out_shape, DType::F32));
        }
        Ok(results)
    }

    // silu_mul: GPU only when called inside a fused chain (no individual override).
    // Standalone call still uses CPU through trait default for shape preservation.

    /// Fused: input_norm → q/k/v matmul → qk_norm Q/K — ONE command buffer.
    fn fused_norm_qkv_qknorm(
        &self,
        hidden: &Tensor,
        input_norm_gamma: &Tensor,
        q_proj_w: &Tensor,
        k_proj_w: &Tensor,
        v_proj_w: &Tensor,
        q_norm_gamma: &Tensor,
        k_norm_gamma: &Tensor,
        eps: f32,
        num_q_heads: usize,
        num_k_heads: usize,
        head_dim: usize,
    ) -> Result<(Tensor, Tensor, Tensor), BackendError> {
        // Eligibility: homogeneous Q8 or Q4 weights, all GPU-resident.
        let kind0 = q_proj_w.dtype;
        let same_kind = k_proj_w.dtype == kind0 && v_proj_w.dtype == kind0;
        let valid_kind = matches!(kind0, DType::Q8 | DType::Q4);
        let on_gpu = matches!(q_proj_w.data, TensorData::Backend(_))
            && matches!(k_proj_w.data, TensorData::Backend(_))
            && matches!(v_proj_w.data, TensorData::Backend(_));
        let block_size = match kind0 {
            DType::Q8 => kernels::q8_matmul::BLOCK_SIZE,
            DType::Q4 => kernels::q4_matmul::BLOCK_SIZE,
            _ => 0,
        };
        let aligned = block_size > 0 && q_proj_w.shape[1] % block_size == 0;
        if !same_kind || !valid_kind || !on_gpu || !aligned
            || hidden.dtype != DType::F32 || input_norm_gamma.dtype != DType::F32
            || q_norm_gamma.dtype != DType::F32 || k_norm_gamma.dtype != DType::F32
        {
            return Backend::fused_norm_qkv_qknorm(
                self, hidden, input_norm_gamma,
                q_proj_w, k_proj_w, v_proj_w,
                q_norm_gamma, k_norm_gamma,
                eps, num_q_heads, num_k_heads, head_dim,
            );
        }
        let pipe = match kind0 {
            DType::Q8 => &self.pipe_q8.0,
            DType::Q4 => &self.pipe_q4.0,
            _ => unreachable!(),
        };
        let simds_per_group = match kind0 {
            DType::Q8 => kernels::q8_matmul::SIMDS_PER_GROUP,
            DType::Q4 => kernels::q4_matmul::SIMDS_PER_GROUP,
            _ => unreachable!(),
        };
        let n_dst_q0 = match kind0 {
            DType::Q8 => kernels::q8_matmul::N_DST,
            DType::Q4 => kernels::q4_matmul::N_DST,
            _ => unreachable!(),
        };

        let batch = hidden.shape[..hidden.shape.len() - 1].iter().product::<usize>() as u32;
        let d = input_norm_gamma.shape[0] as u32;
        let n_blocks_d = (d as usize / block_size) as u32;
        let q_n = q_proj_w.shape[0] as u32;
        let k_n = k_proj_w.shape[0] as u32;
        let v_n = v_proj_w.shape[0] as u32;

        let h_buf = self.buf_ref(hidden)?;
        let g_buf = self.buf_ref(input_norm_gamma)?;
        let qn_buf = self.buf_ref(q_norm_gamma)?;
        let kn_buf = self.buf_ref(k_norm_gamma)?;
        let q_w_h = match &q_proj_w.data {
            TensorData::Backend(b) => b.as_any().downcast_ref::<HcBuffer>().unwrap(),
            _ => unreachable!(),
        };
        let k_w_h = match &k_proj_w.data {
            TensorData::Backend(b) => b.as_any().downcast_ref::<HcBuffer>().unwrap(),
            _ => unreachable!(),
        };
        let v_w_h = match &v_proj_w.data {
            TensorData::Backend(b) => b.as_any().downcast_ref::<HcBuffer>().unwrap(),
            _ => unreachable!(),
        };

        let normed_buf = self.take_scratch((batch as usize * d as usize * 4).max(4))?;
        // Scratch q/k for the matmul output, then re-used as input to qk_norm.
        // v output stays as final tensor.
        let q_buf = self.take_scratch((batch as usize * q_n as usize * 4).max(4))?;
        let k_buf = self.take_scratch((batch as usize * k_n as usize * 4).max(4))?;
        let q_norm_out = self.device.alloc((batch as usize * q_n as usize * 4).max(4))?;
        let k_norm_out = self.device.alloc((batch as usize * k_n as usize * 4).max(4))?;
        let v_out = self.device.alloc((batch as usize * v_n as usize * 4).max(4))?;

        unsafe {
            aruminium::autorelease_pool(|| {
                self.device.dispatch.batch_raw(|enc| {
                    // 1) Input RmsNorm
                    {
                        #[repr(C)]
                        #[derive(Clone, Copy)]
                        struct P { batch: u32, d: u32, eps: f32, pad: u32 }
                        let p = P { batch, d, eps, pad: 0 };
                        enc.bind(&self.pipe_rmsnorm.0);
                        enc.bind_buffer(h_buf.as_buffer(), 0, 0);
                        enc.bind_buffer(g_buf.as_buffer(), 0, 1);
                        enc.bind_buffer(&normed_buf, 0, 2);
                        let bytes = std::slice::from_raw_parts(
                            &p as *const P as *const u8,
                            std::mem::size_of::<P>(),
                        );
                        enc.push(bytes, 3);
                        enc.launch_groups((batch as usize, 1, 1), (256, 1, 1));
                    }
                    enc.memory_barrier_buffers();
                    let dispatch_q = |enc: &aruminium::Batch,
                                      x_buf: &aruminium::Buffer,
                                      w_buf: &aruminium::Buffer,
                                      out: &aruminium::Buffer,
                                      n_rows: u32,
                                      n_blocks: u32| {
                        let total_rows = batch * n_rows;
                        let rows_per_tg = simds_per_group * n_dst_q0;
                        let groups_x = (total_rows + rows_per_tg - 1) / rows_per_tg;
                        let threads_per_group = simds_per_group * 32;
                        #[repr(C)]
                        #[derive(Clone, Copy)]
                        struct Dims { batch: u32, n_rows: u32, n_blocks: u32, pad: u32 }
                        let dims = Dims { batch, n_rows, n_blocks, pad: 0 };
                        enc.bind(pipe);
                        enc.bind_buffer(x_buf, 0, 0);
                        enc.bind_buffer(w_buf, 0, 1);
                        enc.bind_buffer(out, 0, 2);
                        let bytes = std::slice::from_raw_parts(
                            &dims as *const Dims as *const u8,
                            std::mem::size_of::<Dims>(),
                        );
                        enc.push(bytes, 3);
                        enc.launch_groups(
                            (groups_x as usize, 1, 1),
                            (threads_per_group as usize, 1, 1),
                        );
                    };
                    // 2-4) Q/K/V matmul
                    dispatch_q(enc, &normed_buf, &q_w_h.buffer, &q_buf, q_n, n_blocks_d);
                    dispatch_q(enc, &normed_buf, &k_w_h.buffer, &k_buf, k_n, n_blocks_d);
                    dispatch_q(enc, &normed_buf, &v_w_h.buffer, &v_out, v_n, n_blocks_d);
                    enc.memory_barrier_buffers();
                    // 5-6) QK norm (per-head)
                    {
                        #[repr(C)]
                        #[derive(Clone, Copy)]
                        struct P { batch: u32, d: u32, eps: f32, pad: u32 }
                        // Q: batch=num_q_heads, d=head_dim
                        let p_q = P { batch: num_q_heads as u32, d: head_dim as u32, eps, pad: 0 };
                        enc.bind(&self.pipe_rmsnorm.0);
                        enc.bind_buffer(&q_buf, 0, 0);
                        enc.bind_buffer(qn_buf.as_buffer(), 0, 1);
                        enc.bind_buffer(&q_norm_out, 0, 2);
                        let bytes = std::slice::from_raw_parts(
                            &p_q as *const P as *const u8,
                            std::mem::size_of::<P>(),
                        );
                        enc.push(bytes, 3);
                        enc.launch_groups((num_q_heads, 1, 1), (256, 1, 1));

                        // K: batch=num_k_heads, d=head_dim
                        let p_k = P { batch: num_k_heads as u32, d: head_dim as u32, eps, pad: 0 };
                        enc.bind(&self.pipe_rmsnorm.0);
                        enc.bind_buffer(&k_buf, 0, 0);
                        enc.bind_buffer(kn_buf.as_buffer(), 0, 1);
                        enc.bind_buffer(&k_norm_out, 0, 2);
                        let bytes = std::slice::from_raw_parts(
                            &p_k as *const P as *const u8,
                            std::mem::size_of::<P>(),
                        );
                        enc.push(bytes, 3);
                        enc.launch_groups((num_k_heads, 1, 1), (256, 1, 1));
                    }
                });
            });
        }

        self.release_scratch(normed_buf);
        self.release_scratch(q_buf);
        self.release_scratch(k_buf);

        let q_t = self.wrap_output(q_norm_out, vec![1, num_q_heads * head_dim], DType::F32);
        let k_t = self.wrap_output(k_norm_out, vec![1, num_k_heads * head_dim], DType::F32);
        let v_t = self.wrap_output(v_out, vec![1, v_n as usize], DType::F32);
        Ok((q_t, k_t, v_t))
    }

    /// Fused FFN: norm + gate + up + silu_mul + down — ONE command buffer.
    /// Saves ~2 waits per layer (was: norm+gate_up batch, then silu CPU, then down).
    fn fused_norm_swiglu_down(
        &self,
        hidden: &Tensor,
        post_norm_gamma: &Tensor,
        gate_w: &Tensor,
        up_w: &Tensor,
        down_w: &Tensor,
        eps: f32,
    ) -> Result<Tensor, BackendError> {
        // Eligibility check — falls back to default chain if anything off.
        let weights_q8 = matches!(gate_w.dtype, DType::Q8)
            && matches!(up_w.dtype, DType::Q8)
            && matches!(down_w.dtype, DType::Q8);
        let weights_on_gpu = matches!(gate_w.data, TensorData::Backend(_))
            && matches!(up_w.data, TensorData::Backend(_))
            && matches!(down_w.data, TensorData::Backend(_));
        let k_match = gate_w.shape[1] == up_w.shape[1]
            && gate_w.shape[0] == up_w.shape[0]
            && down_w.shape[1] == gate_w.shape[0]
            && gate_w.shape[1] == hidden.shape[hidden.shape.len() - 1];
        let aligned = gate_w.shape[1] % kernels::q8_matmul::BLOCK_SIZE == 0
            && down_w.shape[1] % kernels::q8_matmul::BLOCK_SIZE == 0;
        if !weights_q8 || !weights_on_gpu || !k_match || !aligned
            || hidden.dtype != DType::F32 || post_norm_gamma.dtype != DType::F32
        {
            return Backend::fused_norm_swiglu_down(
                self, hidden, post_norm_gamma, gate_w, up_w, down_w, eps,
            );
        }

        let batch = hidden.shape[..hidden.shape.len() - 1].iter().product::<usize>() as u32;
        let d = post_norm_gamma.shape[0] as u32;          // hidden dim
        let inter = gate_w.shape[0] as u32;               // intermediate dim
        let down_n = down_w.shape[0] as u32;              // == hidden dim
        let n_blocks_d = (d as usize / kernels::q8_matmul::BLOCK_SIZE) as u32;
        let n_blocks_inter = (inter as usize / kernels::q8_matmul::BLOCK_SIZE) as u32;

        let h_buf = self.buf_ref(hidden)?;
        let g_buf = self.buf_ref(post_norm_gamma)?;
        let gate_w_h = match &gate_w.data {
            TensorData::Backend(b) => b.as_any().downcast_ref::<HcBuffer>().unwrap(),
            _ => unreachable!(),
        };
        let up_w_h = match &up_w.data {
            TensorData::Backend(b) => b.as_any().downcast_ref::<HcBuffer>().unwrap(),
            _ => unreachable!(),
        };
        let down_w_h = match &down_w.data {
            TensorData::Backend(b) => b.as_any().downcast_ref::<HcBuffer>().unwrap(),
            _ => unreachable!(),
        };

        let normed_size = (batch as usize * d as usize * 4).max(4);
        let inter_size = (batch as usize * inter as usize * 4).max(4);
        let out_size = (batch as usize * down_n as usize * 4).max(4);
        let normed_buf = self.take_scratch(normed_size)?;
        let gate_buf = self.take_scratch(inter_size)?;
        let up_buf = self.take_scratch(inter_size)?;
        let mid_buf = self.take_scratch(inter_size)?;
        let out_buf = self.device.alloc(out_size)?;

        unsafe {
            aruminium::autorelease_pool(|| {
                self.device.dispatch.batch_raw(|enc| {
                    // 1) Post-RMS norm
                    {
                        #[repr(C)]
                        #[derive(Clone, Copy)]
                        struct P { batch: u32, d: u32, eps: f32, pad: u32 }
                        let p = P { batch, d, eps, pad: 0 };
                        enc.bind(&self.pipe_rmsnorm.0);
                        enc.bind_buffer(h_buf.as_buffer(), 0, 0);
                        enc.bind_buffer(g_buf.as_buffer(), 0, 1);
                        enc.bind_buffer(&normed_buf, 0, 2);
                        let bytes = std::slice::from_raw_parts(
                            &p as *const P as *const u8,
                            std::mem::size_of::<P>(),
                        );
                        enc.push(bytes, 3);
                        enc.launch_groups((batch as usize, 1, 1), (256, 1, 1));
                    }
                    enc.memory_barrier_buffers();
                    // helper to dispatch a q8 matmul reading from `x_buf` into `out`
                    let dispatch_q8 = |enc: &aruminium::Batch,
                                       pipe_to_use: &aruminium::Pipeline,
                                       x_buf: &aruminium::Buffer,
                                       w_buf: &aruminium::Buffer,
                                       out: &aruminium::Buffer,
                                       n_rows: u32,
                                       n_blocks: u32| {
                        let total_rows = batch * n_rows;
                        let rows_per_tg = kernels::q8_matmul::SIMDS_PER_GROUP * kernels::q8_matmul::N_DST;
                        let groups_x = (total_rows + rows_per_tg - 1) / rows_per_tg;
                        let threads_per_group = kernels::q8_matmul::SIMDS_PER_GROUP * 32;

                        #[repr(C)]
                        #[derive(Clone, Copy)]
                        struct Dims { batch: u32, n_rows: u32, n_blocks: u32, pad: u32 }
                        let dims = Dims { batch, n_rows, n_blocks, pad: 0 };
                        enc.bind(pipe_to_use);
                        enc.bind_buffer(x_buf, 0, 0);
                        enc.bind_buffer(w_buf, 0, 1);
                        enc.bind_buffer(out, 0, 2);
                        let bytes = std::slice::from_raw_parts(
                            &dims as *const Dims as *const u8,
                            std::mem::size_of::<Dims>(),
                        );
                        enc.push(bytes, 3);
                        enc.launch_groups(
                            (groups_x as usize, 1, 1),
                            (threads_per_group as usize, 1, 1),
                        );
                    };
                    // Select pipeline: use no-cache large variant when intermediate > TG_MAX_BLOCKS
                    let pipe_down = if n_blocks_inter as usize > kernels::q8_matmul::TG_MAX_BLOCKS {
                        &self.pipe_q8_large.0
                    } else {
                        &self.pipe_q8.0
                    };
                    // 2) gate = normed @ gate_w
                    dispatch_q8(enc, &self.pipe_q8.0, &normed_buf, &gate_w_h.buffer, &gate_buf, inter, n_blocks_d);
                    // 3) up = normed @ up_w
                    dispatch_q8(enc, &self.pipe_q8.0, &normed_buf, &up_w_h.buffer, &up_buf, inter, n_blocks_d);
                    enc.memory_barrier_buffers();
                    // 4) mid = silu(gate) * up
                    {
                        let n = batch * inter;
                        #[repr(C)]
                        #[derive(Clone, Copy)]
                        struct P { n: u32, pad0: u32, pad1: u32, pad2: u32 }
                        let p = P { n, pad0: 0, pad1: 0, pad2: 0 };
                        enc.bind(&self.pipe_silu_mul.0);
                        enc.bind_buffer(&gate_buf, 0, 0);
                        enc.bind_buffer(&up_buf, 0, 1);
                        enc.bind_buffer(&mid_buf, 0, 2);
                        let bytes = std::slice::from_raw_parts(
                            &p as *const P as *const u8,
                            std::mem::size_of::<P>(),
                        );
                        enc.push(bytes, 3);
                        enc.launch_groups((((n as usize) + 63) / 64, 1, 1), (64, 1, 1));
                    }
                    enc.memory_barrier_buffers();
                    // 5) out = mid @ down_w (may use large pipeline for big inter)
                    dispatch_q8(enc, pipe_down, &mid_buf, &down_w_h.buffer, &out_buf, down_n, n_blocks_inter);
                });
            });
        }

        // Return scratch buffers to the pool.
        self.release_scratch(normed_buf);
        self.release_scratch(gate_buf);
        self.release_scratch(up_buf);
        self.release_scratch(mid_buf);

        let mut out_shape = hidden.shape.clone();
        *out_shape.last_mut().unwrap() = down_n as usize;
        Ok(self.wrap_output(out_buf, out_shape, DType::F32))
    }

    fn fused_ffn_residual(
        &self,
        hidden_in: &Tensor,
        post_norm_gamma: &Tensor,
        gate_w: &Tensor,
        up_w: &Tensor,
        down_w: &Tensor,
        eps: f32,
    ) -> Result<Tensor, BackendError> {
        let q_ok = (matches!(gate_w.dtype, DType::Q8 | DType::Q4)
            && gate_w.dtype == up_w.dtype && gate_w.dtype == down_w.dtype);
        let on_gpu = matches!(gate_w.data, TensorData::Backend(_))
            && matches!(up_w.data, TensorData::Backend(_))
            && matches!(down_w.data, TensorData::Backend(_));
        if !q_ok || !on_gpu || hidden_in.dtype != DType::F32
            || post_norm_gamma.dtype != DType::F32
        {
            return Backend::fused_ffn_residual(
                self, hidden_in, post_norm_gamma, gate_w, up_w, down_w, eps,
            );
        }
        let kind = gate_w.dtype;
        let block_size = match kind {
            DType::Q8 => kernels::q8_matmul::BLOCK_SIZE,
            DType::Q4 => kernels::q4_matmul::BLOCK_SIZE,
            _ => unreachable!(),
        };
        let simds = match kind {
            DType::Q8 => kernels::q8_matmul::SIMDS_PER_GROUP,
            DType::Q4 => kernels::q4_matmul::SIMDS_PER_GROUP,
            _ => unreachable!(),
        };
        let n_dst_q = match kind {
            DType::Q8 => kernels::q8_matmul::N_DST,
            DType::Q4 => kernels::q4_matmul::N_DST,
            _ => unreachable!(),
        };
        let d = post_norm_gamma.shape[0] as u32;
        let inter = gate_w.shape[0] as u32;
        let down_n = down_w.shape[0] as u32;
        // Gate/up read from d-dim (hidden_size), down reads from inter-dim.
        // Use large (no-TG-cache) pipeline for down when inter exceeds TG_MAX_BLOCKS.
        let pipe_gate_up = match kind {
            DType::Q8 => &self.pipe_q8.0,
            DType::Q4 => &self.pipe_q4.0,
            _ => unreachable!(),
        };
        let inter_n_blocks = (inter as usize / block_size) as u32;
        let pipe_down = match kind {
            DType::Q8 if inter_n_blocks as usize > kernels::q8_matmul::TG_MAX_BLOCKS =>
                &self.pipe_q8_large.0,
            DType::Q4 if inter_n_blocks as usize > kernels::q4_matmul::TG_MAX_BLOCKS =>
                &self.pipe_q4_large.0,
            _ => pipe_gate_up,
        };
        if (gate_w.shape[1] as u32) != d || (up_w.shape[1] as u32) != d
            || (down_w.shape[1] as u32) != inter
        {
            return Backend::fused_ffn_residual(self, hidden_in, post_norm_gamma, gate_w, up_w, down_w, eps);
        }

        let h_buf = self.buf_ref(hidden_in)?;
        let g_buf = self.buf_ref(post_norm_gamma)?;
        let gate_h = match &gate_w.data {
            TensorData::Backend(b) => b.as_any().downcast_ref::<HcBuffer>().unwrap(),
            _ => unreachable!(),
        };
        let up_h = match &up_w.data {
            TensorData::Backend(b) => b.as_any().downcast_ref::<HcBuffer>().unwrap(),
            _ => unreachable!(),
        };
        let down_h = match &down_w.data {
            TensorData::Backend(b) => b.as_any().downcast_ref::<HcBuffer>().unwrap(),
            _ => unreachable!(),
        };

        let normed_buf = self.take_scratch((d * 4) as usize)?;
        let gate_buf = self.take_scratch((inter * 4) as usize)?;
        let up_buf = self.take_scratch((inter * 4) as usize)?;
        let mid_buf = self.take_scratch((inter * 4) as usize)?;
        let ffn_buf = self.take_scratch((down_n * 4) as usize)?;
        let out_buf = self.device.alloc((down_n * 4) as usize)?;

        let n_blocks_d = (d as usize / block_size) as u32;
        let n_blocks_inter = (inter as usize / block_size) as u32;

        unsafe {
            aruminium::autorelease_pool(|| {
                self.device.dispatch.batch_raw(|enc| {
                    // 1) post_norm
                    {
                        #[repr(C)]
                        #[derive(Clone, Copy)]
                        struct P { batch: u32, d: u32, eps: f32, pad: u32 }
                        let p = P { batch: 1, d, eps, pad: 0 };
                        enc.bind(&self.pipe_rmsnorm.0);
                        enc.bind_buffer(h_buf.as_buffer(), 0, 0);
                        enc.bind_buffer(g_buf.as_buffer(), 0, 1);
                        enc.bind_buffer(&normed_buf, 0, 2);
                        let bytes = std::slice::from_raw_parts(
                            &p as *const P as *const u8, std::mem::size_of::<P>(),
                        );
                        enc.push(bytes, 3);
                        enc.launch_groups((1, 1, 1), (256, 1, 1));
                    }
                    enc.memory_barrier_buffers();
                    let dispatch_q = |enc: &aruminium::Batch,
                                      pipe_to_use: &aruminium::Pipeline,
                                      x_buf: &aruminium::Buffer,
                                      w_buf: &aruminium::Buffer,
                                      out: &aruminium::Buffer,
                                      n_rows: u32,
                                      n_blocks: u32| {
                        let rows_per_tg = simds * n_dst_q;
                        let groups_x = (n_rows + rows_per_tg - 1) / rows_per_tg;
                        let threads_per_group = simds * 32;
                        #[repr(C)]
                        #[derive(Clone, Copy)]
                        struct Dims { batch: u32, n_rows: u32, n_blocks: u32, pad: u32 }
                        let dims = Dims { batch: 1, n_rows, n_blocks, pad: 0 };
                        enc.bind(pipe_to_use);
                        enc.bind_buffer(x_buf, 0, 0);
                        enc.bind_buffer(w_buf, 0, 1);
                        enc.bind_buffer(out, 0, 2);
                        let bytes = std::slice::from_raw_parts(
                            &dims as *const Dims as *const u8, std::mem::size_of::<Dims>(),
                        );
                        enc.push(bytes, 3);
                        enc.launch_groups(
                            (groups_x as usize, 1, 1),
                            (threads_per_group as usize, 1, 1),
                        );
                    };
                    // 2) gate, 3) up
                    dispatch_q(enc, pipe_gate_up, &normed_buf, &gate_h.buffer, &gate_buf, inter, n_blocks_d);
                    dispatch_q(enc, pipe_gate_up, &normed_buf, &up_h.buffer, &up_buf, inter, n_blocks_d);
                    enc.memory_barrier_buffers();
                    // 4) silu(gate) * up → mid
                    {
                        let n = inter;
                        #[repr(C)]
                        #[derive(Clone, Copy)]
                        struct P { n: u32, p0: u32, p1: u32, p2: u32 }
                        let p = P { n, p0: 0, p1: 0, p2: 0 };
                        enc.bind(&self.pipe_silu_mul.0);
                        enc.bind_buffer(&gate_buf, 0, 0);
                        enc.bind_buffer(&up_buf, 0, 1);
                        enc.bind_buffer(&mid_buf, 0, 2);
                        let bytes = std::slice::from_raw_parts(
                            &p as *const P as *const u8, std::mem::size_of::<P>(),
                        );
                        enc.push(bytes, 3);
                        enc.launch_groups((((n as usize) + 63) / 64, 1, 1), (64, 1, 1));
                    }
                    enc.memory_barrier_buffers();
                    // 5) down_proj (may use large/no-cache pipeline for big inter)
                    dispatch_q(enc, pipe_down, &mid_buf, &down_h.buffer, &ffn_buf, down_n, n_blocks_inter);
                    enc.memory_barrier_buffers();
                    // 6) residual: out = hidden_in + ffn_buf
                    {
                        let n = down_n;
                        #[repr(C)]
                        #[derive(Clone, Copy)]
                        struct P { n: u32, a_len: u32, b_len: u32, pad: u32 }
                        let p = P { n, a_len: n, b_len: n, pad: 0 };
                        enc.bind(&self.pipe_add.0);
                        enc.bind_buffer(h_buf.as_buffer(), 0, 0);
                        enc.bind_buffer(&ffn_buf, 0, 1);
                        enc.bind_buffer(&out_buf, 0, 2);
                        let bytes = std::slice::from_raw_parts(
                            &p as *const P as *const u8, std::mem::size_of::<P>(),
                        );
                        enc.push(bytes, 3);
                        enc.launch_groups((((n as usize) + 63) / 64, 1, 1), (64, 1, 1));
                    }
                });
            });
        }

        self.release_scratch(normed_buf);
        self.release_scratch(gate_buf);
        self.release_scratch(up_buf);
        self.release_scratch(mid_buf);
        self.release_scratch(ffn_buf);

        let mut out_shape = hidden_in.shape.clone();
        *out_shape.last_mut().unwrap() = down_n as usize;
        Ok(self.wrap_output(out_buf, out_shape, DType::F32))
    }

    fn forward_decode_fused_layers(
        &self,
        hidden: &Tensor,
        layers: &[crate::backend::LayerFusedInput<'_>],
        past_seq_len: usize,
        max_seq: u32,
        eps: f32,
    ) -> Result<Option<Tensor>, crate::backend::BackendError> {
        use crate::backend::BackendError;
        let t_fused_entry = std::time::Instant::now();
        let hc_timing = std::env::var("HC_TIMING").is_ok();
        if layers.is_empty() { return Ok(None); }

        // Guard: GPU KV cache is a linear (non-ring) buffer; cap at max_seq.
        if past_seq_len + 1 > max_seq as usize { return Ok(None); }

        let l0 = &layers[0];
        let kind = l0.q_proj.dtype;
        if !matches!(kind, DType::Q4 | DType::Q8) { return Ok(None); }
        let has_qk_norm      = l0.q_norm.is_some();
        let has_post_attn_n  = l0.post_attn_norm.is_some();
        let has_post_ffw_n   = l0.post_ffw_norm.is_some();
        let use_gelu_tanh    = l0.use_gelu_tanh;
        let has_qkv_bias = layers.iter().any(|l|
            l.q_bias.is_some() || l.k_bias.is_some() || l.v_bias.is_some());

        // GeluTanh + post norms require Q8 large-kd path (no Q4 variant yet).
        if use_gelu_tanh && kind == DType::Q4 { return Ok(None); }

        let block_size = match kind {
            DType::Q4 => kernels::q4_matmul::BLOCK_SIZE,
            DType::Q8 => kernels::q8_matmul::BLOCK_SIZE,
            _ => unreachable!(),
        };
        let simds = match kind {
            DType::Q4 => kernels::q4_matmul::SIMDS_PER_GROUP,
            DType::Q8 => kernels::q8_matmul::SIMDS_PER_GROUP,
            _ => unreachable!(),
        };
        let n_dst = match kind {
            DType::Q4 => kernels::q4_matmul::N_DST,
            DType::Q8 => kernels::q8_matmul::N_DST,
            _ => unreachable!(),
        };
        let pipe_q = match kind {
            DType::Q4 => &self.pipe_q4.0,
            DType::Q8 => &self.pipe_q8.0,
            _ => unreachable!(),
        };

        let k_dim      = l0.q_proj.shape[1];   // hidden_size = input feature dim
        let inter_size = l0.gate_proj.shape[0]; // FFN intermediate
        let num_heads  = l0.num_heads;
        let kv_heads   = l0.kv_heads;
        let head_dim   = l0.head_dim;
        let rope_dim   = l0.rope_dim;
        let rope_half  = (rope_dim / 2) as usize;
        let rope_theta = l0.rope_theta;
        let n          = layers.len();

        // Verify all layers are uniform and fully GPU-resident
        let on_gpu = |t: &Tensor| matches!(t.data, TensorData::Backend(_));
        for l in layers {
            if l.head_dim != head_dim || l.num_heads != num_heads || l.kv_heads != kv_heads
                || l.q_proj.shape[1] != k_dim || l.gate_proj.shape[0] != inter_size
            { return Ok(None); }
            if l.q_proj.dtype != kind || l.k_proj.dtype != kind || l.v_proj.dtype != kind
                || l.o_proj.dtype != kind || l.gate_proj.dtype != kind
                || l.up_proj.dtype != kind || l.down_proj.dtype != kind
            { return Ok(None); }
            if !on_gpu(l.q_proj) || !on_gpu(l.k_proj) || !on_gpu(l.v_proj)
                || !on_gpu(l.o_proj) || !on_gpu(l.gate_proj)
                || !on_gpu(l.up_proj) || !on_gpu(l.down_proj)
            { return Ok(None); }
            // qk_norm / post norms / activation must be consistent across layers.
            if l.q_norm.is_some() != has_qk_norm { return Ok(None); }
            if l.post_attn_norm.is_some() != has_post_attn_n { return Ok(None); }
            if l.post_ffw_norm.is_some() != has_post_ffw_n { return Ok(None); }
            if l.use_gelu_tanh != use_gelu_tanh { return Ok(None); }
            // post_attn_norm / post_ffw_norm must be GPU-resident
            if let Some(n) = l.post_attn_norm { if !on_gpu(n) { return Ok(None); } }
            if let Some(n) = l.post_ffw_norm  { if !on_gpu(n) { return Ok(None); } }
            // Q4 large-kd (n_blk_kd > 64): LARGE4_NRM/GUS_NRM not yet implemented for Q4.
            if !has_qk_norm && kind == DType::Q4 && k_dim / block_size > 64 { return Ok(None); }
        }
        if k_dim % block_size != 0 || inter_size % block_size != 0 { return Ok(None); }

        let q_dim       = (num_heads * head_dim) as usize;
        let kv_dim      = (kv_heads  * head_dim) as usize;
        let n_blk_kd    = (k_dim      / block_size) as u32;  // for q/k/v/gate/up
        let n_blk_qd    = (q_dim      / block_size) as u32;  // for o_proj
        let n_blk_inter = (inter_size / block_size) as u32;  // for down_proj

        // Q/KV/gate+up pipeline selection by n_blk_kd:
        //   ≤ 32 → TG-cached mb32 (qwen3-0.6b: hidden=1024/32=32 blocks)
        //   > 32 → no-cache LARGE4 for kd>64, mb64 for kd 33-64
        //           down_proj uses LARGE4_RES when n_blk_inter > 96 (mb96 limit).
        //   o_proj uses LARGE4 when n_blk_qd > 64 (14b: q_dim=5120→160 blocks)
        let use_large_kd    = n_blk_kd    as usize > 64;
        let use_mb64_kd     = n_blk_kd    as usize > 32 && n_blk_kd as usize <= 64;
        let use_large_inter = n_blk_inter as usize > 96;
        let use_large_qd    = n_blk_qd    as usize > 64;

        // If a primary AttnState exists for a different geometry, return None so the outer
        // loop falls back per-layer for this group. Prevents allocating large GPU KV caches
        // for secondary geometries.
        {
            let guard = self.attn.lock().unwrap();
            let has_different = !guard.is_empty() && !guard.iter().any(|s|
                s.num_heads == num_heads && s.kv_heads == kv_heads
                && s.head_dim == head_dim && s.max_seq == max_seq);
            if has_different { return Ok(None); }
        }

        // Build or grow the AttnState for this geometry, using compact slot allocation.
        let layer_indices: Vec<usize> = layers.iter().map(|l| l.layer_idx).collect();
        {
            let guard = self.attn.lock().unwrap();
            let needs_grow = !guard.iter().any(|s|
                s.num_heads == num_heads && s.kv_heads == kv_heads
                && s.head_dim == head_dim && s.max_seq == max_seq
                && layer_indices.iter().all(|li| s.layer_slot.contains_key(li)));
            drop(guard);
            if needs_grow {
                let g = self.attn_state(
                    num_heads, kv_heads, head_dim, max_seq,
                    &layer_indices, rope_dim, rope_theta,
                )?;
                drop(g);
            }
        }
        let attn_guard = self.attn.lock().unwrap();
        let st = attn_guard.iter().find(|s|
            s.num_heads == num_heads && s.kv_heads == kv_heads
            && s.head_dim == head_dim && s.max_seq == max_seq
        ).expect("attn_state ensured above");

        // RoPE tables are precomputed in AttnState — use byte offset for current position.
        let rope_byte_off = past_seq_len * rope_half * 4;
        let cos_buf = &st.rope_cos;
        let sin_buf = &st.rope_sin;

        let init_h = self.buf_ref(hidden)?;

        // Scratch buffers pooled across calls; safe because batch_raw waits before returning.
        let hd4 = k_dim * 4;
        let q4  = q_dim * 4;
        let kv4 = kv_dim * 4;
        let i4  = inter_size * 4;
        let hid_a = self.take_scratch(hd4)?;
        let hid_b = self.take_scratch(hd4)?;
        let hid2  = self.take_scratch(hd4)?;
        let q_raw = self.take_scratch(q4)?;
        let k_raw = self.take_scratch(kv4)?;
        let v_raw = self.take_scratch(kv4)?;
        // qr/kr only needed for the qk_norm (qwen3) path; skip for the fused path.
        let qr    = if has_qk_norm { Some(self.take_scratch(q4)?)  } else { None };
        let kr    = if has_qk_norm { Some(self.take_scratch(kv4)?) } else { None };
        let attn  = self.take_scratch(q4)?;
        let mid   = self.take_scratch(i4)?;

        // Pre-resolve HcBuffer pointers — raw for closure capture (safe: tensors
        // live for the duration of this function which outlives batch_raw).
        let get_ptr = |t: &Tensor| -> *const aruminium::Buffer {
            match &t.data {
                TensorData::Backend(b) => {
                    let h = b.as_any().downcast_ref::<HcBuffer>().unwrap();
                    &h.buffer as *const _
                }
                _ => std::ptr::null(),
            }
        };

        struct LPtrs {
            in_norm:       *const aruminium::Buffer,
            q_w:           *const aruminium::Buffer,
            k_w:           *const aruminium::Buffer,
            v_w:           *const aruminium::Buffer,
            q_b:           *const aruminium::Buffer, // null if no bias
            k_b:           *const aruminium::Buffer,
            v_b:           *const aruminium::Buffer,
            q_n:           *const aruminium::Buffer,
            k_n:           *const aruminium::Buffer,
            o_w:           *const aruminium::Buffer,
            post_n:        *const aruminium::Buffer, // pre-FFN norm (post_attention_layernorm)
            post_attn_n:   *const aruminium::Buffer, // Gemma: post-attn output norm; null if absent
            post_ffw_n:    *const aruminium::Buffer, // Gemma: post-FFN output norm; null if absent
            output_scale:  f32,                      // Gemma-4: per-layer scalar (1.0 = no-op)
            gate_w:        *const aruminium::Buffer,
            up_w:          *const aruminium::Buffer,
            down_w:        *const aruminium::Buffer,
            down_w_t:      *const aruminium::Buffer, // transposed; null if not needed
            kv_idx:        usize,
            q_n_rows:      u32,
            kv_n_rows:     u32,
            o_n_rows:      u32,
            g_n_rows:      u32,
            dn_n_rows:     u32,
            total_seq:     u32,
            scale:         f32,
        }
        unsafe impl Send for LPtrs {}

        let n_blk_inter_sz = inter_size / block_size as usize;
        let dn_n_rows_sz    = if n == 0 { 0 } else { layers[0].down_proj.shape[0] };
        let _ = dn_n_rows_sz;

        let mut lptrs: Vec<LPtrs> = Vec::with_capacity(n);
        for l in layers {
            let down_w_t = std::ptr::null();
            lptrs.push(LPtrs {
                in_norm:  get_ptr(l.input_norm),
                q_w:      get_ptr(l.q_proj),
                k_w:      get_ptr(l.k_proj),
                v_w:      get_ptr(l.v_proj),
                q_b:      l.q_bias.map_or(std::ptr::null(), |b| {
                    if let TensorData::Backend(h) = &b.data {
                        &h.as_any().downcast_ref::<HcBuffer>().unwrap().buffer
                    } else { std::ptr::null() }
                }),
                k_b:      l.k_bias.map_or(std::ptr::null(), |b| {
                    if let TensorData::Backend(h) = &b.data {
                        &h.as_any().downcast_ref::<HcBuffer>().unwrap().buffer
                    } else { std::ptr::null() }
                }),
                v_b:      l.v_bias.map_or(std::ptr::null(), |b| {
                    if let TensorData::Backend(h) = &b.data {
                        &h.as_any().downcast_ref::<HcBuffer>().unwrap().buffer
                    } else { std::ptr::null() }
                }),
                q_n:         l.q_norm.map_or(std::ptr::null(), get_ptr),
                k_n:         l.k_norm.map_or(std::ptr::null(), get_ptr),
                o_w:         get_ptr(l.o_proj),
                post_n:      get_ptr(l.post_norm),
                post_attn_n: l.post_attn_norm.map_or(std::ptr::null(), get_ptr),
                post_ffw_n:  l.post_ffw_norm.map_or(std::ptr::null(), get_ptr),
                output_scale: l.layer_output_scale,
                gate_w:      get_ptr(l.gate_proj),
                up_w:     get_ptr(l.up_proj),
                down_w:   get_ptr(l.down_proj),
                down_w_t,
                kv_idx:   *st.layer_slot.get(&l.layer_idx).expect("slot for layer"),
                q_n_rows:  l.num_heads * l.head_dim,
                kv_n_rows: l.kv_heads  * l.head_dim,
                o_n_rows:  l.o_proj.shape[0] as u32,
                g_n_rows:  l.gate_proj.shape[0] as u32,
                dn_n_rows: l.down_proj.shape[0] as u32,
                total_seq: (past_seq_len + 1) as u32,
                scale:     l.attn_scale,
            });
        }

        // Pipeline references (disjoint borrows from self — OK with batch_raw)
        let pipe_rmsnorm   = &self.pipe_rmsnorm.0;
        let _pipe_rope_ref  = &self.pipe_rope.0;
        let pipe_q_dual    = match kind {
            DType::Q4 => &self.pipe_q4_dual.0,
            DType::Q8 => &self.pipe_q8_dual.0,
            _ => unreachable!(),
        };
        let pipe_q_gus     = match kind {
            DType::Q4 => &self.pipe_q4_gus.0,
            DType::Q8 => &self.pipe_q8_gus.0,
            _ => unreachable!(),
        };
        // Sized-MAX_BLOCKS variants — used in fused dispatch for better occupancy.
        let pipe_q_opt     = match kind {
            DType::Q4 => &self.pipe_q4_mb32.0,
            DType::Q8 => &self.pipe_q8_mb32.0,
            _ => unreachable!(),
        };
        let pipe_q_dual_opt = match kind {
            DType::Q4 => &self.pipe_q4_dual_mb32.0,
            DType::Q8 => &self.pipe_q8_dual_mb32.0,
            _ => unreachable!(),
        };
        let pipe_q_gus_opt = match kind {
            DType::Q4 => &self.pipe_q4_gus_mb32.0,
            DType::Q8 => &self.pipe_q8_gus_mb32.0,
            _ => unreachable!(),
        };
        let pipe_q_gus_nrm = match kind {
            DType::Q4 => &self.pipe_q4_gus_nrm_mb32.0,
            DType::Q8 => &self.pipe_q8_gus_nrm_mb32.0,
            _ => unreachable!(),
        };
        let pipe_q_nrm = match kind {
            DType::Q4 => &self.pipe_q4_nrm_mb32.0,
            DType::Q8 => &self.pipe_q8_nrm_mb32.0,
            _ => unreachable!(),
        };
        let pipe_q_dual_nrm = match kind {
            DType::Q4 => &self.pipe_q4_dual_nrm_mb32.0,
            DType::Q8 => &self.pipe_q8_dual_nrm_mb32.0,
            _ => unreachable!(),
        };
        let pipe_q_res_o   = match kind {
            DType::Q4 => &self.pipe_q4_res_mb64.0,
            DType::Q8 => &self.pipe_q8_res_mb64.0,
            _ => unreachable!(),
        };
        let pipe_q_res_d   = match kind {
            DType::Q4 => &self.pipe_q4_res_mb96.0,
            DType::Q8 => &self.pipe_q8_res_mb96.0,
            _ => unreachable!(),
        };
        let pipe_kv_both                = &st.pipe_kv_append_both.0;
        let pipe_rope_kv_attn           = &st.pipe_rope_kv_attn.0;
        let pipe_qk_norm_rope_st        = &st.pipe_qk_norm_rope.0;  // head_dim-parameterized
        let pipe_qk_rope                = &self.pipe_qk_rope.0;
        let _pipe_qk_norm               = &self.pipe_qk_norm.0;
        let _pipe_q8_large_nrm          = &self.pipe_q8_large_nrm.0;
        let _pipe_q8_large_gus_nrm      = &self.pipe_q8_large_gus_nrm.0;
        let _pipe_q8_large_res          = &self.pipe_q8_large_res.0;
        let pipe_q8_large               = &self.pipe_q8_large.0;
        let pipe_q8_large4_nrm          = &self.pipe_q8_large4_nrm.0;
        let pipe_q8_large4_gus_nrm      = &self.pipe_q8_large4_gus_nrm.0;
        let pipe_q8_large4_gus_nrm_gelu = &self.pipe_q8_large4_gus_nrm_gelu.0;
        let pipe_q8_large2_res     = &self.pipe_q8_large2_res.0;
        let pipe_q8_large4_res     = &self.pipe_q8_large4_res.0;
        let pipe_q8_large8_res     = &self.pipe_q8_large8_res.0;
        let pipe_q8_mbx16_res      = &self.pipe_q8_mbx16_res.0;
        let pipe_q8_large4t_res    = &self.pipe_q8_large4t_res.0;
        let pipe_q8_large8t_res    = &self.pipe_q8_large8t_res.0;
        let pipe_q8_large16t_res   = &self.pipe_q8_large16t_res.0;
        let pipe_q8_nrm_mb64        = &self.pipe_q8_nrm_mb64.0;
        let pipe_q8_dual_nrm_mb64   = &self.pipe_q8_dual_nrm_mb64.0;
        let pipe_q8_gus_nrm_mb64    = &self.pipe_q8_gus_nrm_mb64.0;
        let pipe_q8_gus_nrm_mb48    = &self.pipe_q8_gus_nrm_mb48.0;
        let pipe_q8_gus_nrm_mb64_r4 = &self.pipe_q8_gus_nrm_mb64_r4.0;
        let pipe_q4_nrm_mb64        = &self.pipe_q4_nrm_mb64.0;
        let pipe_q4_dual_nrm_mb64   = &self.pipe_q4_dual_nrm_mb64.0;
        let pipe_q4_gus_nrm_mb64    = &self.pipe_q4_gus_nrm_mb64.0;
        let pipe_q4_gus_nrm_mb48    = &self.pipe_q4_gus_nrm_mb48.0;
        let pipe_q4_large_res       = &self.pipe_q4_large_res.0;
        let pipe_add                = &self.pipe_add.0;
        let pipe_scale              = &self.pipe_scale.0;
        let _pipe_qkv       = match kind {
            DType::Q4 => &self.pipe_q4_qkv.0,
            DType::Q8 => &self.pipe_q8_qkv.0,
            _ => unreachable!(),
        };
        let _pipe_q_res     = match kind {
            DType::Q4 => &self.pipe_q4_res.0,
            DType::Q8 => &self.pipe_q8_res.0,
            _ => unreachable!(),
        };

        // Warm up the Metal GPU on the first decode call: run 3 extra CB iterations
        // before the real one to ramp GPU from post-upload idle/low-clock state to
        // full decode speed. The encoding is deterministic (same hidden_in → same
        // hidden_out), so warmup iterations produce identical results and are safe
        // to discard. KV cache writes at the same past_seq_len are idempotent.
        let n_warmup = if !self.warmed_up.swap(true, std::sync::atomic::Ordering::Relaxed) {
            3
        } else {
            0
        };

        unsafe {
            aruminium::autorelease_pool(|| {
                for _wi in 0..=n_warmup {
                self.device.dispatch.batch_raw(|enc| {
                    for (li, lp) in lptrs.iter().enumerate() {
                        // Ensure previous layer's hidden_out write is visible.
                        if li > 0 { enc.memory_barrier_buffers(); }
                        let hidden_in: &aruminium::Buffer = if li == 0 {
                            init_h.as_buffer()
                        } else if li % 2 == 1 {
                            &hid_b
                        } else {
                            &hid_a
                        };
                        let hidden_out: &aruminium::Buffer =
                            if li % 2 == 0 { &hid_b } else { &hid_a };

                        macro_rules! push_bytes {
                            ($enc:expr, $val:expr, $idx:expr) => {{
                                let bytes = std::slice::from_raw_parts(
                                    &$val as *const _ as *const u8,
                                    std::mem::size_of_val(&$val),
                                );
                                $enc.push(bytes, $idx);
                            }};
                        }

                        // 1+2. Q matmul with inline input_norm
                        {
                            let n_rows = lp.q_n_rows;
                            let tpg = simds * 32;
                            #[repr(C)] #[derive(Clone,Copy)]
                            struct D { batch: u32, n_rows: u32, n_blocks: u32, eps: f32 }
                            let d = D { batch: 1, n_rows, n_blocks: n_blk_kd, eps };
                            let (pipe, rows_per_tg) = if use_large_kd {
                                (pipe_q8_large4_nrm, simds * 4)
                            } else if use_mb64_kd {
                                let p = match kind {
                                    DType::Q8 => pipe_q8_nrm_mb64,
                                    DType::Q4 => pipe_q4_nrm_mb64,
                                    _ => unreachable!(),
                                };
                                (p, simds * n_dst)
                            } else {
                                (pipe_q_nrm, simds * n_dst)
                            };
                            let groups = (n_rows + rows_per_tg - 1) / rows_per_tg;
                            enc.bind(pipe);
                            enc.bind_buffer(hidden_in, 0, 0);
                            enc.bind_buffer(&*lp.in_norm, 0, 1);
                            enc.bind_buffer(&*lp.q_w, 0, 2);
                            enc.bind_buffer(&q_raw, 0, 3);
                            push_bytes!(enc, d, 4);
                            enc.launch_groups((groups as usize, 1, 1), (tpg as usize, 1, 1));
                        }
                        // 3. K+V with inline input_norm
                        if use_large_kd {
                            // No dual variant for LARGE4: dispatch K and V separately.
                            let rows_per_tg = simds * 4;
                            let tpg = simds * 32;
                            #[repr(C)] #[derive(Clone,Copy)]
                            struct D { batch: u32, n_rows: u32, n_blocks: u32, eps: f32 }
                            for (w_ptr, out_buf) in [
                                (lp.k_w, &k_raw as *const _),
                                (lp.v_w, &v_raw as *const _),
                            ] {
                                let n_rows = lp.kv_n_rows;
                                let groups = (n_rows + rows_per_tg - 1) / rows_per_tg;
                                let d = D { batch: 1, n_rows, n_blocks: n_blk_kd, eps };
                                enc.bind(pipe_q8_large4_nrm);
                                enc.bind_buffer(hidden_in, 0, 0);
                                enc.bind_buffer(&*lp.in_norm, 0, 1);
                                enc.bind_buffer(&*w_ptr, 0, 2);
                                enc.bind_buffer(&*out_buf, 0, 3);
                                push_bytes!(enc, d, 4);
                                enc.launch_groups((groups as usize, 1, 1), (tpg as usize, 1, 1));
                            }
                        } else {
                            // TG-cached dual dispatch (K+V in one kernel launch).
                            let n_rows = lp.kv_n_rows;
                            let rows_per_tg = simds * n_dst;
                            let groups = (n_rows + rows_per_tg - 1) / rows_per_tg;
                            let tpg = simds * 32;
                            #[repr(C)] #[derive(Clone,Copy)]
                            struct D { batch: u32, n_rows: u32, n_blocks: u32, eps: f32 }
                            let d = D { batch: 1, n_rows, n_blocks: n_blk_kd, eps };
                            let pipe = if use_mb64_kd {
                                match kind {
                                    DType::Q8 => pipe_q8_dual_nrm_mb64,
                                    DType::Q4 => pipe_q4_dual_nrm_mb64,
                                    _ => unreachable!(),
                                }
                            } else { pipe_q_dual_nrm };
                            enc.bind(pipe);
                            enc.bind_buffer(hidden_in, 0, 0);
                            enc.bind_buffer(&*lp.in_norm, 0, 1);
                            enc.bind_buffer(&*lp.k_w, 0, 2);
                            enc.bind_buffer(&*lp.v_w, 0, 3);
                            enc.bind_buffer(&k_raw, 0, 4);
                            enc.bind_buffer(&v_raw, 0, 5);
                            push_bytes!(enc, d, 6);
                            enc.launch_groups((groups as usize, 1, 1), (tpg as usize, 1, 1));
                        }
                        // Fused matmul + residual add: y[row] = matmul(x, w)[row] + residual[row].
                        // rows_per_simd: 1 for TG-cached/LARGE kernels, 4 for LARGE4 kernels.
                        let dispatch_qmm_res = |enc: &aruminium::Batch,
                                                pipe: &aruminium::Pipeline,
                                                x: &aruminium::Buffer,
                                                w: *const aruminium::Buffer,
                                                residual: &aruminium::Buffer,
                                                out: &aruminium::Buffer,
                                                n_rows: u32,
                                                n_blk: u32,
                                                rows_per_simd: u32| {
                            let rows_per_tg = simds * rows_per_simd;
                            let groups = (n_rows + rows_per_tg - 1) / rows_per_tg;
                            let tpg    = simds * 32;
                            #[repr(C)] #[derive(Clone,Copy)]
                            struct D { batch: u32, n_rows: u32, n_blocks: u32, pad: u32 }
                            let d = D { batch: 1, n_rows, n_blocks: n_blk, pad: 0 };
                            enc.bind(pipe);
                            enc.bind_buffer(x, 0, 0);
                            enc.bind_buffer(&*w, 0, 1);
                            enc.bind_buffer(residual, 0, 2);
                            enc.bind_buffer(out, 0, 3);
                            let bytes = std::slice::from_raw_parts(
                                &d as *const D as *const u8, std::mem::size_of::<D>(),
                            );
                            enc.push(bytes, 4);
                            enc.launch_groups((groups as usize, 1, 1), (tpg as usize, 1, 1));
                        };

                        // Barrier: q_raw/k_raw/v_raw must be visible before bias-add and RoPE.
                        enc.memory_barrier_buffers();

                        // Optional QKV bias addition (Qwen2-style attn_bias).
                        // In-place: q_raw[i] += q_bias[i], same for k and v.
                        if has_qkv_bias {
                            macro_rules! add_bias_inplace {
                                ($buf:expr, $bias_ptr:expr, $n:expr) => {
                                    if !$bias_ptr.is_null() {
                                        #[repr(C)] #[derive(Clone, Copy)]
                                        struct P { n: u32, a_len: u32, b_len: u32, pad: u32 }
                                        let p = P { n: $n, a_len: $n, b_len: $n, pad: 0 };
                                        enc.bind(pipe_add);
                                        enc.bind_buffer($buf, 0, 0);
                                        enc.bind_buffer(&*$bias_ptr, 0, 1);
                                        enc.bind_buffer($buf, 0, 2);
                                        push_bytes!(enc, p, 3);
                                        let groups = (($n + 255) / 256) as usize;
                                        enc.launch_groups((groups, 1, 1), (256, 1, 1));
                                    }
                                };
                            }
                            add_bias_inplace!(&q_raw, lp.q_b, lp.q_n_rows);
                            add_bias_inplace!(&k_raw, lp.k_b, lp.kv_n_rows);
                            add_bias_inplace!(&v_raw, lp.v_b, lp.kv_n_rows);
                            // Barrier: bias-modified qkv must be visible before RoPE/attention.
                            enc.memory_barrier_buffers();
                        }

                        if has_qk_norm {
                            // QK-norm + RoPE (Qwen3/Gemma-4 style) — keeps separate qr/kr buffers.
                            // pipe_qk_norm_rope_st is parameterized for this geometry's head_dim.
                            {
                                #[repr(C)] #[derive(Clone,Copy)]
                                struct P { q_heads: u32, kv_heads: u32, rope_half: u32, eps: f32 }
                                let p = P { q_heads: num_heads, kv_heads, rope_half: rope_half as u32, eps };
                                enc.bind(pipe_qk_norm_rope_st);
                                enc.bind_buffer(&q_raw, 0, 0);
                                enc.bind_buffer(&k_raw, 0, 1);
                                enc.bind_buffer(&*lp.q_n, 0, 2);
                                enc.bind_buffer(&*lp.k_n, 0, 3);
                                enc.bind_buffer(cos_buf, rope_byte_off, 4);
                                enc.bind_buffer(sin_buf, rope_byte_off, 5);
                                enc.bind_buffer(qr.as_ref().unwrap(), 0, 6);
                                enc.bind_buffer(kr.as_ref().unwrap(), 0, 7);
                                push_bytes!(enc, p, 8);
                                // Launch one group per head; head_dim threads per group.
                                enc.launch_groups(((num_heads + kv_heads) as usize, 1, 1), (head_dim as usize, 1, 1));
                            }
                            // Barrier: qr/kr must be visible before KV-append reads kr.
                            enc.memory_barrier_buffers();
                            // KV-append K+V in one dispatch
                            {
                                #[repr(C)] #[derive(Clone,Copy)]
                                struct P { position: u32, p0: u32, p1: u32, p2: u32 }
                                let p = P { position: past_seq_len as u32, p0: 0, p1: 0, p2: 0 };
                                enc.bind(pipe_kv_both);
                                enc.bind_buffer(kr.as_ref().unwrap(), 0, 0);
                                enc.bind_buffer(&st.k_caches[lp.kv_idx], 0, 1);
                                enc.bind_buffer(&v_raw, 0, 2);
                                enc.bind_buffer(&st.v_caches[lp.kv_idx], 0, 3);
                                push_bytes!(enc, p, 4);
                                enc.launch_groups((1, kv_heads as usize, 1), (head_dim as usize, 1, 1));
                            }
                            // Barrier: k_cache/v_cache must be visible before SDPA.
                            enc.memory_barrier_buffers();
                            // SDPA
                            {
                                #[repr(C)] #[derive(Clone,Copy)]
                                struct P { total_seq: u32, window: u32, scale: f32, pad: u32 }
                                let p = P { total_seq: lp.total_seq, window: 0, scale: lp.scale, pad: 0 };
                                enc.bind(&st.pipe_attn.0);
                                enc.bind_buffer(qr.as_ref().unwrap(), 0, 0);
                                enc.bind_buffer(&st.k_caches[lp.kv_idx], 0, 1);
                                enc.bind_buffer(&st.v_caches[lp.kv_idx], 0, 2);
                                enc.bind_buffer(&attn, 0, 3);
                                push_bytes!(enc, p, 4);
                                enc.launch_groups((num_heads as usize, 1, 1), (32, 1, 1));
                            }
                        } else {
                            // Fused: RoPE(q,k) + KV-append + SDPA in one kernel (qwen2 style).
                            // Saves 2 barriers + 2 dispatches vs the split path.
                            #[repr(C)] #[derive(Clone,Copy)]
                            struct P { total_seq: u32, position: u32, window: u32, scale: f32 }
                            let p = P {
                                total_seq: lp.total_seq,
                                position:  past_seq_len as u32,
                                window:    0,
                                scale:     lp.scale,
                            };
                            enc.bind(pipe_rope_kv_attn);
                            enc.bind_buffer(&q_raw, 0, 0);
                            enc.bind_buffer(&k_raw, 0, 1);
                            enc.bind_buffer(&v_raw, 0, 2);
                            enc.bind_buffer(cos_buf, rope_byte_off, 3);
                            enc.bind_buffer(sin_buf, rope_byte_off, 4);
                            enc.bind_buffer(&st.k_caches[lp.kv_idx], 0, 5);
                            enc.bind_buffer(&st.v_caches[lp.kv_idx], 0, 6);
                            enc.bind_buffer(&attn, 0, 7);
                            push_bytes!(enc, p, 8);
                            enc.launch_groups((num_heads as usize, 1, 1), (32, 1, 1));
                        }

                        // Barrier: attn must be visible before o_proj reads it.
                        enc.memory_barrier_buffers();

                        // o_proj + optional post_attn_norm + residual.
                        // Without post_attn_norm: fused matmul+residual → hid2.
                        // With post_attn_norm: plain matmul → rmsnorm → add_residual.
                        if lp.post_attn_n.is_null() {
                            if use_large_qd {
                                let p = match kind {
                                    DType::Q8 => pipe_q8_large4_res,
                                    DType::Q4 => pipe_q4_large_res,
                                    _ => unreachable!(),
                                };
                                let rps = if kind == DType::Q8 { 4u32 } else { 1u32 };
                                dispatch_qmm_res(enc, p, &attn, lp.o_w, hidden_in, &hid2, lp.o_n_rows, n_blk_qd, rps);
                            } else {
                                dispatch_qmm_res(enc, pipe_q_res_o, &attn, lp.o_w, hidden_in, &hid2, lp.o_n_rows, n_blk_qd, n_dst);
                            }
                        } else {
                            // Step 1: plain matmul of o_proj → q_raw (q_raw is free after SDPA)
                            {
                                let n_rows = lp.o_n_rows;
                                let rows_per_tg = simds;  // MSL_LARGE: 1 row per SIMD, 16 SIMDs/TG
                                let groups = (n_rows + rows_per_tg - 1) / rows_per_tg;
                                let tpg = simds * 32;
                                #[repr(C)] #[derive(Clone,Copy)]
                                struct D { batch: u32, n_rows: u32, n_blocks: u32, pad: u32 }
                                let d = D { batch: 1, n_rows, n_blocks: n_blk_qd, pad: 0 };
                                enc.bind(pipe_q8_large);
                                enc.bind_buffer(&attn, 0, 0);
                                enc.bind_buffer(&*lp.o_w, 0, 1);
                                enc.bind_buffer(&q_raw, 0, 2);
                                push_bytes!(enc, d, 3);
                                enc.launch_groups((groups as usize, 1, 1), (tpg as usize, 1, 1));
                            }
                            enc.memory_barrier_buffers();
                            // Step 2: rmsnorm(q_raw, post_attn_norm) → hid2
                            {
                                #[repr(C)] #[derive(Clone,Copy)]
                                struct P { batch: u32, d: u32, eps: f32, pad: u32 }
                                let p = P { batch: 1, d: k_dim as u32, eps, pad: 0 };
                                enc.bind(pipe_rmsnorm);
                                enc.bind_buffer(&q_raw, 0, 0);
                                enc.bind_buffer(&*lp.post_attn_n, 0, 1);
                                enc.bind_buffer(&hid2, 0, 2);
                                push_bytes!(enc, p, 3);
                                enc.launch_groups((1, 1, 1), (256, 1, 1));
                            }
                            enc.memory_barrier_buffers();
                            // Step 3: hid2 = hid2 + hidden_in (in-place residual add)
                            {
                                let n = k_dim as u32;
                                #[repr(C)] #[derive(Clone,Copy)]
                                struct P { n: u32, a_len: u32, b_len: u32, pad: u32 }
                                let p = P { n, a_len: n, b_len: n, pad: 0 };
                                enc.bind(pipe_add);
                                enc.bind_buffer(&hid2, 0, 0);
                                enc.bind_buffer(hidden_in, 0, 1);
                                enc.bind_buffer(&hid2, 0, 2);
                                push_bytes!(enc, p, 3);
                                enc.launch_groups((((n + 255) / 256) as usize, 1, 1), (256, 1, 1));
                            }
                        }

                        // Barrier: hid2 must be visible before gate+up reads it.
                        enc.memory_barrier_buffers();
                        // gate+up+act fused with post_norm. SiLU or GeluTanh based on use_gelu_tanh.
                        {
                            let n_rows = lp.g_n_rows;
                            let tpg = simds * 32;
                            #[repr(C)] #[derive(Clone,Copy)]
                            struct D { batch: u32, n_rows: u32, n_blocks: u32, eps: f32 }
                            let d = D { batch: 1, n_rows, n_blocks: n_blk_kd, eps };
                            let (pipe, rows_per_tg) = if use_large_kd {
                                let p = if use_gelu_tanh { pipe_q8_large4_gus_nrm_gelu } else { pipe_q8_large4_gus_nrm };
                                (p, simds * 4)
                            } else if use_mb64_kd {
                                let p = if n_blk_kd as usize <= 48 {
                                    match kind {
                                        DType::Q8 => pipe_q8_gus_nrm_mb48,
                                        DType::Q4 => pipe_q4_gus_nrm_mb48,
                                        _ => unreachable!(),
                                    }
                                } else {
                                    match kind {
                                        DType::Q8 => pipe_q8_gus_nrm_mb64,
                                        DType::Q4 => pipe_q4_gus_nrm_mb64,
                                        _ => unreachable!(),
                                    }
                                };
                                (p, simds * n_dst)
                            } else {
                                (pipe_q_gus_nrm, simds * n_dst)
                            };
                            let groups = (n_rows + rows_per_tg - 1) / rows_per_tg;
                            enc.bind(pipe);
                            enc.bind_buffer(&hid2, 0, 0);
                            enc.bind_buffer(&*lp.post_n, 0, 1);
                            enc.bind_buffer(&*lp.gate_w, 0, 2);
                            enc.bind_buffer(&*lp.up_w,   0, 3);
                            enc.bind_buffer(&mid, 0, 4);
                            push_bytes!(enc, d, 5);
                            enc.launch_groups((groups as usize, 1, 1), (tpg as usize, 1, 1));
                        }

                        // Barrier: mid must be visible before down_proj reads it.
                        enc.memory_barrier_buffers();

                        // down_proj + optional post_ffw_norm + residual.
                        if lp.post_ffw_n.is_null() {
                            if use_large_inter {
                                let p = match kind {
                                    DType::Q8 => pipe_q8_large4_res,
                                    DType::Q4 => pipe_q4_large_res,
                                    _ => unreachable!(),
                                };
                                let rps = if kind == DType::Q8 { 4u32 } else { 1u32 };
                                dispatch_qmm_res(enc, p, &mid, lp.down_w, &hid2, hidden_out, lp.dn_n_rows, n_blk_inter, rps);
                            } else {
                                dispatch_qmm_res(enc, pipe_q_res_d, &mid, lp.down_w, &hid2, hidden_out, lp.dn_n_rows, n_blk_inter, n_dst);
                            }
                        } else {
                            // Step 1: plain matmul of down_proj → q_raw
                            {
                                let n_rows = lp.dn_n_rows;
                                let rows_per_tg = simds;
                                let groups = (n_rows + rows_per_tg - 1) / rows_per_tg;
                                let tpg = simds * 32;
                                #[repr(C)] #[derive(Clone,Copy)]
                                struct D { batch: u32, n_rows: u32, n_blocks: u32, pad: u32 }
                                let d = D { batch: 1, n_rows, n_blocks: n_blk_inter, pad: 0 };
                                enc.bind(pipe_q8_large);
                                enc.bind_buffer(&mid, 0, 0);
                                enc.bind_buffer(&*lp.down_w, 0, 1);
                                enc.bind_buffer(&q_raw, 0, 2);
                                push_bytes!(enc, d, 3);
                                enc.launch_groups((groups as usize, 1, 1), (tpg as usize, 1, 1));
                            }
                            enc.memory_barrier_buffers();
                            // Step 2: rmsnorm(q_raw, post_ffw_norm) → hidden_out
                            {
                                #[repr(C)] #[derive(Clone,Copy)]
                                struct P { batch: u32, d: u32, eps: f32, pad: u32 }
                                let p = P { batch: 1, d: k_dim as u32, eps, pad: 0 };
                                enc.bind(pipe_rmsnorm);
                                enc.bind_buffer(&q_raw, 0, 0);
                                enc.bind_buffer(&*lp.post_ffw_n, 0, 1);
                                enc.bind_buffer(hidden_out, 0, 2);
                                push_bytes!(enc, p, 3);
                                enc.launch_groups((1, 1, 1), (256, 1, 1));
                            }
                            enc.memory_barrier_buffers();
                            // Step 3: hidden_out = hidden_out + hid2 (in-place residual add)
                            {
                                let n = k_dim as u32;
                                #[repr(C)] #[derive(Clone,Copy)]
                                struct P { n: u32, a_len: u32, b_len: u32, pad: u32 }
                                let p = P { n, a_len: n, b_len: n, pad: 0 };
                                enc.bind(pipe_add);
                                enc.bind_buffer(hidden_out, 0, 0);
                                enc.bind_buffer(&hid2, 0, 1);
                                enc.bind_buffer(hidden_out, 0, 2);
                                push_bytes!(enc, p, 3);
                                enc.launch_groups((((n + 255) / 256) as usize, 1, 1), (256, 1, 1));
                            }
                        }
                        // Gemma-4: layer_output_scale in-place multiply (prevents activation explosion).
                        if lp.output_scale != 1.0f32 {
                            enc.memory_barrier_buffers();
                            #[repr(C)] #[derive(Clone,Copy)]
                            struct Sc { value: f32, n: u32, pad0: u32, pad1: u32 }
                            let sc = Sc { value: lp.output_scale, n: k_dim as u32, pad0: 0, pad1: 0 };
                            enc.bind(pipe_scale);
                            enc.bind_buffer(hidden_out, 0, 0);
                            push_bytes!(enc, sc, 1);
                            let groups = ((k_dim as u32 + 255) / 256) as usize;
                            enc.launch_groups((groups, 1, 1), (256, 1, 1));
                        }
                    } // end layer loop
                }); // end batch_raw
                } // end warmup loop
            });
        }

        // FUSED_DEBUG: after batch completes, inspect intermediate buffers for NaN
        if std::env::var("FUSED_DEBUG").is_ok() && n <= 2 {
            let check = |buf: &aruminium::Buffer, label: &str, count: usize| {
                let vals = self.read_f32(buf, count);
                let nans = vals.iter().filter(|v| v.is_nan() || v.is_infinite()).count();
                eprintln!("  [{label}] nan/inf={nans}/{count}  first4={:.4?}", &vals[..4.min(count)]);
            };
            eprintln!("[FUSED_DEBUG inner] n={n} post-batch intermediates:");
            check(&attn,  "attn (sdpa out)", q_dim);
            check(&q_raw, "q_raw (o_proj or down_proj out)", q_dim);
            check(&hid2,  "hid2 (post-attn h1)", k_dim);
            check(&mid,   "mid (gate*up)", inter_size);
            let last_out = if (n.saturating_sub(1)) % 2 == 0 { &hid_b } else { &hid_a };
            check(last_out, "hidden_out (final layer out)", k_dim);
            check(&hid_a, "hid_a", k_dim);
            check(&hid_b, "hid_b", k_dim);
            if let Some(ref qr_buf) = qr {
                check(qr_buf, "qr (post-qknorm-rope q)", q_dim);
            }
        }

        // Layer n-1 wrote to hid_b if (n-1)%2==0, hid_a if (n-1)%2==1
        let last = n.saturating_sub(1);
        let (final_buf, other_hid) = if last % 2 == 0 { (hid_b, hid_a) } else { (hid_a, hid_b) };

        // Return intermediates to the scratch pool for the next token's call.
        self.release_scratch(other_hid);
        self.release_scratch(hid2);
        self.release_scratch(q_raw);
        self.release_scratch(k_raw);
        self.release_scratch(v_raw);
        if let Some(b) = qr  { self.release_scratch(b); }
        if let Some(b) = kr  { self.release_scratch(b); }
        self.release_scratch(attn);
        self.release_scratch(mid);

        if hc_timing {
            let elapsed_ms = t_fused_entry.elapsed().as_secs_f64() * 1000.0;
            eprintln!("[HC_TIMING] fused_layers n={} li={} kd={} qd={} inter={} kv_heads={} head_dim={} max_seq={} past={} gelu={} qknorm={} → {:.1}ms",
                n, layers[0].layer_idx, k_dim, q_dim, inter_size,
                kv_heads, head_dim, max_seq, past_seq_len,
                use_gelu_tanh, has_qk_norm, elapsed_ms);
        }

        Ok(Some(self.wrap_output(final_buf, hidden.shape.clone(), DType::F32)))
    }

    /// Batched RmsNorm — multiple independent (x, gamma) pairs in one command
    /// buffer with a single wait.
    fn rms_norm_multi(
        &self,
        pairs: &[(&Tensor, &Tensor)],
        eps: f32,
    ) -> Result<Vec<Tensor>, BackendError> {
        if pairs.is_empty() { return Ok(Vec::new()); }
        // All inputs must be f32 (host or backend). If any quantized, fall back.
        let supported = pairs.iter().all(|(x, g)| x.dtype == DType::F32 && g.dtype == DType::F32);
        if !supported {
            return Backend::rms_norm_multi(self, pairs, eps);
        }

        // Resolve buf refs and allocate outputs.
        struct Item<'a> {
            x: BufRef<'a>,
            g: BufRef<'a>,
            out: aruminium::Buffer,
            shape: Vec<usize>,
            batch: u32,
            d: u32,
        }
        let mut items: Vec<Item> = Vec::with_capacity(pairs.len());
        for (x, g) in pairs {
            let d = g.shape[0] as u32;
            let batch = x.shape[..x.shape.len() - 1].iter().product::<usize>() as u32;
            let n_bytes = (batch as usize * d as usize * 4).max(4);
            let out = self.device.alloc(n_bytes)?;
            items.push(Item {
                x: self.buf_ref(x)?,
                g: self.buf_ref(g)?,
                out,
                shape: x.shape.clone(),
                batch,
                d,
            });
        }

        unsafe {
            aruminium::autorelease_pool(|| {
                self.device.dispatch.batch_raw(|enc| {
                    for it in &items {
                        #[repr(C)]
                        #[derive(Clone, Copy)]
                        struct P { batch: u32, d: u32, eps: f32, pad: u32 }
                        let p = P { batch: it.batch, d: it.d, eps, pad: 0 };
                        enc.bind(&self.pipe_rmsnorm.0);
                        enc.bind_buffer(it.x.as_buffer(), 0, 0);
                        enc.bind_buffer(it.g.as_buffer(), 0, 1);
                        enc.bind_buffer(&it.out, 0, 2);
                        let bytes = std::slice::from_raw_parts(
                            &p as *const P as *const u8,
                            std::mem::size_of::<P>(),
                        );
                        enc.push(bytes, 3);
                        enc.launch_groups((it.batch as usize, 1, 1), (256, 1, 1));
                    }
                });
            });
        }

        let mut results = Vec::with_capacity(items.len());
        for it in items {
            results.push(self.wrap_output(it.out, it.shape, DType::F32));
        }
        Ok(results)
    }

    /// Fused (RmsNorm + N quant matmul) — encodes the whole chain into ONE
    /// command buffer with ONE wait. Saves the per-op submit overhead that
    /// dominates small-matmul timings.
    fn fused_norm_quant_matmul_multi(
        &self,
        x: &Tensor,
        gamma: &Tensor,
        eps: f32,
        ws: &[&Tensor],
    ) -> Result<Vec<Tensor>, BackendError> {
        if ws.is_empty() { return Ok(Vec::new()); }
        let k = ws[0].shape[1];
        let kind0 = ws[0].dtype;
        let supported = ws.iter().all(|w| {
            w.dtype == kind0
                && w.shape[1] == k
                && (matches!(w.dtype, DType::Q8) && w.shape[1] % kernels::q8_matmul::BLOCK_SIZE == 0
                    || matches!(w.dtype, DType::Q4) && w.shape[1] % kernels::q4_matmul::BLOCK_SIZE == 0)
        });
        let weights_on_gpu = ws.iter().all(|w| matches!(w.data, TensorData::Backend(_)));
        if !supported || !weights_on_gpu || x.dtype != DType::F32 || gamma.dtype != DType::F32 {
            return Backend::fused_norm_quant_matmul_multi(self, x, gamma, eps, ws);
        }

        let batch = x.shape[..x.shape.len() - 1].iter().product::<usize>() as u32;
        let d = gamma.shape[0] as u32;
        if k as u32 != d {
            return Err(BackendError::ShapeMismatch {
                op: "fused_norm_quant_matmul_multi",
                expected: vec![d as usize],
                got: vec![k],
            });
        }

        let block_size = match kind0 {
            DType::Q8 => kernels::q8_matmul::BLOCK_SIZE,
            DType::Q4 => kernels::q4_matmul::BLOCK_SIZE,
            _ => unreachable!(),
        };
        let n_blocks = (k / block_size) as u32;
        // Select kernel by n_blocks. mb192 covers 128<n_blk≤192 (Gemma-4 n_blk=168).
        // Without this, pipe_q8.0 (MAX_BLOCKS=128) would overflow x_shared for n_blk>128.
        let pipe = match kind0 {
            DType::Q8 => {
                if n_blocks as usize > 192 { &self.pipe_q8_large.0 }
                else if n_blocks as usize > kernels::q8_matmul::TG_MAX_BLOCKS { &self.pipe_q8_mb192.0 }
                else { &self.pipe_q8.0 }
            }
            DType::Q4 => &self.pipe_q4.0,
            _ => unreachable!(),
        };
        let simds_per_group = match kind0 {
            DType::Q8 => kernels::q8_matmul::SIMDS_PER_GROUP,
            DType::Q4 => kernels::q4_matmul::SIMDS_PER_GROUP,
            _ => unreachable!(),
        };
        let n_dst_q0 = match kind0 {
            DType::Q8 => kernels::q8_matmul::N_DST,
            DType::Q4 => kernels::q4_matmul::N_DST,
            _ => unreachable!(),
        };
        let x_buf = self.buf_ref(x)?;
        let g_buf = self.buf_ref(gamma)?;

        // Allocate normed (intermediate, pooled) and one output per matmul.
        let normed_buf = self.take_scratch((batch as usize * d as usize * 4).max(4))?;
        let mut weight_refs: Vec<&aruminium::Buffer> = Vec::with_capacity(ws.len());
        let mut ns: Vec<usize> = Vec::with_capacity(ws.len());
        for w in ws {
            let h = match &w.data {
                TensorData::Backend(b) => b.as_any().downcast_ref::<HcBuffer>().ok_or_else(|| {
                    BackendError::Internal("fused: wrong backend tensor".into())
                })?,
                _ => unreachable!(),
            };
            weight_refs.push(&h.buffer);
            ns.push(w.shape[0]);
        }
        let mut out_bufs: Vec<aruminium::Buffer> = Vec::with_capacity(ws.len());
        for &n in &ns {
            out_bufs.push(self.device.alloc((batch as usize * n * 4).max(4))?);
        }

        unsafe {
            aruminium::autorelease_pool(|| {
                self.device.dispatch.batch_raw(|enc| {
                    // 1) RmsNorm: x → normed_buf
                    {
                        #[repr(C)]
                        #[derive(Clone, Copy)]
                        struct NormParams { batch: u32, d: u32, eps: f32, pad: u32 }
                        let p = NormParams { batch, d, eps, pad: 0 };
                        enc.bind(&self.pipe_rmsnorm.0);
                        enc.bind_buffer(x_buf.as_buffer(), 0, 0);
                        enc.bind_buffer(g_buf.as_buffer(), 0, 1);
                        enc.bind_buffer(&normed_buf, 0, 2);
                        let bytes = std::slice::from_raw_parts(
                            &p as *const NormParams as *const u8,
                            std::mem::size_of::<NormParams>(),
                        );
                        enc.push(bytes, 3);
                        enc.launch_groups((batch as usize, 1, 1), (256, 1, 1));
                    }
                    enc.memory_barrier_buffers();
                    // 2) N quant matmuls reading from normed_buf
                    for (i, w_buf) in weight_refs.iter().enumerate() {
                        let n = ns[i] as u32;
                        let total_rows = batch * n;
                        let rows_per_tg = simds_per_group * n_dst_q0;
                        let groups_x = (total_rows + rows_per_tg - 1) / rows_per_tg;
                        let threads_per_group = simds_per_group * 32;

                        #[repr(C)]
                        #[derive(Clone, Copy)]
                        struct Dims { batch: u32, n_rows: u32, n_blocks: u32, pad: u32 }
                        let dims = Dims { batch, n_rows: n, n_blocks, pad: 0 };

                        enc.bind(pipe);
                        enc.bind_buffer(&normed_buf, 0, 0);
                        enc.bind_buffer(w_buf, 0, 1);
                        enc.bind_buffer(&out_bufs[i], 0, 2);
                        let bytes = std::slice::from_raw_parts(
                            &dims as *const Dims as *const u8,
                            std::mem::size_of::<Dims>(),
                        );
                        enc.push(bytes, 3);
                        enc.launch_groups(
                            (groups_x as usize, 1, 1),
                            (threads_per_group as usize, 1, 1),
                        );
                    }
                });
            });
        }

        // Recycle the normed scratch.
        self.release_scratch(normed_buf);

        if std::env::var("RUN_DEBUG_PERLAYER").is_ok() {
            for (i, buf) in out_bufs.iter().enumerate() {
                buf.read(|bytes| {
                    let floats: &[f32] = bytemuck::cast_slice(bytes);
                    let abs_max = floats.iter().map(|v| v.abs()).fold(0f32, f32::max);
                    let rms = (floats.iter().map(|v|v*v).sum::<f32>() / floats.len() as f32).sqrt();
                    eprintln!("  perlayer qkv[{i}] abs_max={abs_max:.4} rms={rms:.4} n={}", floats.len());
                });
            }
        }

        let mut results = Vec::with_capacity(ws.len());
        for (out_buf, n) in out_bufs.into_iter().zip(ns.into_iter()) {
            let mut out_shape = x.shape.clone();
            *out_shape.last_mut().unwrap() = n;
            results.push(self.wrap_output(out_buf, out_shape, DType::F32));
        }
        Ok(results)
    }
}
