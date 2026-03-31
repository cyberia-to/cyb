// Elementwise ops — all fp16

#include <metal_stdlib>
using namespace metal;

struct ElemParams { uint n; };

// Residual add: out = a + b
kernel void add_f16(device const half *a [[buffer(0)]],
                    device const half *b [[buffer(1)]],
                    device half *out [[buffer(2)]],
                    constant ElemParams &p [[buffer(3)]],
                    uint gid [[thread_position_in_grid]]) {
    if (gid >= p.n) return;
    out[gid] = a[gid] + b[gid];
}

// Fused SiLU × gate (SwiGLU): out = silu(gate) * up
kernel void silu_mul_f16(device const half *gate_buf [[buffer(0)]],
                         device const half *up_buf [[buffer(1)]],
                         device half *out [[buffer(2)]],
                         constant ElemParams &p [[buffer(3)]],
                         uint gid [[thread_position_in_grid]]) {
    if (gid >= p.n) return;
    float g = float(gate_buf[gid]);
    float sig = 1.0f / (1.0f + exp(-g));
    out[gid] = half(g * sig * float(up_buf[gid]));
}

// Fused ReLU² × gate (BitNet): out = relu2(gate) * up = max(0,gate)² * up
kernel void relu2_mul_f16(device const half *gate_buf [[buffer(0)]],
                          device const half *up_buf [[buffer(1)]],
                          device half *out [[buffer(2)]],
                          constant ElemParams &p [[buffer(3)]],
                          uint gid [[thread_position_in_grid]]) {
    if (gid >= p.n) return;
    float g = max(0.0f, float(gate_buf[gid]));
    out[gid] = half(g * g * float(up_buf[gid]));
}
