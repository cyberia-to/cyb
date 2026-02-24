# Bevy Shell — Four Worlds + Dioxus UI + Agent Browser

## Context

cyb-ts is a React/TS SPA (Deno 2 + Rspack + React 18). The goal is to build a native Bevy desktop shell that becomes the host for all cyb functionality. In this first phase, Browser mode loads cyb-ts directly (no rewrite needed), while Dioxus UI, Game mode, Terminal mode, and Agent browser provide the new native infrastructure.

Existing code to reuse: `feat/tauri-app` branch has 240+ lines of mining (uhash-core), 280+ lines of IPFS/Kubo lifecycle, CozoDB/RocksDB wrapper, and warp server — all portable with minimal changes (remove `#[tauri::command]` attributes).

cyb-ts dev server: **HTTPS** on `localhost:3001` (`rspack.config.dev.js`).

---

## Version Matrix

| Crate | Version | Notes |
|---|---|---|
| bevy | 0.18.0 | winit 0.30, raw-window-handle 0.6 |
| bevy_egui | 0.39.1 | Bevy 0.18 compat |
| wry | 0.53 | raw-window-handle 0.6, `build_as_child` |
| global-hotkey | 0.7.0 | NonSend, main thread on macOS |
| tray-icon | 0.21.3 | NonSend, re-exports muda for menus |
| dioxus | 0.7.3 | Separate binary (event loop conflict) |
| tokio | 1.40+ | Async runtime |
| cargo-packager | 0.11.8 | Release bundling |
| uhash-core | v0.2.9 | Git tag (from Tauri branch) |
| cozo | 0.7.6 | storage-rocksdb feature |

**Key compat**: wry 0.53 and Bevy 0.18 (via winit 0.30) both use `raw-window-handle 0.6`. `WebViewBuilder::build_as_child()` accepts winit `Window` directly via `HasWindowHandle`.

---

## Final File Structure

```
cyb/
  Cargo.toml                    # workspace root
  Makefile                      # dev/build automation
  .cargo/config.toml            # CPU flags (from feat/tauri-app)

  cyb-shell/                    # Bevy app — main binary
    Cargo.toml
    Packager.toml
    assets/icon_32x32.rgba
    src/
      main.rs                   # App bootstrap, plugin registration
      worlds/
        mod.rs                  # WorldState enum, WorldsPlugin
        game.rs                 # Bevy 3D scene + egui overlay
        browser.rs              # wry WebView (cyb-ts)
        ui.rs                   # Dioxus (dx serve in dev, child process in release)
        terminal.rs             # Embedded Rio (teletypewriter + rio-backend + sugarloaf)
      agent/
        mod.rs
        browser.rs              # AgentCommand/AgentResponse channels
      shell/
        mod.rs
        hotkeys.rs              # global-hotkey (Ctrl+Shift+1/2/3/4)
        tray.rs                 # tray-icon + menu
    web-dist/                   # (release only) cyb-ts build output

  cyb-ui/                       # Dioxus app — separate binary
    Cargo.toml
    Dioxus.toml
    src/main.rs

  cyb-services/                 # Headless Rust services (ported from Tauri)
    Cargo.toml
    src/
      lib.rs
      mining.rs                 # uhash PoW (from feat/tauri-app:src-tauri/src/mining.rs)
      ipfs.rs                   # Kubo lifecycle (from feat/tauri-app:src-tauri/src/ipfs.rs)
      db.rs                     # CozoDB RocksDB (from feat/tauri-app:src-tauri/src/db.rs)
      server.rs                 # Warp HTTP (from feat/tauri-app:src-tauri/src/server.rs)
```

---

## Steps

### Step 1: Workspace Skeleton + Empty Bevy Window

Create `cyb/` directory with workspace `Cargo.toml`, `cyb-shell/` crate, and minimal `main.rs` that opens a 1280x800 Bevy window titled "cyb".

Copy `.cargo/config.toml` from `feat/tauri-app` branch (CPU optimization flags).

**Files**: `cyb/Cargo.toml`, `cyb/cyb-shell/Cargo.toml`, `cyb/cyb-shell/src/main.rs`, `cyb/.cargo/config.toml`

**Verify**: `cargo build -p cyb-shell` compiles. `cargo run -p cyb-shell` opens a window.

