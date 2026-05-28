---
tags: cyb, core, architecture
crystal-type: pattern
crystal-domain: cyber
---

# cyb rendering

one runtime, one chrome, exclusive worlds. two rendering models. five surfaces. zero compositing fights.

---

## the insight

cyb renders fundamentally different kinds of content — text streams, 3D spatial graphs, terminal output, media frames, arbitrary external web. they belong to two natively different rendering models, not one. trying to unify them as a single overlay-stacked surface is what creates compositing problems. treating them as exclusive worlds dispatched by a uri scheme, with chrome as a separate overlay layer, makes everything compose cleanly.

three commitments:

- [[bevy]] is the runtime. it owns the window, input, wgpu device, ecs, and world state machine. it does not render application content itself
- worlds are exclusive. exactly one world owns the surface at a time. switching worlds is a state transition, not a z-order rearrangement
- chrome is a cell-grid overlay rendered by bevy as the last pass over whichever world is active. it never lives inside a world

the chrome carries the address bar, commander, and status — the navigation and identity layer that follows the [[neuron]] across every world. worlds focus on rendering their content; chrome handles cross-world concerns.

---

## the two rendering models

each surface in cyb picks the model that matches its data shape

### model a — continuous pixel positioning

free-form coordinates, sub-pixel anti-aliasing, arbitrary shapes, transforms, blending. required when the content is inherently spatial or arbitrary.

surfaces that use model a:

| surface | renderer | content |
|---|---|---|
| graph | [[mir]] (wgpu) | 3d spatial [[cybergraph]] navigation |
| media | symphonia + decoder | audio waveforms, video frames |
| web | [[wry]] (wkwebview / webview2 / webkitgtk) | arbitrary external sites |

### model b — discrete cell grid

integer cell coordinates, monospace text, ansi styling, snap-to-grid positioning. simpler to reason about, prove, test, record, and accessibility-map. ideal when content is text-structured or form-structured.

surfaces that use model b:

| surface | renderer | content |
|---|---|---|
| terminal + all cyb apps | [[sugarloaf]] (wgpu) | nushell sessions, agent conversations, cyb apps (oracle, brain, sense, sigma, settings, wallet, chat) |
| chrome overlay | bevy cell renderer | address bar, commander, status — shared across worlds |

model b is a strict subset of model a. cells snap to integer coordinates; pixel-positioned content uses arbitrary coordinates. both rasterize through the same wgpu device, share the same glyph atlas, share the same gpu resources. one substrate, two semantic modes.

---

## the stack

```
bevy
  owns: window (winit), wgpu device + queue,
        input events, ecs, world state machine,
        chrome overlay render pass
  ↓
  ├── exclusive world (one active at a time):
  │
  │   sugarloaf   → cell grid, wgpu
  │                 hosts terminal sessions and all native cyb apps
  │                 default world; the bedrock surface
  │
  │   mir         → 3d spatial graph, wgpu
  │                 cinematic cybergraph navigation
  │                 activated by cyb://graph/...
  │
  │   wry         → external surface (os webview)
  │                 open web, pdf, video, agent browser
  │                 activated by cyb://web/...
  │
  │   media       → continuous wgpu
  │                 audio + video playback
  │                 activated by cyb://media/...
  │
  └── chrome overlay (always on top, every world):
      → cell grid rendered by bevy
      → address bar (top), commander (bottom), status
```

[[bevy]] never renders application content. its job is orchestration: route input, manage world state, run the render graph. each world plugs into the render graph as an exclusive content renderer, and chrome plugs in as the final overlay pass.

---

## chrome — cross-world overlay

chrome is the layer the [[neuron]] sees regardless of which world is active. it must transcend worlds. four cell rows total

```
┌─ cyb://graph/ai/safety/alignment ───────  master · 12 peers · 482 ξ ─┐  ← address bar (1 row)
│                                                                       │
│  [ active world renders here — sugarloaf, mir, wry, or media ]        │
│                                                                       │
│                                                                       │
│                                                                       │
│                                                                       │
├───────────────────────────────────────────────────────────────────────┤
│  > _                                                     ⌘k commander │  ← commander (1 row)
└───────────────────────────────────────────────────────────────────────┘
```

chrome cells render via bevy's render graph as the final pass. the active world has already painted its surface (cells, voxels, frames, pixels — whichever); chrome overlays text cells on top. no compositing across heterogeneous engines, no os-window stacking, no transparency hack. just an extra render pass in the same wgpu surface.

the chrome cell renderer shares primitives with sugarloaf — same shaders, same glyph atlas, same cosmic-text shaping pipeline. the only difference is where the cells get drawn from: sugarloaf draws cells representing terminal/app content within its world; chrome draws cells representing navigation/status above any world.

