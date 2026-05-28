# cyb-stream

A typed, framed byte-stream protocol that replaces ANSI as the surface between cyb's computation and rendering layers. Native cyb everywhere. No terminal emulator. No PTY. Legacy TUI bridge intentionally out of scope — the new paradigm is good enough to live without it.

## why this exists

Today the terminal world is a hand-built `alacritty_terminal::Term` grid driven by ANSI bytes that nushell (embedded as a library) generates and sugarloaf re-rasterises into glyphs. Three layers all paying tax to a 1972 protocol:

- nushell's structured `PipelineData` is flattened to text+ANSI before we ever see it
- sugarloaf maintains a 24×80-style cell grid, font shaping per cell, color resolution, scrollback, alt-screen, OSC strings — all in service of replaying that stream
- the user gets text-pretending-to-be-a-table, not a real table

cyb-stream removes the pretense. Producers emit typed framed chunks. Consumers (prysm) render each chunk with the native molecule for its type. Nothing gets converted to ANSI at any point; nothing has to be parsed back out.

The protocol composes cybermark's 13 sigils (semantic role) with the 14-language render set (perception type). Each chunk on the wire is *literally* a cybermark phrase in bytes. Debuggable with `cat`. LLM-readable as structured data. Renderable identically on desktop, mobile, ssh.

## frame format

```
+------+-------+--------+---------+---------+
| 0x1F | sigil | render | varint  | payload |
| 1 B  | 1 B   | 1 B    | 1-9 B   | N bytes |
+------+-------+--------+---------+---------+
```

| Field   | Bytes  | Notes                                                      |
|---------|--------|------------------------------------------------------------|
| Marker  | 1      | `0x1F` ASCII Unit Separator. Spare in `cyber/research/256 symbols.md`. Never appears in valid UTF-8. Not claimed by nox ISA (which stops at 0x1E). Not used by any terminal. |
| Sigil   | 1      | One ASCII byte from cybermark's 13 sigils. The semantic role of the chunk. |
| Render  | 1      | One ASCII letter from the render table below. How the chunk should be drawn. |
| Length  | 1-9    | LEB128 unsigned varint. Length of payload in bytes.        |
| Payload | N      | Bytes. Usually UTF-8. May contain nested `0x1F` frames for compositions. |

Total frame header is 4-12 bytes. Empty chunks (no payload) are 4 bytes.

### sigil byte (semantic role)

From `crystal/markup.md`:

| Byte | Sigil | Name | Role                                       |
|------|-------|------|--------------------------------------------|
| 0x23 | `#`   | hax  | content / particle identity                |
| 0x40 | `@`   | pat  | identity / agent / neuron                  |
| 0x7E | `~`   | sig  | annotation / label / side-info             |
| 0x2F | `/`   | fas  | scope / containment / structure            |
| 0x24 | `$`   | buc  | economic / value-bearing                   |
| 0x5E | `^`   | ket  | lift / abstract / establish                |
| 0x21 | `!`   | zap  | effect / imperative / action               |
| 0x2E | `.`   | dot  | transform / pipeline / apply               |
| 0x7C | `\|`   | bar  | composition / code-with-data               |
| 0x3D | `=`   | tis  | binding / equivalence                      |
| 0x3F | `?`   | wut  | test / decision / input request            |
| 0x3A | `:`   | col  | pair / key-value                           |
| 0x2B | `+`   | lus  | augment / increment                        |

### render byte (perception type)

One ASCII letter, drawn from the 14-language render set in `cyb/languages.md`:

| Byte | Letter | Render type     | Source language                  |
|------|--------|-----------------|----------------------------------|
| `s`  | s      | struct          | Nox — collapsible tree           |
| `b`  | b      | binary / pixels | Bt — raster image                |
| `t`  | t      | text            | Rs — prose / code                |
| `f`  | f      | formula         | Tri — math notation              |
| `v`  | v      | vector          | Arc / Ren / Dif — SVG, 2D/3D     |
| `m`  | m      | movie / video   | Seq — frames                     |
| `T`  | T      | table           | Inf — 2D grid of records         |
| `a`  | a      | audio / sound   | Wav — waveform                   |
| `c`  | c      | component       | Ten — nested composition         |
| `k`  | k      | token view      | Tok — ledger / balance           |

