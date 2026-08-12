---
tags: cyb
crystal-type: entity
crystal-domain: cyber
---
# Features

the capabilities [[cyb]] provides on top of its [[cyb/os]] kernel and [[cyb/stack]] crates. three pillars: a render engine that replaces the DOM, a contract engine that replaces client-server, and a compatibility layer that bridges legacy web.

## PureRender — Flat Streams Instead of Trees

DOM is a tree. trees breed cascading complexity: reflow, repaint, layout thrashing, z-index stacking contexts. cyb replaces the tree with a stream.

```
stream Article {
  text    { # Hello World }
  pixels  { hero.webp }
  text    { Main body of the article... }
  formula { E = mc^2 }
  table   { metrics.csv }
  vector  { diagram.svg }
  action  { Subscribe -> subscribe() }
}
```

a stream is a sequence of primitives, top to bottom. each block knows its own size. layout is trivial. scrolling is an offset. virtualization is free — only render the visible slice. two levels of nesting maximum:

```
grid [sidebar: 300px, main: 1fr] {
  stream sidebar { vector { logo.svg }  struct { nav.toml } }
  stream main    { text { # Dashboard } table { sales.csv } }
}
```

same primitives, different layout mode. screen, PDF, print — same component, same pipeline, different output target.

### component as a single unit

one file. one scope. everything inside.

```
component Dashboard {
  state metrics: Table = load("metrics.csv")
  state filter: Choice = { options: ["day", "week", "month"], selected: "week" }

  <stream>
    <text>{ # Metrics for {filter.selected} }</text>
    <input choice bind={filter} />
    <table data={metrics} filter={filter.selected} />
    <Canvas2D ref={chart} />
    <svg><path d={trendLine(metrics)} stroke="blue" /></svg>
    <formula>{ \sum_{i=1}^{n} x_i }</formula>
  </stream>

  style {
    table { border: 1px solid gray }
    path  { stroke-width: 2 }
  }

  fn trendLine(data: Table): string { ... }
  fn render(chart: Canvas2D) { ... }
}
```

the compiler sees the entire component. dead CSS eliminated. static primitives computed at compile time. reactivity only where `state` exists.

### compilation: everything to WASM + WGSL

no interpreter. no JIT. no runtime parsing. everything compiles.

strict TypeScript: full type system, async/await, generics, interfaces, enums, unions, modules, pattern matching, destructuring. no `any`, no `eval()`, no `arguments`, no prototype chains, no `this` binding, no runtime type checks.

```
Source (TS strict + HTML + CSS + SVG + LaTeX)
          |
    Parse + validate + type check
          |
    Unified IR
          |
    Optimize (DCE, constant fold, static layout, shader compile)
          |
   +------+------+
  WASM          WGSL
  logic         render
   |              |
   +------+------+
          |
     WGPU runtime
          |
  Vulkan / Metal / DX12 / OpenGL ES
```

### GPU render: everything is shaders

every primitive is a draw call or compute dispatch:

```
text        ->  rustybuzz shaping -> swash raster -> GPU glyph atlas
struct      ->  text render + collapse/expand + indent
table       ->  grid compute -> text per cell (virtualized)
vector      ->  Vello: path -> tiles -> compute fill
pixels      ->  texture upload -> sampler -> fragment shader
video       ->  hardware decode -> texture per frame
sound       ->  audio pipeline (visual: waveform compute)
formula     ->  LaTeX parse -> glyph positioning -> Vello paths
action      ->  hit-test region + feedback shader
input *     ->  cursor shader + selection + validation
```

frame loop:

```
every frame (16ms @ 60fps):
  1. WASM: process events, update state
  2. WASM: recompute layout for changed subtrees only
  3. WASM: emit flat draw command array
  4. WGSL: compute pass — text, SVG, formulas, backgrounds
  5. WGSL: render pass — composite into framebuffer
  6. present
```

## Smart Contracts — CosmWasm Native

in Chrome, a dApp is a JS app that talks to a blockchain node over HTTP. in cyb, [[smart contracts]] run locally in the same runtime as UI.

