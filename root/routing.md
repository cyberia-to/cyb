---
tags: cyb, core, architecture
crystal-type: pattern
crystal-domain: cyber
---

# routing

how [[cyb]] navigates. one address space, two routing levels, one source of truth.

---

## the address

the URL is the single source of truth for where the user is. it lives in the WebView (owned by wry). neither Bevy nor Leptos owns it — both subscribe to it.

```
URL changes
  → Bevy reads it via wry navigation event  → activates the right surface
  → Leptos reads window.location            → renders the right aip and sub-page
```

both react independently. no sync, no IPC for routing.

**writing the URL:**

| trigger | who writes | how |
|---|---|---|
| global hotkey | Bevy | `webview.load_url("cyb://brain")` |
| tray menu item | Bevy | `webview.load_url(...)` |
| user taps a link | Leptos | `history.pushState()` / `<A href=...>` |
| programmatic nav | Leptos | `use_navigate()` |

in the browser (no Bevy): Leptos router handles everything alone. same URLs, same behavior. Bevy's absence is invisible to Leptos.

---

## two levels

### level 1 — aip routing (Bevy + Leptos)

the first path segment selects the aip. Bevy reads it to activate the right rendering surface. Leptos reads it to render the right aip component.

| path | aip | Bevy surface |
|---|---|---|
| `/oracle` | oracle — search and knowledge | WebView |
| `/brain` | brain — cybergraph explorer | WebView + mir (wgpu) |
| `/sense` | sense — messages and feed | WebView |
| `/sigma` | sigma — wallet and balances | WebView |
| `/portal` | portal — onboarding | WebView |
| `/sphere` | sphere — heroes and staking | WebView |
| `/senate` | senate — governance | WebView |
| `/teleport` | teleport — token transfers | WebView |
| `/hfr` | hfr — supercomputing resources | WebView |
| `/terminal` | terminal — nushell shell | WebView + sugarloaf |

### level 2 — sub-routing (Leptos only)

within each aip, Leptos handles sub-navigation. Bevy is unaware — it only cares about the first segment.

**oracle:**
```
/oracle                     main search page
/oracle/:cid                single particle view
/oracle/:cid/backlinks      incoming cyberlinks to particle
/oracle/particles           all particles feed
/oracle/neurons             neuron directory
/oracle/blocks              block explorer
/oracle/txs                 transaction explorer
/oracle/contracts           smart contracts
```

**brain:**
```
/brain                      full cybergraph visualization
/brain/:cid                 graph centered on particle
/brain/:neuron/graph        neuron's personal graph
```

**sense:**
```
/sense                      personal feed
/sense/:neuron              specific neuron's messages
/sense/thread/:cid          message thread
```

**sigma:**
```
/sigma                      wallet overview
/sigma/send                 send tokens
/sigma/stake                delegate stake
/sigma/history              transaction history
```

**sphere:**
```
/sphere                     heroes list
/sphere/:neuron             hero profile and delegation
```

**senate:**
```
/senate                     active proposals
/senate/:id                 specific proposal and voting
```

**teleport:**
```
/teleport                   swap interface
/teleport/:from/:to         specific pair (e.g. /teleport/boot/hydrogen)
```

**portal:**
```
/portal                     onboarding entry
/portal/passport            identity and avatar setup
/portal/gift                gift claiming
```

---

## particle addressing

any particle in the [[cybergraph]] has a URL:

```
/oracle/:cid
```

where `:cid` is the particle's content hash. this means every piece of knowledge in cyb is directly addressable. share a URL, share a particle. the oracle aip renders whatever particle type it is — text, image, video, code, pdf.

---

## browser vs desktop

| | browser | desktop |
|---|---|---|
| URL host | browser address bar | wry WebView |
| level 1 routing | Leptos (no Bevy) | Bevy activates surface + Leptos renders |
| level 2 routing | Leptos | Leptos |
| surface activation | Leptos renders canvas elements for 3D/terminal | Bevy activates mir / sugarloaf |
| back / forward | browser history API | Leptos history API + Bevy reacts |

the Leptos router is identical in both cases. on desktop it has a silent co-subscriber (Bevy) that reacts to level 1 changes to activate native surfaces. on web, Leptos handles it alone.

---

see [[rendering]] for how surfaces are activated, [[prysm/layout]] for the grid those surfaces inhabit
