---
tags: cyb, core
alias: zero-copy, hardware stack, cyb-mem, aruminium, rane, ramx
---
zero-copy inference stack for Apple Silicon — direct hardware access in Rust, bypassing every OS abstraction layer between computation and silicon

```
standard path:    Rust → Swift/ObjC wrapper → Framework → IOKit → hardware
direct path:      Rust → IOKit symbols or CPU instructions → hardware
```

## components

### aruminium — Metal GPU

accesses Metal via direct symbol linkage into Metal.framework without Objective-C dispatch overhead. command buffer construction, resource allocation, and kernel dispatch go through raw C-compatible Metal symbols

benchmark: 3× faster than Ollama for equivalent inference workloads on M1 Pro

### rane — Apple Neural Engine

links directly to IOANEInterface symbols in IOKit, bypassing CoreML and the Neural Engine framework entirely. the ANE processes fixed [[neural]] network operations in hardware at extremely low power

### ramx — Apple Matrix Extensions

AMX is an undocumented extension to the ARM instruction set present on all Apple Silicon. hardware matrix multiply-accumulate registers not exposed through any public API. implemented directly in Rust via inline assembly

| matrix size | vs Apple Accelerate |
|-------------|-------------------|
| 16×16 | 3× faster |
| 2K×2K | 10% faster |
| medium | 2× slower (tiling not yet optimal) |

the 16×16 result matters: this is the regime where [[attention]] head computations live (head_dim = 128, parallelized)

### cyb-mem — unified physical memory

the central infrastructure component. standard allocators return virtual addresses. DMA-capable hardware (AMX, ANE, NVMe) requires physical addresses. the gap costs a copy every time data crosses the boundary

cyb-mem provides physically pinned memory with known physical addresses through Hypervisor.framework EL2 stage-2 page table mapping

| operation | cyb-mem | malloc | mmap |
|-----------|---------|--------|------|
| alloc 4KB | 0.9ns | 18ns | 464ns |
| free all | 0.3ns | ~5ms | ~5ms |
| pinned | yes | no | no |
| HW shared | CPU+GPU+AMX+ANE | CPU only | CPU only |

the 0.3ns free: a single atomic store (cursor = 0). arena reset is O(1) regardless of allocation count. pages remain pinned — the next [[inference]] cycle costs zero setup

HW shared: CPU+GPU+AMX+ANE means a buffer allocated once is visible to all compute units simultaneously without any copy. this is the property that makes the full zero-copy [[pipeline]] possible

### cyb-store — NVMe storage (planned)

NVMe DMA directly into cyb-mem PhysPage without CPU involvement. path: m1n1 bare metal access → NVMe controller MMIO at physical address 0x23e010000 → direct submission queue writes with PA from cyb-mem

```
NVMe DMA → PhysPage.pa → AMX/ANE/Metal
```

completes the pipeline: storage → compute → result with zero copies

## the zero-copy principle

every byte travels from storage through preprocessing to [[neural]] [[inference]] without a single memory copy. each component reads from and writes to the same physical addresses. the only data movement is computation itself

see [[pipeline]] for the full .cyb cycle. see [[runtime]] for multi-model memory sharing

discover all [[concepts]]
