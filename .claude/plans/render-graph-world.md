# Graph World — Rendering Implementation Plan

Protocol spec: `cybergraph/reference/render.md` (R-1.0)
Implementation spec: `cyb/render/specs/render.md` (R-1.0-cyb)

---

## Architectural decisions

### Barnes-Hut vs BVH

The deeper question is not "which algorithm for rendering cull" but "does this
manifold need N-body approximation at all?"

Newtonian gravity is unscreened (1/r², infinite range). N-body simulation is O(n²).
Barnes-Hut reduces it to O(n log n) by treating distant clusters as point masses —
an approximation valid because force decays slowly.

Yukawa gravity is screened: potential ∝ e^{−μr}/r, the solution to (∇² − μ²)Φ = −ρ.
The force mediator has mass μ. Interactions beyond 1/μ are exponentially suppressed
by the field equation itself — not approximately negligible, but provably negligible.

The springs operator (L + μI)x* = μx₀ is Yukawa gravity on the graph manifold. Its
Green's function (L + μI)⁻¹ is the discrete Yukawa kernel: entry (i,j) decays as
e^{−√μ·d(i,j)} with graph distance d(i,j). We do not need Barnes-Hut's approximation
because the physics of this manifold already provides exact exponential screening.
Barnes-Hut approximates what Yukawa physics provides exactly.

BVH is still needed — but purely for the rendering question (frustum cull), not
for physics. The physics decides where particles are; the BVH decides which to draw.

### Heat-kernel BVH — one structure, dual purpose

R-1.0 §10 resolves the earlier question of "separate spatial BVH vs topological
heat LOD" definitively: they are the same structure.

Heat-kernel clustering at four τ scales {τ₀, 10τ₀, 100τ₀, 1000τ₀} directly
produces the BVH tree. Each level of clustering is a level of the hierarchy; each
node gets an AABB from its particles' spectral coordinates. The topology drives the
hierarchy; the AABB enables spatial frustum cull. No separate octree.

Because spectral coordinates derive from graph topology, BVH clusters ≈ heat-kernel
clusters by construction. The two concerns agree on "near" without needing to be
reconciled.

### Epoch / frame split

R-1.0 §2: all heavy computation (eigensolver, BVH build, NRF training) runs at
≤1 Hz as epoch work on a background thread. Frames only animate focus luminosity
and diffusion flow against frozen geometry. This is the primary architectural
constraint — it determines threading model, data structures, and API boundaries.

In Bevy: epoch thread owns `EpochState` (double-buffered). Frame loop swaps
atomically on epoch arrival. The eigensolver is never on the render thread.

### T∞ is Phase 2/3 — not Phase 1

The R-1.0 T∞ tier requires:
- per-epoch NRF training (not just inference — actual gradient steps at runtime)
- Instant-NGP hash-grid encoding (Müller 2022) with 16 levels × 2^19 entries
- CT-1.1 cross-attention conditioning (graph-context per nearest k=8 particles)
- Clifford render block (shifted geometric product — kernels not yet in honeycrisp)
- volume ray-march (128 samples/pixel, depth-varying τ)

This is months of work. R-1.0 §7.5 explicitly provides a fallback: "hash-grid-only
MLP without CT-1.1" is a valid conforming T∞ until the full implementation ships.

Phase 1 uses luminosity-weighted point splats for sub-pixel particles.
Phase 2 delivers hash-grid MLP (no CT-1.1).
Phase 3 delivers full T∞ with CT-1.1 + Clifford block.

### Streaming by proximity — omitted

