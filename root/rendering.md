---
tags: cyb, core, architecture
crystal-type: pattern
crystal-domain: cyber
---

# rendering

how [[cyb]] turns data into pixels. three surfaces, one prysm contract, one address space.

---

## the problem

cyb renders fundamentally different kinds of content:

- text, video, images, tables, forms, search results — the browser engine solves this better than anything else. it handles hardware-accelerated video decode, mobile IME, copy/paste, accessibility, fonts, CSS layout. reimplementing any of it is waste.
- 3D graph visualization — a real-time GPU problem. [[mir]] is a Bevy plugin with direct wgpu access and ECS-native scene management.
- terminal — GPU-quality text rendering with a live shell. sugarloaf renders nushell output at native quality using the same wgpu device as everything else.
- desktop shell — global hotkeys, tray, native window chrome, multi-world state machine. requires native code.

the stack gives each problem its ideal tool. no layer compromises for another.

---

## stack and ownership

```
Bevy
  owns: native window (winit), wgpu device + queue, worlds state machine,
        hotkeys, tray, GPU bridge
  ↓
  ├── wry → WebView → Leptos WASM → WKWebView / Android WebView (OS-provided)
  │         URL, navigation events   all prysm UI, all aips
  │
  ├── mir → wgpu native surface
  │         3D cybergraph (/brain)
  │
  └── sugarloaf → offscreen wgpu texture → Bevy Image → Bevy sprite
                  nushell terminal (/terminal)
```

**Bevy owns the window.** Bevy creates the native window via winit. it owns the wgpu device and render queue — the hardware GPU handle. `GpuBridgePlugin` extracts these at startup and makes them available as Bevy resources so other systems (sugarloaf) can share the same GPU context without creating a second one.

**wry owns the WebView.** wry borrows Bevy's window handle and embeds a WebView into it. the WebView is transparent — Bevy renders black (`ClearColor::BLACK`) behind it. [[prysm]]'s glass-on-void effect costs nothing: the WebView is already transparent.

**wry owns the URL.** the URL lives in the WebView's navigation state. it is the single source of truth for where the user is. neither Bevy nor Leptos owns it — both subscribe to it.

---

## three surfaces

### 1. WebView surface

the default surface. Leptos renders [[prysm]] components into the DOM. all aips live here.

```
Leptos WASM
  renders: prysm atoms, molecules, cells, aips
  via: HTML + CSS in WKWebView / Android WebView
  native: video, inputs, text selection, copy/paste, accessibility, fonts
```

glass depth is `backdrop-filter: blur()` + `rgba()` opacity. z-index is CSS z-index. emotion is CSS color variables. none of this needs a canvas — the OS browser engine composites semi-transparent DOM elements on the GPU natively.

### 2. native wgpu surface (graph)

[[mir]] is a Bevy plugin. when the user navigates to `/brain`, Bevy activates the mir `GraphWorldPlugin`. mir renders directly to wgpu — the same device Bevy owns. it paints behind the transparent WebView.

```
/brain navigation:
  Bevy activates mir → wgpu renders 3D cybergraph to native surface
  Leptos renders glass chrome (context bar, commander) in WebView above
  WebView is transparent where there are no DOM elements
  result: graph visible through the glass chrome
```

Leptos does not know about mir. mir does not know about Leptos. both react to the URL independently and compose correctly because the WebView is transparent.

### 3. offscreen texture surface (terminal)

the terminal world is a GPU-rendered texture composited as a Bevy sprite. it does not use a WebView and does not render to the live wgpu scene directly.

**pipeline:**

```
nushell (vendored nu_* crates)
  shell interpreter — executes commands, produces structured output
  ↓
alacritty_terminal (vendored)
  VT100/ANSI parser — maintains terminal grid state, handles escape sequences
  ↓
sugarloaf (vendored Rio renderer)
  renders terminal grid to an offscreen wgpu texture
  GPU-quality text: subpixel rendering, ligatures, ANSI colors, cursor
  runs on Bevy's wgpu device + queue — no separate GPU context
  ↓
CPU readback
  sugarloaf reads offscreen pixels back to CPU
  ↓
Bevy Image asset (Bgra8UnormSrgb)
  pixels uploaded into a Bevy-managed texture
  ↓
Bevy sprite / quad
  Bevy renders the Image as a mesh in the scene — visible through the WebView
```

