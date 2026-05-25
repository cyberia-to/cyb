# cyb-shell Architecture

## Overview

cyb-shell is a native macOS application built on Bevy ECS game engine. It provides four switchable "worlds" — Terminal, Browser, UI, and Game — sharing one window and one GPU pipeline.

```
                      +-------------------+
                      |     main.rs       |
                      |   Bevy App::new() |
                      +--------+----------+
                               |
           +-------------------+-------------------+
           |                   |                   |
   DefaultPlugins      GpuBridgePlugin      WorldsPlugin
   (Window, Render,    (clones Device/      (WorldState FSM)
    Input, ...)         Queue to main        +-- HotkeysPlugin (Cmd+1..4)
                        world)               +-- TrayPlugin (menu bar)
                               |
           +-------------------+-------------------+
           |              |            |           |
   TerminalWorldPlugin  GameWorld   BrowserWorld  UiWorld
   (sugarloaf+nushell)  (bevy 3d)  (wry webview) (wry + child process)
```

## Shared GPU Resources

**Key principle: one wgpu driver per application.**

Bevy creates the wgpu Instance, Device, Queue, and Adapter at startup. These live in Bevy's render sub-app world. `GpuBridgePlugin` clones Device and Queue into the main world so non-render systems (like Terminal) can use them.

```
Bevy DefaultPlugins
  |
  +-- creates wgpu::Instance, Adapter, Device, Queue
  |   (lives in RenderApp sub-world)
  |
  +-- GpuBridgePlugin::finish()
        |
        +-- clones RenderDevice, RenderQueue
        +-- inserts into main world as Resources
```

Sugarloaf (the GPU terminal renderer) receives Bevy's Device and Queue via `Context::new_external()`. It never creates its own GPU stack — no Instance, no Adapter, no Device, no Queue.

## Offscreen Rendering Pipeline

Terminal does NOT create its own wgpu Surface. Instead, it renders offscreen and feeds pixels into a Bevy Sprite:

```
Bevy's wgpu Device + Queue
         |
         v
  Sugarloaf (external mode)
  renders to offscreen texture
         |
         v
  ctx.read_offscreen_pixels()
  (GPU → CPU readback via staging buffer)
         |
         v
  Bevy Image asset (Bgra8UnormSrgb)
  displayed via Camera2d + Sprite
```

There is only one wgpu Surface in the application — Bevy's own. The Terminal world renders to an offscreen texture, reads pixels back to CPU, and writes them into a Bevy `Image` asset. Bevy's standard rendering pipeline then displays the Sprite like any other 2D element.

## World State Machine

```
WorldState (Bevy States enum):

    Terminal  ←──Cmd+1──  (default)
    Browser   ←──Cmd+2──
    Ui        ←──Cmd+3──
    Game      ←──Cmd+4──
```

Each world registers:
- `OnEnter(WorldState::X)` — setup (create resources, spawn entities)
- `OnExit(WorldState::X)` — teardown (despawn entities, release resources)
- `Update` systems with `.run_if(in_state(WorldState::X))` — only tick when active

Switching is handled by `HotkeysPlugin` (global Cmd+1..4 via `global_hotkey` crate) and `TrayPlugin` (macOS menu bar). Both write to `NextState<WorldState>`.

## Component Stack

### 1. wgpu (v27.0.1)

Low-level GPU API. Used by both Bevy and Sugarloaf. Single shared version — types are directly compatible.

Provides: Instance, Device, Queue, Surface, TextureView, CommandEncoder, RenderPass.

### 2. Bevy (v0.18)

ECS game engine. Owns the application lifecycle, window, input, and GPU resources.

- **Window** — single native window via winit
- **Render pipeline** — creates wgpu Instance/Device/Queue at init
- **ECS** — entities, components, systems, resources
- **States** — WorldState FSM drives world switching
- **Input** — keyboard events (`KeyboardInput` messages), mouse scroll (`AccumulatedMouseScroll`)

### 3. Sugarloaf (vendored, Rio terminal fork)

GPU-accelerated text renderer. Renders rich text with font shaping, colors, decorations, and cursor quads.