Plus four meta-render letters for stream-control chunks that have no language equivalent:

| Byte | Letter | Render type   | Use                                           |
|------|--------|---------------|-----------------------------------------------|
| `p`  | p      | progress      | live progress / status update                 |
| `l`  | l      | log           | structured log line                           |
| `e`  | e      | error         | typed error with source location              |
| `x`  | x      | status        | end-of-command sentinel (exit code)           |
| `i`  | i      | input event   | user → producer (response to interaction)     |

Uppercase / lowercase distinguishes block vs inline rendering when ambiguous (e.g. `t` inline text fragment, `T` table; `s` struct tree, `S` reserved for "structural separator" if we need one).

### varint encoding

Unsigned LEB128. Each byte: low 7 bits payload, high bit = continuation. Same encoding used by protobuf, DWARF, wasm. Single-byte for lengths < 128 (the common case).

### nesting

A `(c, *)` component chunk's payload is itself a sequence of cyb-stream frames. Recursion is the only composition primitive. Parsing is the same code at every depth.

```
0x1F | c | (block render) | len=42 | 0x1F t t len=5 "hello"  0x1F ! c len=20 ...
       └ sigil               varint        └ inner chunk         └ another inner
       └ render
```

A struct chunk `(s, s)` works the same way: payload is nested frames forming a key/value tree. No JSON in the protocol itself.

## v0 chunk catalog

Minimum set to bring up the terminal world. Each entry: `(sigil, render)` — meaning — payload format.

### text and identity

| Chunk     | Meaning                                  | Payload                            |
|-----------|------------------------------------------|------------------------------------|
| `(#, t)`  | particle rendered as text                | UTF-8 (may contain cybermark refs) |
| `(#, T)`  | particle rendered as table               | nested chunks (see table)          |
| `(@, t)`  | neuron reference                         | UTF-8 (e.g. `@alice`)              |
| `(~, t)`  | annotated text fragment                  | UTF-8                              |
| `(/, c)`  | scope / breadcrumb header                | nested chunks                      |

### action and interaction

| Chunk     | Meaning                                  | Payload                                            |
|-----------|------------------------------------------|----------------------------------------------------|
| `(!, c)`  | button / action component                | nested: label `(~,t)` + ref `(#,t)`                |
| `(?, c)`  | input request (prompt the user)          | nested: prompt + schema                            |
| `(?, i)`  | input event (user response back)         | nested: ref + value                                |

### value and quantity

| Chunk     | Meaning                                  | Payload                                            |
|-----------|------------------------------------------|----------------------------------------------------|
| `($, t)`  | token expression as text                 | UTF-8 (e.g. `1000 BOOT`)                           |
| `($, k)`  | token view (balance / ledger row)        | nested: amount + symbol + (optional) particle ref  |
| `(+, t)`  | numeric augment (delta)                  | UTF-8 number                                       |

### composition and structure

| Chunk     | Meaning                                  | Payload                                            |
|-----------|------------------------------------------|----------------------------------------------------|
| `(\|, c)`  | composition container                    | nested chunks (rendered as one component)          |
| `(., \|)`  | pipeline view (live, reactive)           | nested: source ref + step list                     |
| `(:, s)`  | key/value pair / record row              | nested: key chunk + value chunk                    |
| `(/, s)`  | struct tree                              | nested chunks                                      |

### meta / stream control