---

### Step 2: WorldState Machine + Global Hotkeys

`WorldState` enum as Bevy `States` (Browser default). Register `OnEnter`/`OnExit` for each state (stubs). `global-hotkey` with NonSend resource for Ctrl+Shift+1/2/3/4.

`GlobalHotKeyManager` is `!Send` → `world.insert_non_send_resource()` in exclusive startup system. `poll_hotkey_events` uses `NonSend<HotkeyManagerRes>` for main thread on macOS.

**Files**: `worlds/mod.rs`, `shell/mod.rs`, `shell/hotkeys.rs`, update `main.rs`

**Verify**: Hotkeys logged in console, work even when window not focused (OS-level).

---

### Step 3: Game World (Bevy wgpu + egui)

Spawn 3D scene (camera, rotating cube, directional light) + egui window with tick counter. Entities marked with `GameSceneMarker`, despawned on exit.

**Files**: `worlds/game.rs`

**Deps**: `bevy_egui = "0.39.1"`

**Verify**: Ctrl+Shift+2 → rotating cube + tick counter. Switching away despawns scene.

---

### Step 4: Browser World (wry WebView) — CRITICAL PATH

Create wry WebView as child of Bevy's winit window: `WinitWindows` (NonSend) → `get_window(entity)` → `WebViewBuilder::build_as_child(winit_window)`.

- `WryWebView` as NonSend resource
- `update_webview_bounds` system resizes on window resize
- On exit: drop resource (destroys webview)
- Dev: `https://localhost:3001`
- Release: `with_custom_protocol("cyb", handler)` serves from `web-dist/`
- Linux: `gtk::init()` before creation, `gtk::main_iteration_do` in Update

**Files**: `worlds/browser.rs`

**Deps**: `wry = "0.53"`, `gtk = "0.18"` (Linux cfg)

**Verify**: App opens showing cyb-ts. Ctrl+Shift+2 → Game → Ctrl+Shift+4 → Browser. Resize works.

---

### Step 5: Agent System (WebView Command Channel)

`mpsc` channels:
- `AgentCommandSender` (Resource, Clone, Send) — any system/thread
- `AgentCommandReceiver` (NonSend) — polled on main thread
- Commands: `Navigate(String)`, `EvalJs(String)`, `GetTitle`
- Responses: `TitleChanged`, `JsResult`, `NavigationComplete`

`process_agent_commands`: reads receiver, calls `webview.load_url()` / `evaluate_script()` if `WryWebView` exists.

Browser mode = same WebView the agent controls, made visible.

**Files**: `agent/mod.rs`, `agent/browser.rs`

**Verify**: Test system sends `Navigate("https://example.com")` → webview navigates.

---

### Step 6: UI World (Dioxus Child Process)

**Event loop conflict**: Dioxus desktop owns its own event loop → cannot coexist with Bevy.

**Dev**: spawn `dx serve --port 8080`, create wry child webview at `http://localhost:8080`.
**Release**: `cyb-ui` compiled as standalone binary, spawned as child process.

On exit: kill child process, drop webview.

**Files**: `worlds/ui.rs`, `cyb-ui/Cargo.toml`, `cyb-ui/Dioxus.toml`, `cyb-ui/src/main.rs`

**Verify**: Ctrl+Shift+1 → Dioxus counter. Switching away kills process.

---

### Step 7: Terminal World (Embedded Rio)

Rio terminal встроен в бинарь через три его crate (все на crates.io v0.2.37):
- `teletypewriter` — кроссплатформенный PTY (спавн шелла)
- `rio-backend` — ANSI парсер, grid state, selection, events (вся логика терминала)
- `sugarloaf` — GPU рендерер на wgpu (Metal/Vulkan/DX12/GLES3)

Sugarloaf и Bevy оба используют wgpu. Интеграция: sugarloaf рендерит в отдельную wgpu текстуру, Bevy показывает эту текстуру через `Image` handle на полноэкранном спрайте/UI node. Ввод перехватывается Bevy `KeyboardInput` events и проксируется в PTY.

On `OnEnter(Terminal)`:
1. Создать PTY через `teletypewriter::create_pty()` (спавн $SHELL)
2. Init `rio-backend` Term grid с размерами окна
3. Init `sugarloaf::Sugarloaf` с wgpu device/queue от Bevy (через `RenderDevice`)
4. Создать render-to-texture target

