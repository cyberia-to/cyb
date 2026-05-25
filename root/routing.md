---
tags: cyb, core, architecture
crystal-type: pattern
crystal-domain: cyber
---

# routing

how [[cyb]] navigates. one history, one write path, coherent across heterogeneous renderers.

---

## the problem

cyb has two subsystems that care about where the user is — Bevy (activates native surfaces) and Leptos (renders aips). naive solutions break:

| approach | what breaks |
|---|---|
| Bevy calls `webview.load_url()` | full page reload — destroys Leptos state, no history entry |
| Bevy sends IPC, Leptos handles it | startup race — handler may not be registered yet, message dropped |
| two-level ownership (Bevy owns level 1, Leptos level 2) | two histories, fragmented back/forward |

the user presses back and lands somewhere that makes no sense. or the app reloads. or nothing happens.

---

## the rule

**`history.pushState()` is the only navigation mechanism.**

`history.pushState()` writes the URL, adds a history entry, and does not reload the page. `popState` fires when the user goes back or forward — Leptos's router already listens to it natively. these two events are the complete navigation contract.

---

## the mechanism

### why evaluate_script + popState works without IPC

Leptos's router listens to the `popstate` event on `window`. this is how the browser's own back button works. there is no handler to register and no Leptos-side setup required — it is wired by the router at startup unconditionally.

when Bevy needs to navigate, it calls `webview.evaluate_script()` to run JavaScript directly in the WebView:

```javascript
history.pushState(null, '', '/brain');
window.dispatchEvent(new PopStateEvent('popstate', { state: history.state }));
```

line 1 writes the URL and adds a history entry. line 2 dispatches the same event that `history.back()` dispatches — Leptos router reads `window.location.pathname` and re-renders the correct aip. no IPC handler, no registration, no startup race.

### hotkey flow

```
user presses Cmd+2
  → Bevy fires hotkey event (GlobalHotKeyManager)
  → Bevy calls webview.evaluate_script(pushState('/brain') + dispatch popState)
  → Bevy activates mir surface immediately — it already knows the target URL
  → popState fires inside WebView
  → Leptos router reads window.location.pathname → '/brain'
  → ContentArea renders TransparentPane
  → graph visible through the transparent pane
```

Bevy does not wait for a navigation event. it activates the surface in the same tick. the ~1ms for the WebView to process the script is invisible to the user.

### link / button flow

```
user taps a link or calls use_navigate()
  → Leptos: history.pushState(new URL)
  → popState fires → Leptos router re-renders ContentArea
  → wry fires navigation event
  → Bevy reacts: reads first URL segment, activates right surface if changed
```

Bevy is purely reactive here. it does not need to act if the surface is already correct (e.g. navigating within /oracle — Bevy does nothing, Leptos re-renders the sub-page).

### back / forward flow

```
user presses back (gesture, keyboard, Leptos back button)
  → history.back() → popState fires
  → Leptos router reads window.location.pathname → renders previous aip
  → wry fires navigation event
  → Bevy reacts: activates surface for previous URL
```

identical mechanism to the hotkey flow — popState is the universal signal. the history is complete because every navigation wrote a pushState entry: hotkeys (via evaluate_script), links (via use_navigate), programmatic navigation (via use_navigate).

---

## ownership

| concern | owner | mechanism |
|---|---|---|
| navigation history | browser history API | pushState / popState |
| hotkey navigation | Bevy writes, Leptos reacts | `evaluate_script(pushState + popState)` |
| link navigation | Leptos writes, Bevy reacts | `use_navigate()` → wry navigation event |
| surface activation (hotkey) | Bevy | immediate — Bevy knows target URL |
| surface activation (link) | Bevy | reactive — wry navigation event |
| aip + sub-page render | Leptos | popState → router reads window.location |
| chrome (TopBar, BottomBar) | Leptos shell | always mounted, unaffected by navigation |

---

## why not webview.load_url()

`webview.load_url()` is a full navigation — equivalent to typing a URL in a browser address bar and pressing enter. it:

- reloads the entire Leptos WASM runtime from scratch
- destroys all reactive signals and component state
- does not push a history entry (or pushes one that causes a full reload on back)
- introduces a visible flash as the page reloads

`evaluate_script(pushState)` does none of these. the Leptos app stays alive, signals are preserved, history is intact, and the transition is instantaneous.

---

## address space

the URL is the shared state between Bevy and Leptos. first segment selects the aip and surface. everything after is Leptos's domain — Bevy does not parse sub-routes.

```
/oracle            search and knowledge      WebView
/oracle/:cid       single particle view      WebView
/brain             3D cybergraph             WebView + mir (wgpu)
/brain/:cid        graph centered on cid     WebView + mir (wgpu)
/sense             personal feed             WebView
/sense/:neuron     neuron's messages         WebView
/sigma             wallet overview           WebView
/portal            onboarding                WebView
/sphere            heroes and staking        WebView
/senate            governance                WebView
/teleport          token swaps               WebView
/hfr               supercomputing            WebView
/terminal          nushell shell             WebView + sugarloaf
```

---

## browser vs desktop

| | browser | desktop |
|---|---|---|
| history | browser history API | browser history API (same) |
| link navigation | Leptos `use_navigate()` | Leptos `use_navigate()` (same) |
| hotkey navigation | not applicable | Bevy `evaluate_script(pushState)` |
| surface activation | Leptos renders canvas / fallback | Bevy activates mir / sugarloaf |
| back / forward | browser UI | popState — same mechanism |

the navigation model is identical in both targets. on web, Bevy is absent and hotkeys are not applicable. Leptos handles everything alone with the same history API.

---

see [[rendering]] for the full stack, the persistent shell, and how surfaces compose with the WebView layer
