---
tags: cyb, core, architecture
crystal-type: pattern
crystal-domain: cyber
---

# rendering

the rendering stack of [[cyb]]. explains what owns what, why each layer exists, and how they compose.

## why this stack

cyb is a knowledge interface. it renders text, images, video, data tables, search results, transaction feeds, and 3D graph visualizations. these are fundamentally different rendering problems:

- text, video, inputs, tables — the browser engine is the best renderer ever built for this. it handles hardware-accelerated video decode, mobile IME, copy/paste, accessibility, CSS layout, web fonts. reimplementing any of this is waste.
- 3D graph visualization — a real-time GPU problem. the browser's WebGL is adequate but the cybergraph renderer ([[mir]]) is a native Bevy plugin. it needs ECS, direct wgpu access, and a GPU buffer shared with the rest of the engine.
- desktop shell — global hotkeys, tray icon, native window chrome, multi-world state machine. these require native code.

the stack is layered to give each problem its ideal tool without compromise.

## layers

```
Bevy (desktop shell)
  owns: native window, GPU device, worlds state machine, hotkeys, tray
  ↓
wry (WebView host)
  owns: WebView embedded in Bevy's window, URL, navigation events
  ↓
Leptos WASM (web UI)
  owns: all [[prysm]] components, all aips, DOM rendering
```

## ownership

**Bevy owns the window.** Bevy creates the native window via winit. It controls the window's lifetime, size, and position. It owns the wgpu device and render queue — the hardware GPU handle.

**wry owns the WebView.** wry borrows Bevy's window handle and embeds a WebView inside it (WKWebView on macOS, Android System WebView on Android). The WebView is transparent — Bevy's black background shows through by default, creating the glass-on-void effect of [[prysm]] without any special treatment.

**wry owns the URL.** The URL lives in the WebView's navigation state. It is the single source of truth for where the user is. Neither Bevy nor Leptos owns it — both subscribe to it.

## surfaces

two kinds of surface exist:

**WebView surface** — the default. Leptos renders [[prysm]] components into the DOM. Glass, depth, emotion, typography, all aips — all here. Web-native: video, inputs, text selection, accessibility, copy/paste all work because WKWebView is a real browser engine.

**native surface** — the 3D graph world. [[mir]] is a Bevy plugin that renders directly to wgpu. when the user navigates to `/brain`, Bevy activates the mir graph alongside the WebView. the graph canvas occupies its rectangle of the screen. Leptos renders the chrome around it (context bar, commander, glass panes) in the WebView layer above.

## routing

the URL is the single address space. there are no routing levels, no sync, no IPC for navigation.

```
URL changes  →  wry fires navigation event  →  Bevy reads URL, shows right surface
             →  Leptos reads window.location, renders right aip
```

both Bevy and Leptos react to the URL independently. they do not coordinate.

**writing the URL:**

- hotkey fires → `webview.load_url("cyb://brain")` — Bevy writes via wry
- user taps link → `history.pushState()` — Leptos writes via browser API
- tray item selected → `webview.load_url(...)` — Bevy writes via wry

no IPC for routing. the WebView's URL is the message bus.

**in the browser (no Bevy):** Leptos router handles everything alone. same URLs, same components, same behavior. Bevy's absence changes nothing for Leptos — it always read `window.location`, never asked Bevy for the route.

## address space

```
/oracle          search and knowledge exploration
/oracle/:cid     single particle view
/brain           3D cybergraph (mir surface activated)
/sense           messages
/sigma           wallet and balances
/portal          onboarding
/sphere          heroes and staking
/senate          governance
/teleport        token transfers
/terminal        nushell (native surface)
```

Bevy maps these to surfaces. Leptos maps these to aips. The URL is the contract between them.

## platforms

| platform | window | WebView | Leptos | native surfaces |
|---|---|---|---|---|
| macOS | Bevy/winit | WKWebView | ✓ WASM | graph (wgpu), terminal |
| web browser | — | the browser itself | ✓ WASM | graph (WebGL via mir) |
| Android | Bevy/winit | Android System WebView | ✓ WASM | graph (wgpu) |

the Leptos WASM bundle is identical across all three. the shell (Bevy) is platform-specific. the web UI (Leptos) is not.

## dependency graph

```
winit (window events)
  ↓
Bevy (ECS, wgpu, worlds)
  ↓
wry (WebView, URL, IPC)
  ↓
Leptos WASM (prysm components, aips)
  ↓
WebKit / Chromium (platform browser engine — not shipped, OS-provided)
```

nothing is shipped except the Rust binary and the Leptos WASM bundle. the browser engine comes from the OS. no Electron, no Node.js, no bundled Chromium.

## glass and depth

