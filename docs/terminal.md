# Plan: Embedded Nushell Engine (replace PTY-spawned binary)

## Context

The current terminal spawns nushell as an external binary via PTY. This is broken — nushell doesn't produce visible output through the PTY→alacritty_terminal→sugarloaf pipeline (while zsh works fine). The root cause: nushell's interactive mode has complex terminal negotiation that conflicts with our PTY bridge.

The fix: embed nushell as a Rust library. No spawned process, no PTY, no file descriptors. Nushell evaluates commands in-process, produces ANSI output bytes, which feed directly into alacritty_terminal's grid, rendered by sugarloaf.

Obsolete plans `004-remove-egui-add-ascii-terminal.md` and `composed-bubbling-floyd.md` will be deleted.

## Architecture

```
Bevy KeyboardInput → LineBuffer (accumulate chars)
    │ on Enter
    ▼
nu-parser::parse() → nu-engine::eval_block() → PipelineData
    │
    ▼
Capture output as Vec<u8> (with ANSI colors from nu-table)
    │
    ▼
alacritty_terminal::Processor::advance(&mut Term, &bytes)  ← NO PTY
    │
    ▼
Term.renderable_content() → sugarloaf content builder → GPU render
```

## Steps

### Step 1: Cargo dependencies

**`cyb/Cargo.toml`** — add nushell to exclude:
```toml
exclude = ["vendor/rio-backend", "vendor/copa", "vendor/nushell"]
```

**`cyb/cyb-shell/Cargo.toml`** — add nu crates as path deps:
```toml
nu-protocol = { path = "../vendor/nushell/crates/nu-protocol" }
nu-engine = { path = "../vendor/nushell/crates/nu-engine" }
nu-parser = { path = "../vendor/nushell/crates/nu-parser" }
nu-command = { path = "../vendor/nushell/crates/nu-command", default-features = false }
nu-cmd-lang = { path = "../vendor/nushell/crates/nu-cmd-lang" }
nu-cmd-extra = { path = "../vendor/nushell/crates/nu-cmd-extra" }
nu-cli = { path = "../vendor/nushell/crates/nu-cli", default-features = false }
nu-std = { path = "../vendor/nushell/crates/nu-std" }
nu-utils = { path = "../vendor/nushell/crates/nu-utils" }
```

Remove: `libc` (was for PTY fd ops). Keep: `alacritty_terminal`, `sugarloaf`, `bevy`.

Verify: `cargo check -p cyb-shell` compiles.

### Step 2: New structs (replace PTY-based TerminalState)

In `terminal.rs`, replace old state with:

```rust
struct NuShellEngine {
    engine_state: EngineState,
    stack: Stack,
}

struct LineBuffer {
    buffer: String,
    cursor_pos: usize,
    history: Vec<String>,
    history_index: Option<usize>,
}

struct TerminalState {
    nu_engine: Option<NuShellEngine>,  // Option because moved to bg thread during eval
    term: Arc<FairMutex<Term<BevyEventProxy>>>,
    processor: Processor,
    line_buffer: LineBuffer,
    cols: usize,
    rows: usize,
    rich_text_id: usize,
    eval_rx: Option<std::sync::mpsc::Receiver<(EvalResult, NuShellEngine)>>,
    eval_in_progress: bool,
}
```

Remove: `pty_writer`, `_reader_handle`, `_pty`, all PTY imports.

### Step 3: Engine initialization (rewrite setup_terminal)

Keep sugarloaf setup identical. Replace PTY block with:

1. `EngineState::new()` + add command contexts (`nu_cmd_lang`, `nu_command`, `nu_cmd_extra`, `nu_cli`)
2. `gather_parent_env_vars()` + set PWD
3. `load_standard_library()`
4. `convert_env_values()`
5. Load bundled config via `eval_source()` (embed `env.nu`/`config.nu` with `include_str!`)
6. Force `config.use_ansi_coloring = True`
7. Create `Term` WITHOUT PTY (just `Term::new(config, dims, BevyEventProxy)`)
8. Create `Processor::new()`
9. Evaluate prompt, feed ANSI bytes into Term
10. No reader thread, no fd duplication, no `tty::new()`

