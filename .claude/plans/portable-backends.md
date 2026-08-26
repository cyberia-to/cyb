# Portable backends: one codebase, fast on Apple and on Android

## Goal

`make android-run` builds and runs the **same** cyb that runs on macOS — same Bevy, same four
worlds, same mir — with a compute path that is fast on both. honeycrisp stays the default on
Apple Silicon and loses nothing; it stops being the only path.

Two seams carry the whole plan, and both already exist in the codebase:

```
mir::backend::RenderBackend      R-1.0 §13.1 — trait + CpuReferenceBackend (§13.2)
unimem::Block                    pinned block; only block.rs + ffi.rs are Apple-specific
```

Nothing here invents an abstraction. Both seams were specified and then bypassed.

---

## What exists today (verified)

honeycrisp is four drivers; only one of them is about the GPU:

| crate | LOC | role | portable analogue |
|---|---|---|---|
| `acpu` | 16 018 | NEON / AMX / SME | none — CPU, out of scope |
| `rane` | 2 877 | ANE, MIL bytecode | none — out of scope |
| `aruminium` | 2 315 | Metal GPU | wgpu, near 1:1 |
| `unimem` | 610 | IOSurface, arena, pool | AHardwareBuffer / memfd |

`unimem` splits cleanly: `block.rs` (158) and `ffi.rs` (81) touch IOSurface, `lib.rs` (34)
mentions it in error text. `grid.rs` (101), `layout.rs` (106) and `tape.rs` (130) are already
portable Rust.

mir calls honeycrisp around its own trait in exactly three places:

| site | call | portability |
|---|---|---|
| `mir/src/epoch/eigensolver.rs:56` | `acpu::sparse::csr_matvec_set` | trivial — CSR matvec |
| `mir/src/frame/diffusion.rs:10` | `acpu::sparse::csr_matvec_set` | trivial — same |
| `mir/src/frame/cull.rs` | `aruminium::Gpu/Pipeline/Queue/Buffer` | the real work |

14 Metal kernels are embedded as MSL strings: `cull.rs` 1, `edges.rs` 6, `tiers/t1.rs` 3,
`tiers/t2.rs` 2, `tiers/t3.rs` 1, `tiers/tinf.rs` 1.

Blocking today: `unimem` links `IOSurface` and `CoreFoundation` as frameworks, so
`cargo build --target aarch64-linux-android` cannot compile mir at all. mir and `arboard` are
behind the `desktop` feature as a stopgap (commit `59115e1f`) — which means Android currently
has **no** mir, i.e. not one codebase. P1 and P2 repay that.

---

## Verification model

Straight from `cyber/engineering` — four dimensions over one workload:

| dimension | implementation | role |
|---|---|---|
| reference | `CpuReferenceBackend` (§13.2, fp64) | ground truth, already written |
| manual | honeycrisp (Metal + AMX) | the expert floor on Apple |
| automated | wgpu (Metal / Vulkan) | portable path, races the floor |

`mir::conformance` already emits `P_RENDER_TOPO` / `P_RENDER_POS` (ε = 1e-3 × R_scene) and
serialises to `r1_conformance.toml`. Every phase below is done when all three dimensions agree
on that report. A backend that cannot match reference is a soundness bug, not a slow path.

Standing rule: **honeycrisp stays the default on `target_vendor = "apple"`.** If the wgpu path
ever matches it on Apple, that is a measurement to bring to the user, not a licence to delete.

---

## Phases

### P1 — mir speaks only through `RenderBackend` (~1–2 sessions)

- Add `csr_matvec_set` to `mir::backend` in portable Rust; `AppleBackend` forwards to
  `acpu::sparse`, every other backend uses the portable one. Same signature, same result.
- Route `eigensolver.rs:56` and `diffusion.rs:10` through the backend handle.
- Move `frame/cull.rs`'s aruminium use behind `RenderBackend::compute_dispatch`; the MSL string
  stays for now, carried as `AppleBackend`'s shader source.
- `mir/Cargo.toml`: honeycrisp deps become optional under
  `[target.'cfg(target_vendor = "apple")'.dependencies]`, features `apple` (default on Apple)
  and `cpu`.

