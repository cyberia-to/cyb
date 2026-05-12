---
spec: render
version: R-1.0-cyb
status: draft
conforms-to: cybergraph/reference/render.md (R-1.0)
alias: render, render spec, cyb render implementation
---

# render — cyb implementation of R-1.0

Cyb's implementation of the deterministic 3d rendering protocol defined in
[[cybergraph]] `reference/render.md` (R-1.0). Every conforming neuron running
R-1.0 on the same graph state arrives at the same world; this document specifies
how cyb produces that world.

The implementation lives in `cyb/render/` (Rust crate), integrated into the
Bevy shell as `WorldState::Graph`.

## conformance

| R-1.0 section | cyb phase | status |
|---|---|---|
| §3 spectral layout (normalized Laplacian, LOBPCG) | Phase 1 | planned |
| §4 coordinate frame (Procrustes, anchor-1024) | Phase 1 | planned |
| §5.1 schemaless visual encoding | Phase 1 | planned |
| §5.2 crystal-v5 metadata overlay | Phase 1 | planned |
| §6.1 T0 content entry | Phase 1 | planned |
| §6.2 T1 surface + label | Phase 1 | planned |
| §6.3 T2 analytic impostor | Phase 1 | planned |
| §6.4 T3 Gaussian splat | Phase 1 | planned |
| §7 T∞ NRF — hash-grid MLP only, no CT-1.1 (§7.5 fallback) | Phase 2 | deferred |
| §7 T∞ NRF — full CT-1.1 backbone + Clifford block | Phase 3 | deferred |
| §8 edges + heat-kernel bundling | Phase 1 | planned |
| §9 navigation (warp, portal-step, follow-flow) | Phase 1 | planned |
| §10 heat-kernel BVH (four τ scales) | Phase 1 | planned |
| §12 determinism contract (topo, position, pixel) | Phase 1 | planned |
| §13/14 honeycrisp backend | Phase 1 | planned |

Sub-pixel particles render as luminosity-weighted point splats until Phase 2
delivers a conforming T∞ implementation.

## architecture

### epoch / frame split

R-1.0 §2 defines two timescales. Cyb maps them onto Bevy's threading model:

**Epoch work** (≤1 Hz, background `std::thread`):
- eigensolver: LOBPCG on normalized Laplacian ℒ via `acpu` (AMX)
- Procrustes alignment to anchor-1024
- heat-kernel BVH build at four τ scales {τ₀, 10τ₀, 100τ₀, 1000τ₀}
- NRF training (Phase 2+)
- results atomically swapped into double-buffered `EpochState` resource

**Frame work** (display refresh, Bevy `Update` schedule):
- GPU BVH cull (`aruminium` compute) → VisibleSet
- tier dispatch (T0–T3 draw calls via `aruminium`)
- focus luminosity animation (1–2 diffusion steps via `acpu`)
- edge flow UV offset update
- composite → Bevy surface via `unimem` IOSurface (zero copy, no GPU→CPU readback)

Between epochs, particle positions and cluster IDs are frozen. Only focus
luminosity and edge flow animate within a frame epoch.

### Bevy ECS integration

`WorldState::Graph` is a new variant in `cyb-shell/src/worlds/mod.rs`.

**Components** (visible set only; ECS holds ≤10K entities, not all 3M particles):

```rust
struct VisibleParticle(u32);  // ParticleIdx into epoch GPU buffers
struct TierLevel(u8);         // 0=T0, 1=T1, 2=T2, 3=T3
```

**Resources:**

```rust
struct EpochState {           // double-buffered; swapped atomically on epoch arrival
    positions:   HcBuffer,    // n × 12 B  (f32x3 aligned spectral coords)
    focus:       HcBuffer,    // n × 4 B   (f32 φ*)
    bvh:         HcBuffer,    // heat-kernel BVH nodes
    cluster_ids: HcBuffer,    // per-particle cluster ID at each τ scale
}
struct GraphCamera {
    tau:        f32,          // current heat-kernel scale
    tau_target: f32,          // smooth interpolation target (driven by scroll)
}
```