| Chunk     | Meaning                                  | Payload                                            |
|-----------|------------------------------------------|----------------------------------------------------|
| `(., p)`  | progress update                          | nested `(=, s)`: id, label, current, total         |
| `(., l)`  | log line                                 | nested `(=, s)`: level, source, message            |
| `(!, e)`  | typed error                              | nested `(=, s)`: level, source, message, span?     |
| `(., x)`  | status / end-of-command                  | nested `(=, s)`: code                              |

Each `(=, s)` pair contains `(~, t)[key]` + typed value chunk. Read with `cyb_stream::read_kv()`. The TIS sigil (`=`) is "binding / equivalence" — the same semantics as a TOML `key = value` assignment, expressed natively in the protocol.

### table chunk layout

Tables are the workhorse. Payload of a `(#, T)` chunk is a sequence of inner chunks:

```
0x1F | # | T | len=L |
    [ 0x1F | / | s | len=N | <nested chunks: one (~,t) per column header> ]    ← schema row
    [ 0x1F | : | s | len=M | <nested chunks: one cell chunk per column> ]     ← data row 1
    [ 0x1F | : | s | len=M | <nested chunks: one cell chunk per column> ]     ← data row 2
    ...
```

Each cell is itself a chunk — so a column can hold text, formulas, vectors, sub-tables, anything. This is the structural equivalent of nushell's `Value` tree, on the wire.

### text chunk and cybermark inlining

The payload of a `(*, t)` text chunk is UTF-8. Cybermark sigils inside the text are live: `#cyber/truth`, `@alice`, `$BOOT`, `[[wikilink]]` — rendered as inline references by the consumer. For v0 the parser does light recognition (sigil followed by ident → clickable link). Full cybermark inline computation (`#[expr]`, `~[expr]`) lands later when rune is wired in.

## prysm molecules (v0)

One molecule per `(sigil, render)` pair, plus a few utility ones. Each molecule:

- accepts a parsed `Chunk` struct
- spawns a Bevy entity hierarchy
- updates in place when a chunk with the same `id` (where defined) arrives again

```
prysm/src/stream/
├── mod.rs                stream consumer + scrollback container
├── chunk.rs              `Chunk` struct, parser, writer (re-export from cyb-stream crate)
├── text.rs               (#,t), (~,t), (@,t), (#,t with cybermark) — text widget
├── table.rs              (#,T) — header row + data rows, sortable columns
├── action.rs             (!,c) — button, emits (!,i) on click
├── progress.rs           (.,p) — bar with label, dedup by id
├── error.rs              (!,e) — red-tinted text panel with source link
├── log.rs                (.,l) — single-line log, level-coloured
├── status.rs             (.,x) — invisible sentinel; signals "ready for next prompt"
├── component.rs          (|,c), (/,c) — recursive container; instantiates child molecules
├── pipeline.rs           (.,|) — live pipeline view (later)
└── input.rs              (?,c) — prompt molecule; collects answer, emits (?,i)
```

### molecule contract

```rust
pub trait Molecule {
    /// One ASCII byte for the sigil this molecule handles.
    fn sigil(&self) -> u8;
    /// One ASCII byte for the render this molecule handles.
    fn render(&self) -> u8;
    /// Spawn the widget hierarchy for this chunk under `parent`.
    /// Returns the root entity for later update.
    fn spawn(&self, commands: &mut Commands, parent: Entity, chunk: &Chunk) -> Entity;
    /// Optional: update an existing entity from a new chunk (e.g. progress).
    fn update(&self, commands: &mut Commands, entity: Entity, chunk: &Chunk) { /* default: respawn */ }
}
```

A registry maps `(sigil, render)` → boxed molecule. Adding a new chunk type = adding a molecule + registering it. No core changes.

### the scrollback container

```rust
#[derive(Component)]
struct StreamScrollback {
    chunks: Vec<(ChunkId, Entity)>,  // append-only
    last_status: Option<i32>,
}
```

A vertical Bevy flex column. Each incoming chunk is appended (or coalesced for progress). A `(., x)` status chunk marks the end of a command's output — used by the terminal world to know when to render the next prompt.