On `Update` (run_if Terminal):
1. Read PTY output → feed в rio-backend grid
2. Sugarloaf рендерит grid в текстуру
3. Bevy показывает текстуру
4. Forward keyboard/mouse events → PTY write

On `OnExit(Terminal)`: drop PTY, cleanup sugarloaf + текстура.

**Files**: `worlds/terminal.rs`

**Deps**: `teletypewriter = "0.2"`, `rio-backend = "0.2"`, `sugarloaf = "0.2"`

**Verify**: Ctrl+Shift+3 → терминал рендерится внутри Bevy окна. Команды работают. Ctrl+C, vim, цвета — всё через Rio.

---

### Step 8: Tray Icon + Menu

`tray-icon` (NonSend). Menu: four worlds + separator + Quit. Poll `MenuEvent::receiver()` in Update, set `NextState<WorldState>` or `AppExit`.

32x32 RGBA icon from existing assets.

**Files**: `shell/tray.rs`, `assets/icon_32x32.rgba`

**Verify**: Tray icon visible. Menu switches worlds. Quit exits.

---

### Step 9: Port Tauri Services to cyb-services

Extract from `feat/tauri-app` branch:
- `mining.rs` → remove `#[tauri::command]`, `State<>` → `&Arc<>`
- `ipfs.rs` → remove `#[tauri::command]`, keep Kubo lifecycle
- `db.rs` → remove `#[tauri::command]`, keep CozoDB wrapper
- `server.rs` → remove Tauri deps, keep warp API

Deps: `uhash-core` (git v0.2.9), `cozo` 0.7.6, `warp` 0.3.7, `tokio`, `hex`, `dirs`, `reqwest`.

`cyb-shell` depends on `cyb-services`, spawns services in startup system.

**Verify**: `cargo build -p cyb-services`. Unit tests for `meets_difficulty`.

---

### Step 10: Dev/Release Workflows + Makefile

**Dev**:
```
Terminal 1: cd cyb-ts && deno task start     # HTTPS :3001
Terminal 2: cd cyb && cargo run -p cyb-shell # Browser mode → :3001
Terminal 3: cd cyb/cyb-ui && dx serve        # Optional: UI dev
```

**Release**:
```
deno task build → cyb-ts/build/
cp -r build → cyb-shell/web-dist/
cargo build --release
dx build --release
cargo packager --release → .app/.dmg/.deb/.exe
```

**Files**: `Makefile`, `Packager.toml`

---

### Step 11: Integration Testing

| Test | Expected |
|---|---|
| Rapid Ctrl+Shift+1→2→3→4 | Clean transitions, no leaks |
| Terminal: vim, colors, Ctrl+C | Full ANSI support via Rio |
| cyb-ts not running + Browser | wry error page, no crash |
| Window close (X) | All children killed |
| macOS fullscreen | WebView resizes |

Cleanup system on `AppExit`: kill all child processes.

---

## Risks

| Risk | Mitigation |
|---|---|
| wry child steals focus from Bevy | OK for Browser/UI modes. Agent works headless. |
| Dioxus event loop conflict | dx serve + wry in dev, child process in release |
| sugarloaf + Bevy wgpu sharing | Render-to-texture; fallback: sugarloaf в отдельном surface |
| wry 0.53 + winit 0.30 compat | Both rwh 0.6. Test in Step 4. |
| NonSend bottleneck | All on main thread, acceptable for setup/polling |

---

## Criteria

- [ ] `cargo build --release` all workspace members
- [ ] `dx serve` in `cyb-ui/` without Node
- [ ] Bevy shell → Browser mode shows cyb-ts
- [ ] Ctrl+Shift+1 → Dioxus UI
- [ ] Ctrl+Shift+2 → Game (cube + egui)
- [ ] Ctrl+Shift+3 → Terminal (embedded Rio, shell commands work)
- [ ] Ctrl+Shift+4 → Browser (cyb-ts)
- [ ] Hotkeys from any window (OS-level)
- [ ] Tray icon + menu
- [ ] `AgentCommandSender` as Bevy Resource
- [ ] `cargo packager --release` creates distributable
- [ ] All children terminate on exit
