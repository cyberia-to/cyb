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

see [[prysm/layout]] for the component model, [[mir]] for the graph renderer, [[prysm]] for the design system
