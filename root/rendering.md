---
tags: cyb, core, architecture
crystal-type: pattern
crystal-domain: cyber
---

# cyb rendering and navigation

one shell, three surfaces, one history.

---

## the insight

cyb renders three fundamentally different kinds of content — DOM (text, video, forms), 3D graph (wgpu), and terminal (GPU text). each needs its ideal tool. the problem: the user doesn't care. they want to navigate freely across all of them, press back, press a hotkey, and have it all feel like one app.

the solution has two parts that reinforce each other:

**1. the WebView never turns off.** it is the permanent topmost layer, always transparent where there is no DOM content. the Leptos shell — top bar, bottom bar, navigation — is always mounted. for aips with a native surface beneath, the content area is a transparent pane. mir and sugarloaf render behind the hole. the chrome is identical in all modes because it is always the same component.

**2. `history.pushState()` is the only navigation mechanism.** Bevy never calls `webview.load_url()` for routing — that is a full page reload that destroys Leptos state and fragments history. for hotkeys, Bevy calls `evaluate_script(pushState + popState)` directly and activates the surface immediately. for links, Leptos calls `use_navigate()`. either way, one write path, one history. back and forward work everywhere, across aip switches and within-aip navigation alike.

```
all modes — what the user sees:

  ┌─────────────────────────────────────┐
  │  TopBar  (Leptos, always mounted)   │  ← location, commander, back/forward
  ├─────────────────────────────────────┤
  │                                     │
  │  /oracle  → DOM content (Leptos)    │
  │  /brain   → transparent pane        │  ← mir wgpu renders behind this
  │  /terminal→ transparent pane        │  ← sugarloaf renders behind this
  │  /sense   → DOM content (Leptos)    │
  │  ...                                │
  │                                     │
  ├─────────────────────────────────────┤
  │  BottomBar (Leptos, always mounted) │  ← aip switcher, status
  └─────────────────────────────────────┘
```

---

## the stack

```
Bevy
  owns: native window (winit), wgpu device + queue,
        hotkeys, tray, GPU bridge, surface activation
  ↓
  ├── wry → WebView (always on, always topmost, transparent)
  │         → Leptos WASM → WKWebView / Android WebView
  │           AppShell (persistent) + aip content
  │
  ├── mir → wgpu native surface
  │         3D cybergraph — active at /brain
  │         paints behind the transparent WebView
  │
  └── sugarloaf → offscreen wgpu texture → Bevy Image → Bevy sprite
                  nushell terminal — active at /terminal
                  composited behind the transparent WebView
```

**Bevy owns the window.** it creates the native window via winit and owns the wgpu device and render queue. `GpuBridgePlugin` exposes these as Bevy resources so sugarloaf can share the same GPU context without creating a second one.

**wry embeds the WebView.** wry borrows Bevy's window handle and embeds a transparent WebView. Bevy clears to black behind it — [[prysm]]'s glass-on-void effect costs nothing.

**the WebView is always active.** there is no mode in which the WebView is hidden or replaced. when a native surface is active, the WebView remains on top with only its content area transparent.

---

## the shell

the Leptos app root is a persistent shell that never unmounts. it is the same component regardless of which aip or surface is active:

```rust
<AppShell>
    <TopBar />           // location, commander, back/forward — always
    <ContentArea>        // switches on route
        <Route path="/oracle/*"> <OracleAip />       </Route>
        <Route path="/brain/*">  <TransparentPane /> </Route>
        <Route path="/terminal"> <TransparentPane /> </Route>
        <Route path="/sense/*">  <SenseAip />        </Route>
        <Route path="/sigma/*">  <SigmaAip />        </Route>
        // ...
    </ContentArea>
    <BottomBar />        // aip switcher, status — always
</AppShell>
```

`TransparentPane` is a `div` with `background: transparent; width: 100%; height: 100%`. no special rendering — just a hole. the native surface behind it is activated by Bevy when the URL changes.

because the shell never unmounts, Leptos signals and component state are preserved across aip switches. navigating from `/oracle/QmXxx` to `/brain` and back restores oracle's state exactly — no reload, no signal reset.

---

## three surfaces

### 1. WebView — all aips

the default. Leptos renders [[prysm]] components into the DOM. the full content area is DOM.

```
Leptos WASM → HTML + CSS → WKWebView / Android WebView
  video, inputs, text selection, copy/paste, accessibility, fonts — OS-native
  glass depth: backdrop-filter: blur() + rgba() opacity
  z-index: CSS z-index
  emotion: CSS color variables
  none of this needs a canvas — the OS engine composites on the GPU natively
```

### 2. native wgpu — graph (/brain)

[[mir]] is a Bevy plugin. when the URL changes to `/brain`, Bevy activates `GraphWorldPlugin`. mir renders the 3D cybergraph directly to wgpu — the same device Bevy owns — behind the transparent WebView.

```
/brain:
  Bevy activates mir → wgpu renders 3D cybergraph to native surface (behind WebView)
  Leptos renders TransparentPane in ContentArea → graph visible through it
  TopBar and BottomBar remain in WebView above — same chrome as every other aip
```

Leptos does not know about mir. mir does not know about Leptos. they compose correctly because the WebView is transparent.

### 3. offscreen texture — terminal (/terminal)

sugarloaf renders nushell output to a GPU texture, which Bevy composites as a sprite behind the transparent WebView.

```
nushell (vendored nu_* crates)
  shell interpreter — structured output (tables, records)
  ↓
alacritty_terminal (vendored)
  VT100/ANSI parser — terminal grid state, escape sequences
  ↓
sugarloaf (vendored Rio renderer)
  renders terminal grid to offscreen wgpu texture
  subpixel rendering, ligatures, ANSI colors, cursor
  uses Bevy's wgpu device + queue — no separate GPU context
  ↓
CPU readback → Bevy Image (Bgra8UnormSrgb) → Bevy sprite
  composited behind the transparent WebView
```