**System schedule:**

```
PreUpdate:   swap_epoch_if_ready     — atomic swap from background epoch thread
Update:      tick_diffusion          — 1–2 PageRank steps (acpu)
             update_camera_tau       — scroll input → τ smooth-step
             gpu_bvh_cull            — aruminium compute → VisibleSet + TierLevel
             sync_visible_entities   — diff previous VisibleSet; spawn/despawn ECS entities
PostUpdate:  dispatch_tiers          — T0/T1/T2/T3 draw via aruminium
             animate_edges           — UV offset update for flow texture
             composite               — IOSurface present (unimem zero-copy)
```

**OnEnter(Graph):**

1. mmap `.graph` snapshot signals section (zero-copy via `mc::graph::Graph`)
2. build CSR adjacency matrix from `CyberlinkIter`
3. show loading overlay (Bevy UI entity)
4. spawn background epoch thread → eigensolver → Procrustes → BVH build
5. on first `EpochState` arrival: despawn overlay, begin frame loop

**OnExit(Graph):**

- despawn visible particle entities + `GraphCamera`
- keep `EpochState` alive (eigenvectors expensive; reused on re-entry)
- background epoch thread continues paused until re-entry

### heat-kernel BVH

Implements R-1.0 §10. Heat-kernel clustering at four τ scales produces a 4-level
hierarchy. Each BVH node stores:
- AABB of contained particles (from spectral coordinates)
- aggregated φ* (luminosity sum for cluster billboard)
- dominant cluster color (for T∞ far-field tinting)
- up to 16 children

Built on `acpu` (AMX) during epoch. GPU cull pass (`aruminium` compute) traverses
it per frame, emitting indirect-draw argument buffers for each tier.

The same hierarchy serves both spatial frustum cull (AABB) and topological LOD
(cluster membership at each τ level). One structure, dual purpose.

### rendering tiers

#### T0 — content entry

When particle p's projected screen diameter exceeds s_T0 (200 px):

- camera transitions to 3r(p) along surface normal, looking at X'(p)
- content renderer dispatches by particle language: text, image, video, WASM component
- WASM runs sandboxed with CPU/GPU/memory quotas (see [[cyb/component]])
- exit: camera reverses along entry path until projected diameter < s_T0

Bevy: content entry triggers a `ContentWorldPlugin` sub-state within
`WorldState::Graph`. Content render pass composites over the graph scene.

#### T1 — surface + label

Analytic impostor surface + sugarloaf text label in world space + edge preview
with flow texture. Labels attached to tube midpoints at edge T1.

#### T2 — analytic impostor

One quad per particle; fragment shader ray-casts the analytic primitive determined
by crystal-type overlay (or degree-role inference when no overlay is active).
Single GPU indirect draw per tier batch. Tile-shaded deferred via `aruminium`
on Apple Silicon.

#### T3 — Gaussian splat

3D anisotropic Gaussian splat per Kerbl et al. 2023. Covariance isotropic by
default, optionally stretched along local u₅ eigenvector gradient. Rasterized
front-to-back with alpha blending. Handles the instanced-geometry to sub-pixel
transition band; cross-fades with T∞ at the 1 px boundary.

#### T∞ — deferred

Phase 2: hash-grid-only MLP (Müller 2022, Instant NGP), no CT-1.1 conditioning.
Valid per R-1.0 §7.5 fallback; produces correct world at reduced quality.

Phase 3: full NRF — CT-1.1 cross-attention conditioning, Clifford render block
(shifted geometric product), volume ray-march 128 samples/pixel, depth-varying τ.

Until Phase 2: sub-pixel particles render as luminosity-weighted point splats.

### honeycrisp backend

Implements R-1.0 §13.1 `RenderBackend` trait.