## nushell adapter (producer)

The embedded nushell engine currently returns `PipelineData` which we flatten to bytes via the `table` command. Replace that path with a direct `PipelineData → Chunk` walker.

```rust
// shell/src/worlds/terminal/nushell_to_stream.rs

pub fn pipeline_to_chunks(
    data: PipelineData,
    engine: &mut NuShellEngine,
    tx: &Sender<Chunk>,
) {
    match data {
        PipelineData::Empty => {}
        PipelineData::Value(v, _) => value_to_chunks(v, tx),
        PipelineData::ListStream(stream, _) => list_stream_to_chunks(stream, tx),
        PipelineData::ByteStream(stream, _) => byte_stream_to_chunks(stream, tx),
    }
}

fn value_to_chunks(v: Value, tx: &Sender<Chunk>) {
    match v {
        Value::String { val, .. }  => tx.send(Chunk::text(&val)).ok(),
        Value::Int    { val, .. }  => tx.send(Chunk::text(&val.to_string())).ok(),
        Value::Float  { val, .. }  => tx.send(Chunk::text(&val.to_string())).ok(),
        Value::Bool   { val, .. }  => tx.send(Chunk::text(if val {"true"} else {"false"})).ok(),
        Value::Record { val, .. }  => tx.send(record_to_chunk(val)).ok(),
        Value::List   { vals, .. } => tx.send(list_to_chunk(vals)).ok(),
        Value::Nothing { .. }      => None,
        Value::Error  { error, .. }=> tx.send(error_to_chunk(error)).ok(),
        Value::Closure| Value::Custom { .. } | _ => tx.send(Chunk::text(&v.to_debug_string())).ok(),
    };
}

fn list_to_chunk(vals: Vec<Value>) -> Chunk {
    // If all items are records with the same key set → table chunk.
    // Otherwise → struct chunk (nested values).
    if vals.iter().all(is_record_with_same_shape) {
        records_to_table(vals)
    } else {
        values_to_struct(vals)
    }
}
```

### streaming externals

External commands (cargo, git, ffmpeg) come through `PipelineData::ByteStream`. We read in 4-8 KB chunks and emit `(~, t)` text chunks as they arrive — exactly the streaming we ship today, just typed.

Optionally: detect progress patterns (`\r`-overwriting lines, `[##---] 30%`) and convert into `(., p)` progress chunks. This is opt-in heuristic enrichment — never required.

### errors

Nushell's `ShellError` has rich diagnostic info — file, span, label, help. Map directly to `(!, e)` with structured payload.

### prompt

The shell prompt (`PROMPT_COMMAND`, `PROMPT_INDICATOR`) becomes a `(/, c)` scope chunk with text children — clickable breadcrumb of cwd, git branch, etc. No more ANSI escape codes in the prompt template; nushell config emits structured data.

## terminal world rewrite

Strip the `Term` + `Processor` + sugarloaf path. Replace with a chunk consumer that pipes into prysm.

### new terminal state

```rust
struct TerminalState {
    nu_engine: Option<NuShellEngine>,
    line_buffer: LineBuffer,
    eval_rx: Option<Receiver<EvalMsg>>,
    eval_in_progress: bool,
    scrollback: Entity,   // the StreamScrollback Bevy entity
    ctrlc_flag: Arc<AtomicBool>,
    key_cursor: MessageCursor<KeyboardInput>,
}

enum EvalMsg {
    Chunk(Chunk),
    Done { engine: NuShellEngine, error: Option<String> },
}
```

Gone: `Term<BevyEventProxy>`, `Processor`, `Sugarloaf`, `cols/rows`, `rich_text_id`, `image_handle`, all the cell-grid resize plumbing, ANSI byte conversion.

### update loop