[[prysm/glass]] is CSS. `backdrop-filter: blur()` and `rgba()` opacity. z-index is CSS z-index. emotion is CSS color variables. none of this needs a canvas. the browser composites glass panes natively using the GPU, the same as it composites any semi-transparent DOM element.

the wry WebView is transparent. Bevy renders black (`ClearColor::BLACK`) behind it. the glass panes of prysm float over void by default, with zero special handling.

## 3D and Leptos

the graph world is the one case where Bevy renders something the user sees directly (not behind a WebView). the pattern:

```
/brain navigation:
  1. Bevy activates mir GraphWorldPlugin — wgpu renders graph to native surface
  2. Leptos renders glass chrome (context bar, commander) in WebView above
  3. the WebView is transparent where there are no DOM elements
  4. user sees: graph rendered by Bevy through the transparent WebView chrome
```

Leptos does not know about Bevy's graph. Bevy does not know about Leptos's chrome. both react to the URL independently and happen to compose correctly because the WebView is transparent.

## terminal surface: sugarloaf + nushell

the terminal world is a third kind of surface — not WebView, not a live wgpu scene. it is a GPU-rendered texture composited as a Bevy sprite.

**the pipeline:**

```
nushell (shell interpreter, vendored)
  ↓  executes commands, produces output
alacritty_terminal (VT100/ANSI parser, vendored)
  ↓  parses escape sequences, maintains terminal grid state
sugarloaf (Rio terminal renderer, vendored)
  ↓  renders terminal grid to an offscreen wgpu texture
     GPU-quality text: subpixel rendering, ligatures, ANSI colors
Bevy Image asset
  ↓  sugarloaf's offscreen texture is read back to CPU pixels
     uploaded into a Bevy Image (Bgra8UnormSrgb)
Bevy sprite / quad
  ↓  Bevy renders the Image as a mesh in the scene
displayed in the Bevy window
```

**the GPU sharing trick.** sugarloaf does not create its own GPU context. it runs on **Bevy's wgpu device and queue**. `GpuBridgePlugin` extracts Bevy's `RenderDevice` and `RenderQueue` into the main world at startup. when the terminal world initializes, it passes those handles to sugarloaf:

```rust
let device = world.resource::<RenderDevice>().wgpu_device().clone();
let queue  = world.resource::<RenderQueue>()...clone();
let ctx    = SugarloafContext::new_external(device, queue, ...);
```

one GPU context, shared. sugarloaf renders offscreen into a texture on Bevy's device, reads the pixels back, and uploads them to a Bevy Image. no second GPU stack, no context switching overhead.

**why this approach and not a WebView terminal:**

a WebView terminal (xterm.js style) would work but adds JS, a DOM terminal emulator, and loses native font rendering. sugarloaf is the Rio terminal engine — production-quality GPU text rendering designed for exactly this use case, written in Rust, runs on the same wgpu context as the rest of cyb.

**nushell** is the shell. not bash, not zsh. nu gives the terminal structured output (tables, records, lists) rather than raw text streams. nu scripts are used in the cyb build system (`nu/`) and in the agent scripting layer. the terminal world embeds the full nu engine — `nu_cli`, `nu_command`, `nu_protocol`, `nu_engine` — as native Rust crates.

**prysm and the terminal.** the terminal content (the text grid rendered by sugarloaf) does not obey prysm layout rules — it is a terminal, it has its own grid. the **chrome around it** (the glass pane border, context bar, commander) is rendered by Leptos in the WebView layer above, and those elements do obey prysm. the terminal is a raw wgpu texture inside a prysm glass container.

## surfaces summary

three kinds of surface exist in cyb:

| surface | renderer | what it shows | prysm |
|---|---|---|---|
| WebView | WKWebView / Chromium | all prysm UI, all aips | full contract |
| native wgpu scene | Bevy + mir | 3D cybergraph | chrome only (WebView layer) |
| offscreen texture | sugarloaf → Bevy Image | nushell terminal | chrome only (WebView layer) |

the WebView is transparent. Bevy renders behind it. native surfaces (graph, terminal) are Bevy content that shows through the transparent WebView where there are no DOM elements.

## prysm contract

[[prysm]] is the law for all rendering in cyb. every visual element — whether rendered by Leptos in the DOM or by Bevy in a native surface — obeys prysm's three rules: ECS structure, fold conformations, and layout algebra. the rendering layer is an implementation detail. the prysm contract is not.

### ECS

prysm defines components (organelles) and systems, not inheritance hierarchies. in Leptos this is an architectural pattern, not a runtime:

| prysm ECS | Leptos implementation |
|---|---|
| entity (organelle) | component function |
| ECS component (`Sizing`, `GlassDepth`, `Emotion`) | typed prop |
| system (`ConstrainSystem`, `EmotionSystem`) | reactive effect / memo |
| component tree | Leptos view tree |