**GPU sharing.** `GpuBridgePlugin` exposes Bevy's device and queue. sugarloaf receives them at init:

```rust
let device = world.resource::<RenderDevice>().wgpu_device().clone();
let ctx    = SugarloafContext::new_external(device, queue, ...);
```

one GPU, no context switching. the terminal has the same chrome as every other aip — TopBar and BottomBar always present, always Leptos.

### surfaces compared

| surface | renderer | content area | chrome |
|---|---|---|---|
| WebView | WKWebView / Android WebView | DOM — aip content | same shell |
| native wgpu | Bevy + mir | 3D cybergraph | same shell |
| offscreen texture | sugarloaf → Bevy sprite | nushell terminal | same shell |

---

## navigation

### the rule

**`history.pushState()` is the only navigation mechanism.** Bevy never calls `webview.load_url()` for routing — that is a full page reload, destroys Leptos state, and does not create a history entry.

### flows

**link / button / programmatic:**
```
Leptos: use_navigate() or <A href=...>
  → history.pushState(new URL)
  → popState fires → Leptos router re-renders ContentArea
  → wry fires navigation event → Bevy reacts: activates right surface
```

**hotkey / tray:**
```
user presses Cmd+2
  → Bevy fires hotkey event
  → Bevy calls webview.evaluate_script:
      history.pushState(null, '', '/brain');
      window.dispatchEvent(new PopStateEvent('popstate', { state: history.state }));
  → Bevy activates mir immediately (already knows target URL)
  → popState fires in WebView → Leptos router re-renders ContentArea
```

Bevy writes the URL directly via `evaluate_script` — no IPC handler, no Leptos-side registration, no startup race. dispatching `popstate` is exactly what `history.back()` does — Leptos's router already listens to it natively. Bevy activates the surface in the same tick without waiting for a navigation event.

**back / forward:**
```
user presses back
  → history.back() → popState fires
  → Leptos router reacts: ContentArea renders previous aip
  → wry fires navigation event → Bevy reacts: activates surface for previous URL
```

back/forward work across the full history — hotkey surface switches and link taps alike — because every navigation went through pushState.

### ownership

| concern | owner | mechanism |
|---|---|---|
| navigation history | browser history API | pushState / popState |
| writing the URL | Leptos (links) or Bevy (hotkeys) | `use_navigate()` / `evaluate_script(pushState)` |
| surface activation | Bevy | hotkey: immediate; link: wry navigation event |
| aip + sub-page render | Leptos | reacts to popState via router |
| hotkey dispatch | Bevy | `evaluate_script` — no handler registration needed |
| chrome (top/bottom bar) | Leptos shell | always mounted, never re-rendered |

### address space

```
/oracle            search and knowledge
/oracle/:cid       single particle view
/brain             3D cybergraph          → activates mir
/brain/:cid        graph centered on particle
/sense             personal feed
/sense/:neuron     neuron's messages
/sigma             wallet
/portal            onboarding
/sphere            heroes and staking
/senate            governance
/teleport          token swaps
/hfr               supercomputing resources
/terminal          nushell shell          → activates sugarloaf
```

Bevy reacts to the first segment (surface selection). Leptos handles the full path (aip + sub-page). neither layer needs to know about the other's domain.

---

## prysm contract

[[prysm]] is the law for all visual elements in cyb — regardless of which layer renders them. the rendering layer is an implementation detail. the contract is not.

### shared types

one crate, no rendering dependency:

```
prysm/
├── types.rs    Sizing, GlassDepth, Emotion, FoldSet, Conformation, Urgency
├── palette.rs  9 emotion values, all color tokens
├── grid.rs     G = 8px constant, grid unit math
└── layout.rs   Sizing → CSS string (Leptos), Sizing → taffy Val (Bevy)
```

a component that takes `depth: GlassDepth` cannot receive an arbitrary opacity float. the contract is enforced at the type level.

### ECS, layout, fold, emotion

| prysm rule | Leptos | Bevy |
|---|---|---|
| ECS | typed props + reactive effects | native ECS |
| layout (`Sizing`) | CSS Flexbox/Grid, `--g: 8px` | taffy, `const G: f32 = 8.0` |
| fold | `use_fold()` + ResizeObserver signal | `FoldSystem` + `ActiveConformation` |
| emotion | `use_emotion()` context provider | `Emotion` resource / component |
| glass / depth | `backdrop-filter` + `rgba()` | `bevy_ui` material |

emotion is ambient — computed from cyberank and context state, not assigned manually. it flows down the component tree (Leptos context, Bevy resource) and drives glass tint, saber glow, and text color.

---

## platforms

| platform | window | WebView | Leptos WASM | native surfaces |
|---|---|---|---|---|
| macOS desktop | Bevy / winit | WKWebView | ✓ via Trunk | mir (wgpu), sugarloaf |
| web browser | — | the browser itself | ✓ served statically | mir (WebGL) |
| Android | Bevy / winit | Android System WebView | ✓ via Trunk | mir (wgpu), sugarloaf |

the Leptos WASM bundle is identical across all targets. on web, Bevy is absent — Leptos handles routing and surface activation alone (mir falls back to WebGL canvas, terminal renders as a WebView fallback). the history API is the same everywhere.

nothing is shipped except the Rust binary and the WASM bundle. the browser engine is OS-provided. no Electron, no Node.js, no bundled Chromium.

---

see [[routing]] for the full navigation mechanism and flows, [[prysm]] for the design system and component model, [[mir]] for the graph renderer, [[prysm/layout]] for the layout algebra, [[nu]] for the nushell integration