```rust
fn terminal_update(world: &mut World) {
    // 1. read commander dispatches → push into nushell
    // 2. read keyboard → line buffer (commander); on Enter → dispatch_eval
    // 3. drain eval_rx → for each Chunk, spawn the molecule into the scrollback
    // 4. when EvalMsg::Done arrives → render prompt, restore engine
}
```

The render system goes away entirely. Prysm widgets layout themselves through Bevy UI. No `render_terminal_content`, no offscreen pixel readback, no image asset.

### file size

Current `terminal.rs`: 1550 lines.
Estimated post-rewrite: 350-450 lines, split across:

```
shell/src/worlds/terminal/
├── mod.rs                       plugin, setup, teardown        ~80
├── state.rs                     TerminalState                  ~40
├── input.rs                     keyboard → line buffer          ~120
├── eval.rs                      dispatch_eval, poll loop        ~80
├── nushell_to_stream.rs         PipelineData → Chunk            ~150
└── prompt.rs                    PROMPT_COMMAND → chunks         ~40
```

## implementation slices

Each slice is one focused session. They land in order; each one leaves the tree compilable and produces visible progress.

### slice 1 — protocol spec + parser crate (one session)

Write `cyb-stream` as a new workspace crate.

- `cyb/cyb-stream/Cargo.toml` — no Bevy dependency, pure Rust.
- `cyb/cyb-stream/src/lib.rs`
  - `Chunk { sigil: u8, render: u8, payload: Bytes, id: Option<ChunkId> }`
  - `Reader<R: io::Read>` — pulls frames out of a byte stream
  - `Writer<W: io::Write>` — emits frames
  - varint encode/decode
  - constants for sigil and render bytes
- Tests:
  - roundtrip every v0 chunk type
  - nested chunks (component containing text + table)
  - truncated frames return Pending, not Error
  - malformed frames (missing 0x1F) skip to next 0x1F

Acceptance: `cargo test -p cyb-stream` green. ~400 lines incl tests.

### slice 2 — prysm molecules (one session)

In `prysm/src/stream/`:

- `mod.rs` — `StreamConsumer` resource + `StreamScrollback` component
- `MoleculeRegistry` — `HashMap<(u8, u8), Box<dyn Molecule>>`
- molecules for: text, error, log, status, progress, action, component, table
- one Bevy system: `consume_chunks` reads from a channel, spawns molecules into the active scrollback

Visual sanity: a `prysm-stream-demo` example binary that hand-emits a stream of every v0 chunk type and renders them. Run it, see the result. This is the user-visible payoff of slice 2.

Acceptance: example binary runs, table sorts on column-header click, progress chunk updates in place, action button emits an event on click.

### slice 3 — nushell → cyb-stream adapter (one session)

In `shell/src/worlds/terminal/nushell_to_stream.rs`:

- `pipeline_to_chunks(PipelineData, &mut NuShellEngine, Sender<Chunk>)`
- `value_to_chunks` for every `Value` variant
- `list_to_chunk` heuristic (homogeneous records → table; else struct)
- `byte_stream_to_chunks` for externals (chunked text)
- `error_to_chunk` for `ShellError`

Unit tests in isolation: feed handcrafted `PipelineData` values, assert the chunk sequence. No Bevy needed.

Acceptance: `cargo test -p cyb-shell nushell_to_stream` green.

### slice 4 — wire it into the terminal world (one session)

Rewrite `shell/src/worlds/terminal.rs` → `shell/src/worlds/terminal/`. Replace sugarloaf+alacritty Term with the prysm StreamScrollback. Connect the eval loop to push chunks instead of bytes.

The commander (Cmd+K) keeps working — it already routes to the terminal world; now its forwarded command runs through nushell → cyb-stream → prysm.

Acceptance:
- `ls` shows a real native table with cyb fonts and chroma colors
- `cargo build` shows live progress chunks updating in place
- `git log` shows a list of commits as nested chunks
- `1 + 1` shows `2` as a single text chunk
- errors render in red with source location
- prompt is a `/c` scope chunk with cwd breadcrumb