### Step 4: Keyboard input → line buffer

Replace PTY-write with line buffer accumulation:

- **Characters**: insert into buffer, echo to Term via `processor.advance()`
- **Enter**: take buffer, dispatch eval
- **Backspace**: remove char, send `\x08 \x08` to Term
- **Arrow up/down**: navigate history (clear line with `\r\x1b[K`, redraw prompt + history entry)
- **Arrow left/right**: move cursor in buffer, send `\x1b[D`/`\x1b[C`
- Block input while `eval_in_progress`

### Step 5: Command evaluation with output capture

```rust
fn evaluate_and_capture(engine: &mut NuShellEngine, input: &str) -> Result<Vec<u8>, String>
```

1. `parse()` input → AST block
2. `eval_block::<WithoutDebug>()` → `PipelineData`
3. Capture output:
   - `PipelineData::Empty` → nothing
   - `PipelineData::ByteStream` → `write_to(&mut buf)`
   - `PipelineData::Value/ListStream` → pipe through `table` command → collect string with ANSI codes
4. `engine_state.merge_env(&stack)` to persist env changes (cd, export, etc.)
5. Return `Vec<u8>` with ANSI-colored output

**Never** call `print_table()` or `print_pipeline()` — they write to real stdout.

### Step 6: Async eval (background thread)

For long-running commands:

1. `dispatch_eval()`: take `NuShellEngine` out of state, spawn `std::thread`, send result via `mpsc::channel`
2. New system `poll_eval_results`: `try_recv()` each frame, on result:
   - Return engine to state
   - Feed output bytes into Term
   - Evaluate and render prompt
   - Set `eval_in_progress = false`

### Step 7: Prompt rendering

Evaluate `$env.PROMPT_COMMAND` and `$env.PROMPT_INDICATOR` closures:
- Use `ClosureEvalOnce` to run them
- Collect string output (contains ANSI codes)
- Feed `\r` + prompt bytes into Term

### Step 8: Render system (NO CHANGES)

`render_terminal` stays exactly as-is. It reads `term.renderable_content()` and builds sugarloaf content. The only difference: Term is now fed by `processor.advance()` from eval results instead of a PTY reader thread.

### Step 9: Makefile cleanup

- Remove `nu-build` target
- Remove `nu` binary copy from `dmg` target
- Remove nushell config copy (configs now embedded via `include_str!`)
- `dmg` depends only on `release` (not `nu-build`)

### Step 10: Build & verify

`cd cyb && make dmg` — should produce smaller DMG (no 56MB nu binary).

## Files

| File | Action |
|---|---|
| `cyb/cyb-shell/Cargo.toml` | Add nu-* path deps, remove libc |
| `cyb/Cargo.toml` | Add vendor/nushell to exclude |
| `cyb/cyb-shell/src/worlds/terminal.rs` | Full rewrite of setup + keyboard + new eval systems |
| `cyb/Makefile` | Remove nu-build, simplify dmg |
| `.claude/plans/004-remove-egui-add-ascii-terminal.md` | Delete (obsolete) |
| `.claude/plans/composed-bubbling-floyd.md` | Delete (obsolete) |

## Risks

| Risk | Mitigation |
|---|---|
| Workspace dependency conflicts (nu crates vs cyb deps) | `cargo tree -d` after adding, pin if needed |
| External commands (git, etc.) write to real stdout | Phase 1: accept this. Phase 2: redirect via `OutDest` pipes |
| Binary size increase (nu-command pulls many deps) | Start with defaults, strip features later |
| `table` command output capture | Manual invocation, never `print_table()` |
| EngineState thread safety | `Option<NuShellEngine>` pattern — take/return on eval |

## Verify

1. `cargo check -p cyb-shell` — compiles
2. `cd cyb && make dmg` — DMG builds (smaller, no nu binary)
3. Launch app → Cmd+3 → Terminal:
   - Prompt with ANSI colors visible
   - Type `1 + 1` → see `2`
   - Type `ls` → see colored table output
   - Type `"hello" | str upcase` → see `HELLO`
   - Arrow up → history navigation
   - Cursor visible and blinking
4. Cmd+4 → Cmd+3 → no crash (state cleanup/reinit)