---

## sugarloaf as the universal cell canvas

[[sugarloaf]] is more than a terminal renderer. in cyb it is the substrate for every cell-grid surface — terminal sessions, ai chat, agent conversations, settings forms, wallet ui, knowledge graph viewers, oracle search, sense feeds, sigma wallet. all of these are cell-grid uis. all of them share the same rendering pipeline.

### the cell-grid model

```
sugarloaf surface = grid of cells
  cell = (column, row, glyph, fg, bg, attrs)
  ansi escape codes drive styling and positioning
  osc escape codes embed cyb-native rich content
```

the cell grid is the logical structure. the visual rendering is gpu-accelerated with sub-pixel anti-aliasing, ligatures, smooth scroll inertia, gradient backgrounds, backdrop blur, semi-transparency, and animation. cell-grid layout does not mean austere visual style — modern terminals (wezterm, kitty, ghostty, warp) demonstrate the visual depth available within this model.

### what sugarloaf hosts

| content type | source |
|---|---|
| terminal session | nushell or any pty-speaking process via [[portable-pty]] |
| ai agent | streaming llm output rendered as ansi |
| neuron conversation | peer's messages over radio rendered as ansi |
| cyb apps | prysm cell-grid compositions rendered directly |
| session replay | recorded byte stream replayed |
| pipe chain | composable: `find-particles | summarize | claude` |

a single molecule shape (`shell`) covers all six cases. only the backend differs. that is the architectural compression that makes terminal-first feasible at this scope.

### osc extensions for inline particles

terminals already extend ansi via osc (operating system command) sequences. iterm2, kitty, and wezterm all use osc codes for inline images, hyperlinks, and metadata. cyb defines its own osc namespace for [[particle]] references

```
\x1b]cyb;particle=bafyrei...;type=image/png\x1b\\
\x1b]cyb;particle=bafyrei...;type=video/mp4\x1b\\
\x1b]cyb;particle=bafyrei...;type=audio/opus\x1b\\
\x1b]cyb;particle=bafyrei...;type=application/cyb-3d\x1b\\
\x1b]cyb;widget=date-picker;value=2026-05-26\x1b\\
\x1b]cyb;hyperlink=cyb://particle/bafyrei...\x1b\\
```

a renderer that understands the cyb namespace fetches the particle via [[radio]] and renders it inline at the cursor position. non-cyb terminals ignore the osc and render the rest of the stream — graceful degradation built in.

recorded sessions reference particles by hash, not by inline bytes. a recording of a chat with images and a video stays tiny — the media is fetched from radio at replay time. cybergraph-scale session archiving becomes practical.

---

## mir — the 3d escalation

[[mir]] is the spatial cybergraph viewer. when the address resolves to `cyb://graph/...`, bevy transitions to the mir world. mir owns the full surface; chrome overlays it. mir is not part of the cell grid — it is the model-a escalation for spatial content.

```
cyb://graph/ai/safety
  → bevy: transition world state to Graph
  → mir: activate, focus camera on the graph location indicated by the path
  → chrome: keep address bar and commander visible as overlay
  → no other world is rendering
```

mir provides the cinematic "wow factor" without polluting the cell-grid model that handles everything else. when the [[neuron]] is done navigating, they return to a sugarloaf-hosted world via the address bar or a hotkey.

mir is also reachable from within sugarloaf via an osc that requests a spatial preview region — a future extension for embedding small graph views inline with text.

---

## wry — the open-web escalation

[[wry]] hosts arbitrary external web content. it is wgpu's opposite — an os-provided webview that cannot composite into bevy's surface. cyb treats wry as a model-a escalation: when active, wry owns the full surface (as a child window or fullscreen overlay), bevy pauses cell-grid rendering for that surface region, chrome continues overlaying on top.

uses for wry

- viewing third-party websites
- pdf documents (os webview has a pdf renderer)
- video playback when the source is web-hosted
- the [[agent]]'s headless browser instance (off-screen, separate from ui)
- legacy webapps that cannot be ported

wry is a sandbox-for-the-external-web. it is not the chrome. it is not a daily-driver world. it is the escape hatch when content does not fit the cell-grid model and cannot be served by mir or media.

---

## media — symphonia and decoders

audio playback uses [[symphonia]] for decode and [[rodio]] for output. when the address resolves to `cyb://media/<particle>`, the media world activates with a player ui (also cells in chrome) and a continuous-pixel waveform/video surface below.

video uses native rust decoders where available (ffmpeg-next, av1-rs, or a vendored video crate) or falls back to wry for codecs that are too costly to implement.

---

## the unified wgpu pipeline

