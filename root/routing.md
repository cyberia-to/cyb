---
tags: cyb, core, architecture
crystal-type: pattern
crystal-domain: cyber
---

# routing

how [[cyb]] navigates. one history, one write path, two reactive subscribers.

---

## the problem

cyb has two subsystems that care about where the user is — Bevy (activates surfaces) and Leptos (renders aips). if both write the URL, you get two histories and back/forward breaks.

specifically: `webview.load_url()` is a **full page reload**. it blows away Leptos state and does not push a history entry. `history.pushState()` is an **SPA navigation** — it preserves state, adds to history, and fires a popState event. mixing them fragments history from the user's perspective.

---

## the rule

**`history.pushState()` is the only navigation mechanism in cyb.**

Bevy never calls `webview.load_url()` for in-app routing. it only reacts.

---

## navigation flow

### user-initiated (link, button, programmatic)

```
Leptos calls use_navigate() / <A href=...>
  → history.pushState(new URL)
  → wry fires navigation event
  → Bevy reads URL → activates right surface (if changed)
  → Leptos router reads URL → renders right aip + sub-page
```

### hotkey / tray

```
user presses Cmd+2 (or tray item)
  → Bevy fires hotkey event
  → Bevy sends IPC message to WebView: { navigate: "/brain" }
  → Leptos receives IPC message → calls use_navigate()("/brain")
  → history.pushState("/brain")
  → wry fires navigation event
  → Bevy reacts: activates mir surface
  → Leptos reacts: renders brain aip
```

Bevy does not navigate directly. it delegates to Leptos via IPC and reacts to the result. the round-trip is ~1ms — invisible to the user.

### back / forward

```
user presses back (gesture, keyboard, button)
  → browser history.back() → popState fires in WebView
  → Leptos router reacts: renders previous aip + sub-page
  → wry fires navigation event
  → Bevy reacts: activates surface for previous URL
```

back/forward work for the full history — across aip switches (Cmd+hotkey) and within-aip navigation (link taps) — because every navigation went through pushState.

---

## ownership

| concern | owner | mechanism |
|---|---|---|
| navigation history | browser history API | pushState / popState |
| writing the URL | Leptos (only) | `use_navigate()` / `history.pushState()` |
| surface activation | Bevy | reacts to wry navigation events |
| aip + sub-page render | Leptos | reacts to URL via router |
| hotkey dispatch | Bevy | IPC message → Leptos navigates |

Bevy owns surface activation. Leptos owns URL writes. the history API owns history. nobody owns more than their domain.

---

## address space

the URL is the shared state. first segment = aip, rest = sub-page within that aip.

```
/oracle            search and knowledge
/oracle/:cid       single particle
/brain             3D cybergraph          → activates mir surface
/brain/:cid        graph centered on particle
/sense             personal feed
/sigma             wallet
/portal            onboarding
/sphere            heroes and staking
/senate            governance
/teleport          token swaps
/hfr               supercomputing resources
/terminal          nushell shell          → activates sugarloaf surface
```

Bevy only reacts to the first segment (surface selection). Leptos handles the full path (aip + sub-page). neither layer needs to know about the other's domain.

---

## browser vs desktop

| | browser | desktop |
|---|---|---|
| history | browser history API | browser history API (same) |
| navigation write path | Leptos `use_navigate()` | Leptos `use_navigate()` (same) |
| hotkey dispatch | not applicable | Bevy → IPC → Leptos |
| surface activation | Leptos renders canvas/fallback | Bevy activates mir / sugarloaf |
| back/forward | browser UI | browser history API + Bevy reacts |

the navigation model is identical in both targets. on desktop, Bevy adds surface activation and hotkey dispatch. on web, Leptos handles everything alone. the history API is the same.

---

see [[rendering]] for how surfaces are activated once the URL changes