**Deliverable:** `cargo build --target aarch64-linux-android -p mir` succeeds.
**Verification:** conformance report byte-identical between `apple` and `cpu` backends on macOS.

### P2 — Bevy on Android, one codebase (~2–3 sessions) — DONE 2026-08-26 (emulator-verified)

Landed as cyb e943eacc + c2e8968c, mir a0e17a1. All four worlds verified by touch on the API 34
arm64 emulator: terminal prompts (nushell cross-compiled untouched), sigma trades, the landing
renders from a baked-in cell, mir runs the CPU epoch with paint pending P3. World tabs in the
bottom chrome are the touch navigation. Gotchas that cost time, recorded for the next pass:
libc++_shared.so must ship in jniLibs (bevy's android_shared_stdcxx); android-activity 0.6.0
pairs with games-activity 2.0.2 but 0.6.1 with 4.4.0 — a mismatch dies at JNI registration
(onTouchEventNative); GameActivity is an AppCompatActivity and refuses a non-AppCompat theme.
Physical-device run pending USB debugging on the Pixel.


- Undo the stopgap: mir returns to unconditional deps in `shell/Cargo.toml`; `arboard` gets a
  cfg-gated no-op on Android rather than being cut.
- Replace `shell/src/android/mod.rs` (tao + wry) with `android_main(AndroidApp)` calling the
  same plugin set as `main.rs`. Delete the stub. See `android-support.md` P1 for the manifest
  and Gradle details — that plan's paths predate the `bevy/` → `shell/` rename.
- CPU backend selected on Android. Slow, correct, real.

**Deliverable:** `make android-run` shows the Graph world on the Pixel, Cmd-less nav via tray
equivalent, `make android-log` clean of panics.
**Verification:** the four worlds reachable on device; conformance run on-device matches macOS.

This is the phase that answers "билдить под андроид из одной кодовой базы". Everything after it
is speed.

### P3 — wgpu backend (~3–4 sessions)

- `mir::backend::WgpuBackend` implementing `RenderBackend`. wgpu 27.0.1 is already in the tree
  via Bevy; `evy/forks/naga` is its shader compiler. No new dependency weight.
- Port the 14 MSL kernels to WGSL. Two traps, both pervasive and both cheap if done up front:
  - WebGPU has no non-uniform grid — `dispatch_workgroups` is in whole groups, so every kernel
    needs an `if (gid >= n) { return; }` guard that Metal's `dispatchThreads` made unnecessary.
  - No intra-pass barrier. `Batch::memory_barrier_buffers()` sequences become separate passes.
- Feature-gate the natives cyb actually needs and check them at adapter selection:
  `TIMESTAMP_QUERY`, `MAPPABLE_PRIMARY_BUFFERS`, and `SHADER_INT64` where a kernel touches
  Goldilocks. Metal advertises `SHADER_INT64` (`wgpu-hal/src/metal/adapter.rs:972`); Vulkan
  reports it per device, so any 64-bit field kernel needs a u32-pair fallback or it will not run
  on some Android GPUs. Decide per kernel, do not assume.

**Deliverable:** GPU cull on Android; wgpu selectable on macOS via `--features portable`.
**Verification:** all three dimensions agree; record wgpu-vs-honeycrisp timings on macOS in
`mir/.claude/other/` — that number decides whether P5 is ever worth doing.

### P4 — `unimem::Block` gets an Android backend (~2–3 sessions)

Split the Apple-specific 240 lines behind one trait, leave the other 370 alone:

```
unimem::Block
├── apple    IOSurface                 → MTLBuffer(noCopy)  → hal metal::Buffer::from_raw
└── android  memfd → VkImportMemoryFd  → VkBuffer(imported) → hal vulkan::Buffer::from_raw
```

- `block/apple.rs` is today's `block.rs` + `ffi.rs`, unchanged behaviour.
- `block/android.rs`: `memfd_create` → `mmap` for the CPU view → import as `VkDeviceMemory`.
  wgpu-hal already enables `khr::external_memory_fd` and `ext::external_memory_dma_buf` when the
  driver has them (`wgpu-hal/src/vulkan/adapter.rs:1106–1112`), so this needs no fork.
- Hand the result to wgpu:
  `wgpu_hal::vulkan::Buffer::from_raw(vk_buffer)` (`vulkan/mod.rs:908`) →
  `Device::create_buffer_from_hal::<Vulkan>` (`wgpu/src/api/device.rs:371`). Metal mirrors it via
  `metal::Device::buffer_from_raw` (`metal/device.rs:361`).
- `Gpu::wrap(&unimem::Block)` keeps its exact signature. Only the innards branch.

Three things that will bite if not designed in from the start:
- **Coherency.** Metal shared storage handles it; Vulkan needs `HOST_COHERENT` or explicit
  `vkFlushMappedMemoryRanges` / `Invalidate`. Getting this wrong produces wrong data and no error.
- **Alignment.** `VK_EXT_external_memory_host` wants `minImportedHostPointerAlignment` (commonly
  4 KB), and Android 15+ runs 16 KB pages — `.cargo/config.toml` already passes
  `-C link-arg=max-page-size=16384` for the Android target, so unimem's allocator must agree with
  that number rather than assume 4 KB.
- **Ownership.** wgpu refuses to map an imported buffer, by design and stated in its own source.
  unimem stays the sole owner of the CPU view. That is the right split; do not add a second one.

Cheaper prelude, worth doing first and measuring before committing to the import path: enable
`MAPPABLE_PRIMARY_BUFFERS` and map storage buffers directly. Its own doc says it exists for
systems that share memory between CPU and GPU. That removes the staging copy without a line of
unsafe — the remaining gap is that portable wgpu cannot hold a mapping across a submit.

**Deliverable:** one allocation from entry to exit on both platforms.
**Verification:** allocation counter and a copy-free assertion in the cull path; STREAM-style
bandwidth number on device.

### P5 — retire what lost (0–1 session, conditional)

Only if P3's measurements say the wgpu path matches honeycrisp on Apple. Until then this phase
does not exist.

---

## File change summary

| file | change |
|---|---|
| `mir/src/backend.rs` | portable `csr_matvec_set`; `AppleBackend`; `WgpuBackend` |
| `mir/src/epoch/eigensolver.rs` | call through the backend handle |
| `mir/src/frame/diffusion.rs` | same |
| `mir/src/frame/cull.rs` | dispatch through the trait; MSL becomes one backend's source |
| `mir/src/frame/{edges,tiers/*}.rs` | WGSL alongside MSL |
| `mir/Cargo.toml` | honeycrisp under `cfg(target_vendor = "apple")`; `apple`/`cpu`/`portable` |
| `honeycrisp/unimem/src/block/{apple,android}.rs` | split; `mod.rs` holds the trait |
| `honeycrisp/unimem/Cargo.toml` | Apple deps target-gated |
| `honeycrisp/aruminium/src/device.rs` | `wrap` takes the trait, not `IOSurfaceRef` |
| `cyb/shell/Cargo.toml` | mir unconditional again; `arboard` cfg-gated |
| `cyb/shell/src/android/mod.rs` | deleted, replaced by `android_main` |

---

## Estimate

| phase | sessions |
|---|---|
| P1 — mir behind its own trait | 1–2 |
| P2 — Bevy on Android, one codebase | 2–3 |
| P3 — wgpu backend + 14 kernels to WGSL | 3–4 |
| P4 — unimem Android backend, zero-copy both sides | 2–3 |
| **total to fast on both** | **8–12** |

P1+P2 alone — **3–5 sessions** — gets the real cyb running on the phone from one codebase.
Everything after that is throughput.

## Open questions

1. Goldilocks on Android: `SHADER_INT64` or u32-pair emulation? Decide per kernel in P3 after
   reading which of the 14 actually touch field arithmetic.
2. `AHardwareBuffer` instead of memfd in P4 — needed only when a buffer must be shared with the
   camera, media, or another process. Defer until the Sense world asks for it; wgpu-hal does not
   request that extension, so it would mean building the `ash::Device` via
   `vulkan::Adapter::device_from_raw`.
3. Does the Android terminal world need sugarloaf's own wgpu device, or can it share mir's?
   Two devices on a mobile GPU is a real cost.