### slice 5 — strip the old terminal-emulator stack (next session)

Once the new path is solid for a day of real use:

- delete `Term`, `Processor`, `Sugarloaf` usage from terminal
- drop `alacritty_terminal`, `sugarloaf` from `shell/Cargo.toml`
- remove the ANSI color-resolution code, the cell grid, the offscreen pixel readback, the resize plumbing
- update `terminal.md` to reflect the new architecture

Estimated reduction: ~3-5 MB binary, several hundred crates from the dependency tree, ~1100 lines from terminal.rs.

## explicitly out of scope (v0)

These are intentional non-goals — capture here so they don't drift in.

- **Legacy TUI compatibility.** No PTY, no ANSI parser, no `vt/` chroma. helix, htop, vim, less, tmux do not run inside cyb. We're betting the new paradigm + native editor (helix-core integration later) is enough.
- **External shells.** No bash, no zsh. Only embedded nushell. One way to invoke things.
- **Per-cell formatting in tables.** v0 cells are typed chunks; sub-cell rich formatting (a span of red text inside a cell) waits for cybermark inline parsing.
- **rune inline computation.** `#[expr]`, `.[expr|step|step]` inside text payloads. v0 just shows the literal text. rune wiring comes after the protocol is stable.
- **Image/video/sound rendering.** Chunk types are defined, molecules deferred. v0 renders these as a placeholder badge with the chunk metadata.
- **Reactive subscriptions.** A page referencing `#cyber/truth` should re-render when the particle changes. v0 emits chunks once; subscription replay is a v1 add-on.
- **Network streaming.** cyb-stream is local-process-only in v0. Cross-process / cross-machine wrapping (cyb-stream over a socket, cyb-stream over IPFS) is later.

## risks and how we handle them

| Risk | Mitigation |
|------|------------|
| Hot scrollback (lots of chunks) consumes Bevy UI tree | Cap scrollback at N chunks; virtualise above that |
| Table with thousands of rows blows out widgets | Table molecule lazy-spawns visible rows only |
| Nushell `PipelineData` variant we don't handle | Default branch: fall through to debug text chunk so nothing crashes |
| Progress chunks flood the stream | Coalesce by `id` in the consumer (only the latest survives) |
| Terminating a long external command mid-stream | `(., x)` status chunk on the way down + drop the receiver |
| Binary size growth from new prysm widgets | They replace much heavier sugarloaf+wgpu compositor — net reduction |

## the dependency picture after slice 5

```
shell/                 Bevy plugins, terminal world, chroma, nav
   ↓
prysm/                 atoms + molecules + cyb-stream consumer
   ↓
cyb-core/              chroma ids, intent particles, signal bus
   ↓
cyb-stream/            chunk type + reader + writer (NEW)
   ↓
cybergraph/            graph ops, NeuronId, Particle
   ↓
bbg/                   authenticated state, hemera hashing
```

No alacritty_terminal. No sugarloaf. No PTY. The terminal world is a cyb-stream consumer; everything else falls out from that.

## what success looks like

Open cyb. Cmd+4 → Terminal world. Prompt is a breadcrumb showing `~/cyber/cyb` with a git branch chip. Type `ls`. A native table renders — column headers clickable to sort, rows tightly typeset in the chroma font, modified-time column with relative-time chips. Type `cargo build`. Native progress bar at the top of the output area updates smoothly while logs stream into a scrolling list below it; warnings show in amber, errors in red with clickable source links. Type `git log --oneline -10`. A list of commit chunks — each a small composition with hash, author chip, message, expand-to-show-diff action. Type `1 + 1`. The answer `2` shows as a single text node.

Nothing looks like ANSI. Nothing looks like a terminal. Everything looks like cyb — same typography, same chroma colors, same interaction model as every other surface of the app. And under the hood, the protocol carrying it is small enough to print and read.

That's the goal. The plan above gets us there in five focused sessions.