all five surfaces share the same wgpu device and queue, owned by bevy

```
bevy
  RenderDevice + RenderQueue (shared)
    ↓
  ┌── sugarloaf (cell grid)   ─┐
  ├── mir (3d)                  │
  ├── wry (excluded — os pipe)  ├── all use the same wgpu device
  ├── media (continuous)        │   no context switching
  └── chrome overlay (cells)   ─┘   one glyph atlas across cell renderers
```

bevy's `GpuBridgePlugin` exposes the wgpu device as an ecs resource. cell-grid renderers (sugarloaf for worlds, chrome for overlay) share a single glyph atlas — fonts are cached once. mir and media each maintain their own gpu pipelines but share the same device.

wry is the exception. it renders into an os-provided surface that cannot share resources with wgpu. this is the architectural cost of supporting the open web; it is paid only in the wry world, not everywhere.

---

## composition rules

### exclusivity

at most one world is active. switching worlds is an ecs state transition. the previous world's render pass is removed from the graph; the new world's render pass is added. no overlap, no z-order ambiguity, no transparent layers fighting over which pixels show through.

### chrome always renders last

chrome's render pass is unconditional and runs after the active world's pass. nothing can occlude chrome. the address bar and commander remain visible regardless of which world is showing.

### input routing

bevy's input system routes events based on hit-test:

1. if the event hits a chrome cell → chrome handles it (typing in address bar, in commander, etc.)
2. else, forward to the active world's input handler

this is a clean two-layer model. no ambiguity. cell-grid hit-testing is integer cell math, trivial.

### transitions

world transitions can fade or crossfade (a simple opacity ramp across two render passes during a transition window of ~150ms). chrome stays solid throughout — only the world surface fades. this gives cinematic feel without compositing complexity.

---

## platforms

| platform | window | sugarloaf | mir | wry | media | chrome |
|---|---|---|---|---|---|---|
| macos desktop | bevy / winit | ✓ wgpu (metal) | ✓ wgpu (metal) | wkwebview | ✓ | ✓ |
| android | bevy / winit | ✓ wgpu (vulkan) | ✓ wgpu (vulkan) | android webview | ✓ | ✓ |
| linux desktop | bevy / winit | ✓ wgpu (vulkan) | ✓ wgpu (vulkan) | webkitgtk | ✓ | ✓ |
| windows | bevy / winit | ✓ wgpu (d3d12) | ✓ wgpu (d3d12) | webview2 | ✓ | ✓ |
| any terminal (over ssh) | — | rendered as ansi to stdout | not available | not available | not available | rendered as ansi |

terminal-only operation is a first-class deploy target. `cyb` running over ssh in a remote shell renders cells as ansi escapes to stdout. mir and wry are not available in that mode; sugarloaf-hosted worlds work fully. this is the same code path used inside the cyb window — only the backend pass differs (gpu cells vs ansi stdout).

---

## prysm contract

[[prysm]] is the composition protocol for cell-grid content. it specifies how atoms (glass, text, ion, saber, images) combine into molecules and cells. it does not specify how cells are painted to pixels — that is the renderer's job.

| layer | role |
|---|---|
| prysm core | layout function: element tree × viewport → cell coordinates |
| prysm atoms | the nine primitives (glass, text, ion, saber, image, sound, number, neuron, field) |
| prysm molecules | 24 composed widgets (commander, input, button, neuron-card, etc.) |
| prysm cells | full-screen applications (oracle, brain, sense, sigma, etc.) |

prysm is renderer-blind. the same prysm tree can rasterize to wgpu cells (sugarloaf), to ansi escapes (terminal stdout), to html (seo export), or to any future cell-grid backend. the layout is pure: tree × viewport → coordinates, deterministic, o(n).

---

## what disappears in this architecture

problems that consumed prior iterations and are now structurally absent

- the always-on transparent webview that fought bevy's wgpu surface
- coordinate-system divergence between wry (logical pixels) and wgpu (physical pixels)
- click-through routing via css `pointer-events` and pixel-transparency hacks
- z-order battles between chrome (webview) and worlds (wgpu)
- compositing different rendering models in the same surface
- scale-factor bugs across windowed and fullscreen modes
- ipc-vs-eval-script choices for cross-layer navigation
- duplicate ui implementations per world for chrome elements

these are not solved — they do not exist. the architecture removes the conditions that produce them.

---

see [[routing]] for how addresses dispatch worlds and drive navigation, [[prysm]] for the composition protocol and atom/molecule/cell system, [[mir]] for the spatial graph renderer, [[sugarloaf]] for the cell-grid wgpu backend, [[nu]] for the nushell integration, [[radio]] for particle resolution behind osc references