a `<Glass>` component in Leptos takes `depth: GlassDepth` and `emotion: ReadSignal<Emotion>` as typed props. these are prysm's ECS components expressed as Rust types. the system that computes emotion from cyberank state is a Leptos effect that writes to those signals.

in Bevy, ECS is native — prysm components are actual Bevy components (`GlassDepth`, `Emotion`, `Sizing`) and prysm systems are actual Bevy systems. no translation needed.

the same prysm vocabulary (`Sizing::Fix(n)`, `GlassDepth::Midground`, `Emotion::Joy`) is used in both layers. the types are shared in a common crate. Leptos and Bevy both depend on it.

### fold

every component declares its fold conformations — the set of conformations it can collapse to at different widths. the active conformation is selected by container width, not viewport width.

```
component declares:
  l₁  (w ≥ 40g)  full layout
  l₂  (w ≥ 20g)  compact
  l₃  (w ≥ 10g)  minimal

runtime selects:
  ResizeObserver watches the component's container
  signal carries current width in g units
  memo derives active conformation
  component renders the matching layout
```

in Leptos: `use_fold(breakpoints)` returns a `ReadSignal<Conformation>`. the component switches its `view!` branch on it. CSS container queries handle the simple cases declaratively; the signal handles cases where Rust logic is needed.

in Bevy: `FoldSet` is a component. `FoldSystem` reads the organelle's `OccupiedSize`, writes `ActiveConformation`. the rendering system reads `ActiveConformation` and picks the right layout.

### layout

prysm's layout algebra — constrain → occupy → place — maps directly to CSS Flexbox/Grid. the browser already implements these properties. the job is to encode prysm's sizing types as CSS, not to re-implement layout:

| prysm | CSS |
|---|---|
| `g = 8px` | `--g: 8px` CSS variable |
| `Sizing::Fix(n)` | `width: calc(n * var(--g))` |
| `Sizing::Fill` | `flex: 1` / `width: 100%` |
| `Sizing::Scale(r)` | `flex-grow: r` |
| stack horizontal | `display: flex; flex-direction: row; gap: var(--g)` |
| stack vertical | `display: flex; flex-direction: column; gap: var(--g)` |
| grid zones | `display: grid; grid-template-areas: ...` |
| `padding: g` | `padding: var(--g)` |

every Leptos component in prysm takes a `sizing: Sizing` prop and translates it to the corresponding CSS at render time. the layout algebra's theorems (T1–T14) hold because CSS Flexbox/Grid implements exactly these properties — the proofs describe what the browser does, not a custom engine.

in Bevy: `bevy_ui` with `taffy` handles layout. `Sizing::Fix(n)` maps to `Val::Px(n * G)`. `Sizing::Fill` maps to `Val::Percent(100.0)`. the same types, the same mapping, different backend.

### emotion

emotion is an ambient signal — it flows down the component tree without being passed explicitly as a prop to every child. it is computed from cyberank and context state, not assigned manually.

```
bbg / cybergraph state
  ↓
EmotionContext (Leptos context provider at app root)
  provides: ReadSignal<Emotion>
  ↓
any component calls use_emotion() → gets the current emotion signal
  ↓
glass tint, saber glow, text color all derive from it
```

the tri-kernel computes the emotion value from: cyberank of visible content, karma of active neuron, transaction state, message state. the result is one of nine values from the [[prysm/palette]]. components consume it without knowing where it came from.

in Bevy: `Emotion` is a Bevy resource (global) or component (per-entity). `EmotionSystem` writes to it. rendering systems read it.

### the shared prysm crate

both Leptos and Bevy depend on one crate that defines the prysm vocabulary:

```
prysm/
├── types.rs      Sizing, GlassDepth, Emotion, FoldSet, Conformation, Urgency
├── palette.rs    the 9 emotion values and all color tokens as constants
├── grid.rs       G constant, grid unit math
└── layout.rs     Sizing → CSS string / Sizing → taffy Val conversions
```

this crate has no rendering dependency. it is pure types and constants. Leptos uses `Sizing → CSS`. Bevy uses `Sizing → taffy Val`. the prysm contract is enforced at the type level — if a component takes `depth: GlassDepth`, it cannot receive an arbitrary opacity value.

### summary

```
prysm rule      Leptos                    Bevy
────────────    ──────────────────────    ──────────────────────
ECS             architectural pattern     native ECS runtime
fold            use_fold() signal         FoldSystem + FoldSet
layout          Sizing → CSS props        Sizing → taffy Val
emotion         use_emotion() context     Emotion resource/component
glass/depth     CSS backdrop-filter       bevy_ui material
grid (g=8)      --g CSS variable          G constant in Val::Px
```

the rendering layer changes. the prysm contract does not.

see [[prysm/layout]] for the component model, [[mir]] for the graph renderer, [[prysm]] for the design system
