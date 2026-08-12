# cyb roadmap

## architecture

cyb is one robot with four implementations sharing one reference.

```
/ (git root)
├── bevy/              the native shell — orchestrates all modes
├── react/             web UI (cyb-ts)
├── leptos/            WASM UI
├── nu/                scripting/terminal interface
├── reference/         specification, knowledge graph, docs
├── docs/              documentation
├── Cargo.toml         Rust workspace (bevy, leptos, nu)
├── Makefile           build orchestration for all four
```

bevy is the root runtime. all four implementations are accessible from bevy:
- bevy renders its own native UI (graph, 3D, effects)
- bevy embeds react via WebView
- bevy embeds leptos via WebView (WASM)
- bevy embeds nu as integrated terminal/scripting engine

each implementation is self-contained and independently buildable.
all four read the same reference/ for specifications.

## restructuring (next)

current state: react app at git root, rust workspace nested in cyb/.
target state: rust workspace at git root, react in react/ subproject.

rename map:
```
cyb/cyb/    →  bevy/
cyb/cyb-portal/   →  leptos/
cyb/cyb-services/ →  nu/
cyb/vendor/       →  vendor/
src/               →  react/src/
deno.json          →  react/deno.json
rspack.config.*    →  react/
package.json       →  react/
```

files that need path updates:
- Makefile: 7 occurrences of ../ become react/
- .github/workflows/: cd react && deno task build
- .gitmodules: cyb/vendor/nushell → vendor/nushell
- .gitignore: full rewrite
- netlify.toml: base = "react"
- Dockerfile, docker-compose.yml
- CLAUDE.md, README.md

files that need NO changes (relative paths preserved):
- all Cargo.toml (workspace members stay relative)
- all .rs source files
- all rspack.config.*.js (use __dirname)
- deno.json, tsconfig.json, biome.json, codegen.ts

## four modes in bevy

bevy shell provides mode switching:
- desktop: keyboard shortcuts (Cmd+1/2/3/4)
- android: bottom tab or swipe navigation
- all platforms: command palette

| mode | tech | what | shortcut |
|------|------|------|----------|
| graph | bevy native | 3D graph, effects, native UI | Cmd+1 |
| portal | leptos WASM | portal, citizenship, staking | Cmd+2 |
| legacy | react | full cyb-ts app (oracle, robot, settings) | Cmd+3 |
| terminal | nu | scripting, datalog, graph queries | Cmd+4 |

## android

**principle: android cyb = desktop cyb.** one codebase, one binary, same Bevy app, same worlds,
same terminal, same nushell. android adds `nu_plugin_android` exposing hardware APIs that desktop
doesn't have. no separate android UI, no JS bundles, no special android terminal.

**one binary rule:** WebView is only for web content (Legacy, Portal worlds). all other
rendering — terminal, graph, 3D — is pure Rust via Bevy + wgpu. no JS bundles, no external
runtimes embedded in the binary.

current state:
- [x] wry asset loader with cyb.assets domain
- [x] strip CSP for local file loading
- [x] 16KB page alignment for modern Android
- [x] unblock all routes on mobile

android parity plan (see `reference/android.md`):
- [ ] **phase 1** — swap tao+wry stub for Bevy `android_main` (2 days)
- [ ] **phase 2** — terminal world on Android: zero code changes, just Android env init (1 day)
- [ ] **phase 3** — `nu_plugin_android`: GPS, camera, sensors, intent, clipboard, battery (4–5 days)
- [ ] mode switching UI for android (swipe / bottom tabs)
- [ ] leptos/react worlds via WebView inside Bevy (same as desktop)

## react app (legacy mode)

recent changes:
- [x] miner moved from circular menu to AOS hub
- [x] avatar page: @username.moon header, dedicated menu (sigma, sense, time, brain)
- [x] lightning strikes on real blocks only
- [x] ledger integration, keplr removal
- [x] mobile routes unblocked

pending:
- [ ] merge feat/merge-mining-web into master (185 commits, 0 conflicts, ff possible)
- [ ] clean up react code structure (components vs containers vs pages)
- [ ] typescript strict mode
- [ ] test coverage

## infrastructure

- [ ] restructure repo (this plan)
- [ ] update CI for new structure
- [ ] single Makefile for all targets: make web, make desktop, make android, make all
- [ ] unified dev workflow: make dev (starts all needed servers)