Load the full .graph snapshot at world entry. At bostrom-23M scale (312MB signals,
mmap'd zero-copy) this fits in unified memory. Streaming deferred to a future spec.

---

## Complexity analysis

Bostrom at block 23M: n=2,921,230 particles, m=2,705,332 cyberlinks, avg degree ~1.85.

### Epoch operations (background, ≤1 Hz)

| operation | algorithm | cost | note |
|---|---|---|---|
| Eigensolver (full) | LOBPCG on normalized ℒ, k=32 Krylov | O(n·d̄·k) | every 64 epochs or ΔG > 5% |
| Eigensolver (incremental) | perturbation theory, h-hop neighborhood | O(\|Nₕ\|·d̄) | per epoch when applicable |
| Procrustes alignment | SVD on anchor-1024 | O(\|anchor\|) | per epoch |
| Heat-kernel BVH | Chebyshev K=20 at 4 τ scales | O(K·m) per scale | per epoch |
| NRF training (Phase 2+) | Adam on 2^16 points | O(N_train) | per epoch, ~50ms ANE |

Yukawa screening (μ parameter) gives h-local perturbation: edits propagate only
O(log 1/ε) hops. Incremental eigensolver reuses this — only the affected neighborhood
is recomputed.

### Frame operations (foreground, display refresh)

| operation | cost | hardware |
|---|---|---|
| GPU BVH traversal (frustum cull) | O(log n) | aruminium compute |
| Diffusion step (1–2 iterations) | O(m) = ~2.7M ops | acpu AMX |
| Instance buffer upload (≤10K visible) | ~240 KB | unimem |
| T2 analytic impostor draw | O(visible), tile-shaded | aruminium |
| T3 Gaussian splat draw | O(visible at T3 screen size) | aruminium |
| Edge flow UV update | O(visible edges) | aruminium |
| IOSurface composite + present | O(pixels) | aruminium + unimem |

Target frame budget on M3 Pro at 4K 120Hz (8.3ms):

```
BVH cull compute:        0.3ms   aruminium
diffusion step:          0.1ms   acpu
T2 impostor pass:        1.5ms   aruminium (tile-shaded deferred)
T3 splat pass:           1.0ms   aruminium
edge pass (bundled):     1.0ms   aruminium
composite + present:     0.5ms   aruminium + unimem
slack:                   3.9ms
```

---

## Existing code inventory

### mc crate — data layer (ready)

`mc/src/graph/record.rs`: `Cyberlink` — 128-byte fixed records (neuron, from, to,
token, amount, valence, block). `CyberlinkIter` over mmap'd bytes — zero-copy.
`mc::graph::Graph` reader — opens .graph, mmap's signals section.

Gaps: CSR adjacency builder, particle index (hemera→u32), crystal metadata reader.
These go in `render::graph`.

### honeycrisp — Apple Silicon compute (ready for adaptation)

`BufferPool` (power-of-two size classes, zero alloc hot path) → reuse for position,
focus, instance buffers. Matmul + attention kernels → T∞ NRF (Phase 2+).
`unimem` IOSurface → zero-copy frame handoff (no GPU→CPU readback).

Gaps: sparse CSR matvec, Chebyshev dispatch, GPU BVH traversal, Gaussian splat
rasterization. Clifford kernels for Phase 3.

### bevy shell — ECS and GPU bridge (ready)

`GpuBridgePlugin`: device + queue shared to main world — render crate gets them free.
`WorldState` FSM + `OnEnter`/`OnExit`/`Update` pattern — add `WorldState::Graph`.
Sugarloaf offscreen pattern (proven) → T1 label rendering in world space.

### cybergraph crate — physics (separate repo, consumed as lib)

Eigensolver, PageRank, heat-kernel approximation live in `cybergraph` crate — not
in `render`. Render imports computed positions and focus as inputs. This is the
"discrete field theory engine"; render is the observer.

---

## Implementation order (Phase 1)

| step | module | deliverable |
|---|---|---|
| 1 | `render::graph` | snapshot loader, CSR builder, particle index |
| 2 | `render::epoch::eigensolver` | LOBPCG positions; Procrustes aligned |
| 3 | `render::epoch::bvh` | heat-kernel BVH, 4 τ scales, AABB per node |
| 4 | `render::frame::cull` | GPU BVH traversal → VisibleSet + TierLevel |
| 5 | `render::frame::tiers::t2` | analytic impostor (indirect draw, tile-shaded) |
| 6 | `render::frame::tiers::t3` | Gaussian splat (Kerbl 2023) |
| 7 | `render::bevy` | GraphWorldPlugin, WorldState::Graph, Cmd+5 |
| 8 | `render::frame::tiers::t1` | labels via sugarloaf, world-space text |
| 9 | `render::frame::edges` | bundled tubes, flow UV animation |
| 10 | `render::frame::tiers::t0` | content entry: camera transition + sandbox |
| 11 | conformance | P-RENDER-TOPO, P-RENDER-POS, P-RENDER-FPS |

Phase 2: T∞ hash-grid MLP (no CT-1.1).
Phase 3: T∞ full NRF — CT-1.1 + Clifford block + volume ray-march.