```
Chrome:     JS -> fetch() -> REST API -> Go node -> CosmWasm -> result
            Round trip: 200-2000ms

Cyb:        Component (wasmi) -> direct call -> Contract (wasmi) -> result
            Round trip: <1ms
```

the component IS the contract:

```
component/contract Token {
  state balances: Map<Address, u128>

  <stream>
    <text>{ # Balance: {balances[viewer]} }</text>
    <input text bind={recipient} />
    <input range bind={amount} max={balances[viewer]} />
    <action>{ Send -> transfer(recipient, amount) }</action>
  </stream>

  fn transfer(to: Address, amount: u128) {
    // executes in CosmWasm sandbox
    // simulates locally, signs and broadcasts when online
  }
}
```

no separation between frontend and on-chain logic. one file, one scope, one runtime. UI and state machine unified. gas metering: wasmi counts fuel per instruction. every WASM module — UI, contract, plugin — is metered.

## Legacy Web Compatibility

### SLM legacy bridge

a small language model (~100-300M parameters) that understands intent behind old CSS:

```
Layer 1: Native subset       -> direct compilation
Layer 2: Legacy -> SLM       -> cached permanently
Layer 3: Graceful degradation -> unknown ignored
```

### WASM adoption

import adapter auto-detects what a module needs:

```
.wasm arrives -> inspect imports
  -> wasi_snapshot_preview1.* -> WASI shim
  -> __wbindgen_*             -> wasm-bindgen shim
  -> env.emscripten_*         -> Emscripten compat
  -> env.db_read/db_write     -> native CosmWasm
  -> unknown                  -> SLM classifies, generates adapter
```

| Chrome concept | Cyb upgrade |
|----------------|------------|
| Cookie | Cryptographic keypair |
| localStorage | SQLite |
| fetch to REST | Direct contract call (<1ms) |
| IndexedDB | SQLite + Merkle proofs |
| Service Worker | [[radio]] content-addressed storage |
| OAuth / JWT | Cryptographic signatures |
| WebSocket | P2P [[libp2p]] connection |

## Local Inference

three portable formats for three processor types:

| Processor | Format | What cyb uses it for |
|-----------|--------|---------------------|
| CPU | WASM (wasmi) | Logic, layout, events, contracts, state |
| GPU | WGSL (wgpu) | Pixels, vectors, text, video, ML fallback |
| NPU | ONNX (burn-webnn) | SLM inference, AI features |

```
Browser:   WASM (native) + WGSL (WebGPU) + ONNX (WebNN -> NPU)
Desktop:   WASM (wasmi) + WGSL (wgpu -> Vulkan/Metal/DX12) + ONNX (burn)
Mobile:    WASM (wasmi) + WGSL (wgpu -> Metal/GLES) + ONNX (CoreML/NNAPI)
```

same codebase. same pixels. everywhere.

## Numbers

### 130K lines

```
PureRender                              ~100K
  15 primitives (parse, layout, render)    40K
  TS strict compiler                       22K
  Runtime (wgpu, layout, events)           22K
  Infrastructure (PDF, fetch, SQLite)      17K

CosmWasm integration                     ~26K
WASM adoption layer                      ~14K
burn-webnn                               ~12K

TOTAL                                   ~130K
```

CybOS core: ~85-125K lines (human-authored, auditable by one person in a month). neural driver layer: ~500K-1M lines (model-generated, compiler + test validated).

### cyb vs Chrome

| | Chrome | Cyb |
|-|--------|-----|
| Codebase | 35M lines C++ | 130K lines [[Rust]] |
| Render | Blink (DOM/CSS) | PureRender (9 primitives + 4 layouts) |
| WASM | V8 JIT | wasmi (deterministic, metered) |
| Identity | Cookie | Keypair |
| State | Server-side | Local SQLite + on-chain |
| Contracts | Via HTTP to node | Native, same runtime as UI |
| Binary | ~150+ MB | ~10 MB |

270× reduction in code for a system that does more.

see [[cyb/stack]] for the seven crates. see [[cyb/os]] for the kernel. see [[cyb/apps]] for the applications built on these features