| component | role |
|---|---|
| `aruminium` | Metal GPU: BVH cull compute, impostor / splat / edge draw, composite |
| `rane` | ANE: NRF head inference (Phase 2+) |
| `acpu` | AMX: LOBPCG eigensolver, Procrustes alignment, BVH build, diffusion steps |
| `unimem` | IOSurface: zero-copy frame handoff across CPU / GPU / ANE |

Kernels needed before Phase 1 is complete (honeycrisp roadmap):

- sparse CSR matvec — for LOBPCG and diffusion steps
- Chebyshev polynomial dispatch — heat-kernel approximation at each τ scale
- GPU BVH traversal compute — aruminium indirect-draw emission
- Gaussian splat rasterization — T3 front-to-back alpha pass

Clifford kernels (`roll`, `shifted_inner`, `shifted_wedge`, `clifford_block`)
required for Phase 3 only.

## crate layout

```
cyb/render/
  Cargo.toml

  src/
    lib.rs                    — CyberRenderPlugin (Bevy plugin entry)

    graph/
      snapshot.rs             — GraphSnapshot resource: mmap'd .graph + particle index
      adjacency.rs            — CSR builder from CyberlinkIter (mc crate)
      vocab.rs                — hemera hash → ParticleIdx; crystal metadata reader

    epoch/
      mod.rs                  — EpochState (double-buffered); background thread lifecycle
      eigensolver.rs          — LOBPCG on normalized Laplacian ℒ (acpu AMX)
      procrustes.rs           — Procrustes alignment to anchor-1024
      bvh.rs                  — heat-kernel BVH at four τ scales (acpu AMX)

    frame/
      cull.rs                 — GPU BVH traversal → VisibleSet + TierLevel (aruminium)
      diffusion.rs            — 1–2 PageRank steps per frame (acpu)
      edges.rs                — bundled edge tubes + flow UV animation
      composite.rs            — IOSurface present (unimem zero-copy)
      tiers/
        t0.rs                 — content entry: camera transition + ContentWorldPlugin
        t1.rs                 — surface impostor + sugarloaf world-space label
        t2.rs                 — analytic impostor (indirect draw, tile-shaded deferred)
        t3.rs                 — Gaussian splat (Kerbl 2023, front-to-back alpha)
        tinf.rs               — point-splat placeholder; Phase 2/3 stub

    bevy/
      plugin.rs               — CyberRenderPlugin: systems + render graph nodes
      world.rs                — GraphWorldPlugin: OnEnter / Update / OnExit
      components.rs           — VisibleParticle, TierLevel
      resources.rs            — EpochState, GraphCamera, GpuBuffers
      camera.rs               — scroll → τ smooth-step; camera-to-τ mapping
```

## phase 1 milestones

| step | deliverable | verifies |
|---|---|---|
| 1 | `graph/` — snapshot loader, CSR, particle index | .graph loads; particle count matches |
| 2 | `epoch/eigensolver` + `procrustes` | positions match R-1.0 §3–4 within ε |
| 3 | `epoch/bvh` — heat-kernel BVH four τ scales | P-RENDER-TOPO |
| 4 | `frame/cull` — GPU BVH traversal | frustum cull correct; VisibleSet ≤10K |
| 5 | `frame/tiers/t2` — analytic impostor | particles visible at mid-range |
| 6 | `frame/tiers/t3` — Gaussian splat | sub-pixel particles visible |
| 7 | `bevy/` — GraphWorldPlugin wired into WorldState::Graph | world launches, Cmd+5 |
| 8 | `frame/tiers/t1` — labels via sugarloaf | particle titles in world space |
| 9 | `frame/edges` — bundled tubes + flow | cyberlinks animate |
| 10 | `frame/tiers/t0` — content entry | particle opens; content renders |
| 11 | conformance run | P-RENDER-TOPO, P-RENDER-POS, P-RENDER-FPS |