**External mode** (`Context::new_external`):
- Accepts wgpu Device + Queue from outside (Bevy's)
- No Instance, no Adapter, no Surface, no swapchain management
- Surface fields are `Option::None`

**Offscreen mode** (`ctx.enable_offscreen()`):
- Creates an offscreen wgpu Texture (RENDER_ATTACHMENT | COPY_SRC)
- Creates a readback Buffer (MAP_READ | COPY_DST) for GPU→CPU transfer
- `ctx.offscreen_view()` returns a TextureView for rendering
- `ctx.read_offscreen_pixels()` copies texture → buffer → `Vec<u8>`

**Key APIs used by terminal.rs**:
- `Sugarloaf::new_with_context(ctx, fonts, layout)` — create with external GPU context
- `sugarloaf.content().sel(id).add_text(text, style)` — build rich text content
- `sugarloaf.set_objects(vec![RichText {...}, Quad {...}])` — set render objects
- `sugarloaf.render_to_view(&view)` — render to an offscreen TextureView
- `sugarloaf.set_background_color(color)` — terminal background
- `sugarloaf.resize(w, h)` — update context dimensions on window resize

**render_to_view flow**:
```
compute dimensions → compute state updates →
create CommandEncoder → run render passes (rect, rich_text) →
queue.submit(encoder)
```

### 4. alacritty_terminal (v0.25.1)

Terminal emulator grid and ANSI parser. Used as a library — **no PTY, no process spawning**.

- `Term<BevyEventProxy>` — the terminal grid (rows x cols of cells with characters, colors, flags)
- `Processor` — VT100/ANSI escape sequence parser
- `processor.advance(&mut term, bytes)` — feed raw bytes (including ANSI codes) into the grid
- `term.renderable_content()` — read the grid for rendering (cells with fg/bg colors, flags, cursor position)
- `term.damage()` / `term.reset_damage()` — dirty tracking to skip unchanged frames

The Term is just an in-memory grid. Bytes go in (from nushell), styled cells come out (to sugarloaf).

### 5. Nushell (v0.110, vendored)

Shell engine embedded as a Rust library. **No spawned process, no PTY, no file descriptors.**

Crates used:
- `nu-protocol` — types (EngineState, Stack, PipelineData, Value)
- `nu-engine` — eval_block, env management, ClosureEvalOnce
- `nu-parser` — parse input to AST
- `nu-command` — built-in commands (ls, cd, table, etc.)
- `nu-cmd-lang` — core language commands
- `nu-cli` — gather_parent_env_vars, eval_source
- `nu-std` — standard library

**Eval flow**:
```
User types "ls" + Enter
  |
  v
LineBuffer.take_line() → "ls"
  |
  v
nu-parser::parse("ls") → AST Block
  |
  v
nu-engine::eval_block(block) → PipelineData
  |
  v
pipe through `table` command → ANSI-colored string
  |
  v
Vec<u8> with escape codes like "\x1b[36mfile.txt\x1b[0m"
  |
  v
processor.advance(&mut term, &bytes)  → grid cells with colors
  |
  v
term.renderable_content() → sugarloaf render
```

**Async eval**: Long-running commands run on a background `std::thread`. The NuShellEngine is moved to the thread (via `Option::take`), result is sent back via `mpsc::channel`, polled each frame by `poll_eval_results`. Ctrl+C sets an `AtomicBool` flag checked by nushell's `Signals`.

**Prompt**: Evaluates `$env.PROMPT_COMMAND` and `$env.PROMPT_INDICATOR` closures from nushell config after each command completes.

### 6. wry (v0.53)

WebView library for Browser and UI worlds. Creates child WebViews inside Bevy's winit window via `build_as_child()`. WebViews are created/destroyed on world enter/exit.

## Terminal Data Flow (complete)

```
+------------------+     +----------------+     +-------------------+
|  Bevy Input      |     |  LineBuffer    |     |  Nushell Engine   |
|  (KeyboardInput) | --> |  (accumulate   | --> |  (parse + eval    |
|                  |     |   characters)  |     |   on bg thread)   |
+------------------+     +----------------+     +-------------------+
                                                         |
                                                    Vec<u8> ANSI bytes
                                                         |
                                                         v
                          +------------------+     +-------------------+
                          |  alacritty_terminal|   |  Processor        |
                          |  Term grid       | <-- |  (ANSI parser)    |
                          |  (cells + colors)|     +-------------------+
                          +--------+---------+
                                   |
                            renderable_content()
                                   |
                                   v
                          +------------------+
                          |  Sugarloaf       |
                          |  RichText builder|
                          |  + cursor Quad   |
                          +--------+---------+
                                   |
                            render_to_view(&offscreen_view)
                                   |
                                   v
                          +------------------+
                          |  Offscreen       |
                          |  wgpu Texture    |
                          +--------+---------+
                                   |
                            read_offscreen_pixels()
                            (GPU → CPU readback)
                                   |
                                   v
                          +------------------+
                          |  Bevy Image      |
                          |  (Assets<Image>) |
                          +--------+---------+
                                   |
                            Bevy Camera2d + Sprite
                            (standard 2D pipeline)
                                   |
                                   v
                          +------------------+
                          |  Bevy Surface    |
                          |  (presentation)  |
                          +------------------+
```

## Terminal Lifecycle

```
setup_terminal (first enter):
  RenderDevice.wgpu_device() → device
  RenderQueue → queue
  Context::new_external(device, queue, Bgra8UnormSrgb, size, scale)
  ctx.enable_offscreen()  → creates offscreen texture + readback buffer
  Sugarloaf::new_with_context(ctx, fonts, layout)
  init_nushell_engine() → NuShellEngine
  evaluate_prompt() → feed initial prompt into term
  create Bevy Image (Bgra8UnormSrgb, window size)
  spawn Camera2d + Sprite (with image handle)
  insert TerminalNonSendState

terminal_update (each frame):
  check_resize() → resize sugarloaf + term grid + Bevy Image if window changed
  process_keyboard_input() → feed chars into LineBuffer + Term
  process_scroll_input() → term.scroll_display()
  poll_eval_results() → receive output from bg thread, feed into term
  render_terminal():
    term.damage() → skip if no changes
    render_terminal_content() → build sugarloaf RichText from term grid
    sugarloaf.render_to_view(&offscreen_view)
    ctx.read_offscreen_pixels() → Vec<u8>
    copy pixels into Bevy Image data

destroy_terminal (on world switch away):
  despawn Camera2d + Sprite entities (TerminalMarker)
  keep TerminalNonSendState alive (state persists between switches)

setup_terminal (re-enter):
  detect existing TerminalNonSendState → skip init
  check_resize() in case window changed while inactive
  re-spawn Camera2d + Sprite with existing image handle
  force_full_render = true
```

## World Details

### Terminal (Cmd+1, default)
Offscreen GPU rendering via sugarloaf → Bevy Sprite. Embedded nushell engine. Full details above.

### Browser (Cmd+2)
wry WebView child window loading `https://cyb.ai` (release) or `https://localhost:3001` (debug). IPC handler registered for webview→app messages. Created on first frame of active state, destroyed on exit. Bounds updated each frame to match window size.

### UI (Cmd+3)
wry WebView child window + spawned child process. In debug mode runs `dx serve --port 8080` (Dioxus dev server). In release mode spawns bundled `cyb-ui` binary on port 8080. WebView points to `http://localhost:8080`. Child process killed on world exit.

### Game (Cmd+4)
Standard Bevy 3D scene: Camera3d, rotating Cuboid mesh with green StandardMaterial, DirectionalLight. Uses Bevy's native render pipeline directly.

## Agent Plugin

Browser automation via `AgentCommandSender` resource (mpsc channel). Commands: `Navigate(url)`, `EvalJs(code)`, `GetUrl`. Processes commands against the Browser world's WryWebView. Responses sent as Bevy `Message<AgentResponse>`.

## File Map

```
cyb/cyb-shell/src/
  main.rs              App entry, plugin registration, GpuBridgePlugin
  shell/
    hotkeys.rs         Cmd+1..4 global hotkeys (global_hotkey crate)
    tray.rs            macOS menu bar (tray-icon crate)
  worlds/
    mod.rs             WorldState enum, WorldsPlugin
    terminal.rs        Terminal world (sugarloaf + alacritty + nushell)
    game.rs            Game world (Bevy Camera3d + rotating cube)
    browser.rs         Browser world (wry WebView → cyb.ai)
    ui.rs              UI world (wry WebView + Dioxus child process)
  agent/
    mod.rs             Agent plugin exports
    browser.rs         AgentCommandSender, Navigate/EvalJs/GetUrl commands

cyb/cyb-shell/assets/
  nu-config/
    env.nu             Nushell environment config (embedded via include_str!)
    config.nu          Nushell config (embedded via include_str!)

cyb/vendor/
  sugarloaf/           Vendored GPU text renderer (forked from Rio terminal)
    src/
      context/mod.rs   wgpu Context (new_external + offscreen mode)
      sugarloaf.rs     Main renderer (new_with_context, render_to_view)
      components/      Brushes: rect, rich_text, filters, layer
      font/            Font loading and shaping
  nushell/             Vendored Nushell engine
    crates/            nu-protocol, nu-engine, nu-parser, nu-command, etc.
```

## Build

```bash
cd cyb && make dmg
```

Produces `cyb/target/release/cyb.dmg` — a macOS app bundle with:
- `cyb-shell` binary (single executable, nushell embedded)
- No external nu binary, no config files to copy
- Nushell config embedded via `include_str!`
- Optional `web-dist/` resources if present