**GPU sharing.** sugarloaf does not spin up its own GPU context. `GpuBridgePlugin` exposes Bevy's device and queue in the main world. the terminal world passes them directly to sugarloaf at init:

```rust
let device = world.resource::<RenderDevice>().wgpu_device().clone();
let queue  = /* from RenderQueue */;
let ctx    = SugarloafContext::new_external(device, queue, ...);
```

one GPU. no context switching. sugarloaf renders offscreen, reads pixels back, Bevy uploads and draws.

**why not a WebView terminal (xterm.js).** a WebView terminal would add a JS emulator and lose native font rendering. sugarloaf is the Rio terminal engine — production-quality GPU text rendering written in Rust, sharing the same wgpu context as mir and the rest of the engine. nushell gives structured output (tables, records) rather than raw text streams.

### surfaces compared

| surface | renderer | content | prysm |
|---|---|---|---|
| WebView | WKWebView / Android WebView | all aips, all prysm UI | full contract |
| native wgpu | Bevy + mir (wgpu) | 3D cybergraph | chrome in WebView |
| offscreen texture | sugarloaf → Bevy Image | nushell terminal | chrome in WebView |

for surfaces 2 and 3: the content inside them follows their own rules (3D graph, terminal grid). the **chrome around them** — glass panes, context bar, commander — is always Leptos in the WebView layer above, following the full prysm contract.

---

## routing

the URL is the single address space. there are no routing levels, no IPC for navigation, no sync between Bevy and Leptos.

```
URL changes
  → wry fires navigation event → Bevy reads URL, activates right surface
  → Leptos reads window.location, renders right aip
```

both react independently to the same URL. they do not coordinate.

**writing the URL:**

| who | how | example |
|---|---|---|
| hotkey fires | Bevy calls `webview.load_url(...)` | Cmd+2 → `/brain` |
| user taps link | Leptos calls `history.pushState()` | tap oracle result → `/oracle/:cid` |
| tray item | Bevy calls `webview.load_url(...)` | tray → `/sense` |

**in the browser (no Bevy).** Leptos router handles everything alone. same URLs, same components. Bevy's absence changes nothing — Leptos always reads `window.location` and never asks Bevy for the route.

**address space:**

```
/oracle            search and knowledge exploration
/oracle/:cid       single particle view
/brain             3D cybergraph — activates mir surface
/sense             messages
/sigma             wallet and balances
/portal            onboarding and citizenship
/sphere            heroes and staking
/senate            governance
/teleport          token transfers and swaps
/terminal          nushell — activates sugarloaf surface
```

---

## prysm contract

[[prysm]] is the law for all rendering in cyb. every visual element obeys prysm's rules regardless of which layer renders it. the rendering layer is an implementation detail. the contract is not.

### shared types

both Leptos and Bevy depend on one crate that holds the prysm vocabulary:

```
prysm/
├── types.rs    Sizing, GlassDepth, Emotion, FoldSet, Conformation, Urgency
├── palette.rs  9 emotion values and all color tokens as constants
├── grid.rs     G = 8px constant, grid unit math
└── layout.rs   Sizing → CSS string (Leptos), Sizing → taffy Val (Bevy)
```

no rendering dependency. pure types and constants. the contract is enforced at the type level — a component that takes `depth: GlassDepth` cannot receive an arbitrary opacity float.

### ECS

prysm defines components and systems, not inheritance. in Leptos this is an architectural pattern:

| prysm ECS concept | Leptos | Bevy |
|---|---|---|
| entity (organelle) | component function | ECS entity |
| ECS component (`Sizing`, `GlassDepth`, `Emotion`) | typed prop | Bevy component |
| system (`ConstrainSystem`, `EmotionSystem`) | reactive effect / memo | Bevy system |

in Bevy, ECS is native — no translation needed. in Leptos, the same vocabulary (`Sizing::Fix(6)`, `GlassDepth::Midground`, `Emotion::Joy`) is expressed as typed Rust props. the types are shared; only the runtime model differs.

### layout

prysm's constrain → occupy → place maps directly to CSS Flexbox/Grid. the browser already implements these properties — the job is to encode prysm's sizing types as CSS, not to reimplement layout:

| prysm | Leptos (CSS) | Bevy (taffy) |
|---|---|---|
| `g = 8px` | `--g: 8px` | `const G: f32 = 8.0` |
| `Sizing::Fix(n)` | `width: calc(n * var(--g))` | `Val::Px(n * G)` |
| `Sizing::Fill` | `flex: 1` | `Val::Percent(100.0)` |
| `Sizing::Scale(r)` | `flex-grow: r` | `FlexGrow(r)` |
| stack horizontal | `display: flex; flex-direction: row` | `FlexDirection::Row` |
| stack vertical | `display: flex; flex-direction: column` | `FlexDirection::Column` |
| grid zones | `display: grid; grid-template-areas: ...` | `bevy_ui` grid |
| `padding: g` | `padding: var(--g)` | `UiRect::all(Val::Px(G))` |

### fold

every component declares conformations — layouts it can collapse to at different container widths. the active conformation is selected at runtime from the container's measured width.

```
component declares:  l₁ (w ≥ 40g)  full
                     l₂ (w ≥ 20g)  compact
                     l₃ (w ≥ 10g)  minimal

Leptos:  ResizeObserver → width signal → memo → active conformation
         use_fold(breakpoints) → ReadSignal<Conformation>

Bevy:    OccupiedSize component → FoldSystem reads it → writes ActiveConformation
         rendering system picks layout branch from ActiveConformation
```

### emotion

emotion is an ambient signal flowing down the component tree. it is computed from cyberank, karma, and context state — not assigned manually by components.

```
bbg / cybergraph state
  ↓
EmotionContext (Leptos context provider at app root)
  ↓
any component: use_emotion() → ReadSignal<Emotion>
  ↓
glass tint (15% color overlay), saber glow, text color all derive from it
```

in Bevy: `Emotion` is a Bevy resource (global) or component (per-entity). `EmotionSystem` computes and writes it.

### summary

| prysm rule | Leptos | Bevy |
|---|---|---|
| ECS | architectural pattern (typed props + effects) | native ECS |
| layout | `Sizing` → CSS Flexbox/Grid | `Sizing` → taffy |
| fold | `use_fold()` signal + ResizeObserver | `FoldSystem` + `ActiveConformation` |
| emotion | `use_emotion()` context provider | `Emotion` resource/component |
| glass/depth | CSS `backdrop-filter` + `rgba()` | `bevy_ui` material |
| grid (g=8) | `--g: 8px` CSS variable | `const G: f32 = 8.0` |

---

## platforms

| platform | window | WebView | Leptos WASM | native surfaces |
|---|---|---|---|---|
| macOS desktop | Bevy / winit | WKWebView | ✓ via Trunk | graph (wgpu), terminal (sugarloaf) |
| web browser | — | the browser itself | ✓ served statically | graph (WebGL via mir) |
| Android | Bevy / winit | Android System WebView | ✓ via Trunk | graph (wgpu), terminal (sugarloaf) |

the Leptos WASM bundle is identical across all three targets. the shell (Bevy + wry) is platform-specific. nothing is shipped except the Rust binary and the WASM bundle. the browser engine is OS-provided. no Electron, no Node.js, no bundled Chromium.

---

see [[prysm]] for the design system and component model, [[mir]] for the graph renderer, [[prysm/layout]] for the layout algebra, [[nu]] for the nushell integration
