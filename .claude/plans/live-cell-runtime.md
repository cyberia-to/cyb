# Milestone: cyb as a live cell-runtime — the robot landing as the first cell

## the goal

Ship a real, running cyb that renders a **robot sales landing page**, built entirely
from the stack we have (rune + prysm + radio + inf + cybergraph). The landing must be
**updatable live, inside cyb, without rebuilding the binary**.

## the key idea

cyb stops being a monolith where every screen is hardcoded Rust. It becomes a thin
native **shell** (chrome + renderer + runtimes) that loads **cells** — application pages
authored in `rune`, decoded by `prysm`, rendered as Bevy entities — at runtime.

```
rune source (a cell)  →  rune parse/lower/eval  →  chunk-nouns  →  prysm dispatch  →  Bevy UI
        ↑
   source of truth:
   dev:  local file  cyb/cells/landing.rune     (file-watched, instant reload)
   prod: radio particle  cell://landing → hash  (publish = new particle, no rebuild)
```

The pipeline already exists end-to-end for the terminal (`rune <expr>` →
`noun_to_chunks` → `dispatch` → scrollback). This milestone redirects that pipeline
from "one expression into a scrollback log" to "a whole cell into a composed page,"
and feeds it from an external, swappable source.

**Update the app = edit/publish a rune cell. The binary is frozen.**

## what exists today (verified)

- `rune/rs/lower/lib.rs` — chunk-noun constructors: `text`, `anno`, `error`, `log`,
  `button(label,target)`, and `col(e1..en)` (the multi-element/page list). ✅
- `rune/rs/prysm/lib.rs` `noun_to_chunks()` — decodes a `[LIST ...]` noun to `Vec<Chunk>`. ✅
- `prysm/system/rs/scrollback.rs` `dispatch()` — routes each chunk to an atom/molecule
  (text, anno, error, log, status, **action/button**, component). ✅
- `cyb/shell/src/worlds/terminal/mod.rs` — `rune <expr>` is wired live (parse→lower→
  eval→chunks→dispatch). The `TerminalHost` implements the `emit` act. ✅
- `prysm::theme` — H1/H2/H3/BODY/CAPTION sizes, ACID palette, glass atoms. ✅
- `radio` (iroh-blobs) — content-addressed blob store, fetch-by-hash; `particle`
  CLI hashes/encodes/decodes. ✅ (not yet wired to rune loading)

## what's missing (the work)

- A **page renderer** distinct from scrollback: a centered, non-scrolling cell container
  that composes a tree, not an append-only log.
- A **cell source loader**: file (dev) + radio particle (prod), with reload.
- **Button → action** routing: a click must travel from a Bevy entity back into the host
  and trigger a re-render / navigation. Only `emit` is wired today.
- A few **layout/atom primitives** the landing needs (heading variants, image, section/hero).
- `query` act wired to **inf/cybergraph** for live data (later phase).
- `grid.rs` is an empty stub — we deliberately do NOT need full proof-grid for a landing;
  a minimal section/column layout is enough.

---

## phases

### Phase 0 — the cell contract (design, ~½ session)
Write `cyb/specs/cells.md` (or `rune/specs/cells.md`) defining:
- A **cell** = a rune source whose evaluation yields a prysm layout noun (a tree).
- Page primitives needed beyond `col`: `heading(level, text)`, `image(src)`,
  `section(...)` / `hero(...)`. Decide which are new rune builtins vs composed from `col`+`anno`.
- **Cell addressing**: `cell://<name>` resolves to a local path (dev) or a name→hash
  pointer (prod).
- **Button/action semantics for MVP**: a button's `target` names a cell; clicking it
  navigates (loads + renders that cell). Defer general acts/ward.
- Output: one short spec file. No code.

### Phase 1 — cell world + static renderer (the core, ~1 session)
- Add `WorldState::Cell` (or `Landing`) in `cyb/shell/src/worlds/mod.rs`.
- New module `cyb/shell/src/worlds/cell/` — a renderer system that:
  1. reads a rune source string,
  2. runs parse→lower→eval (reuse terminal's `rune_eval_to_chunks` path),
  3. dispatches chunks into a **page container** (centered column, prysm theme,
     non-scrolling) instead of the scrollback buffer.
- Source = local file `cyb/cells/landing.rune` (hardcoded path for now).
- Author `cyb/cells/landing.rune`: hero headline ("every human needs a robot. own,
  don't rent." — from cyberia/strategies/robot.md), sub-copy, a **buy** button.
- **Deliverable:** launch cyb → the robot landing renders, sourced from a rune file.

### Phase 2 — live reload (dev) + the atoms the landing needs (~1 session)
- File watcher (`notify` crate) on `cyb/cells/*.rune` → on change, re-run the renderer.
  Now editing `landing.rune` + save updates cyb live. **This proves "no rebuild to update."**
- Implement the missing primitives identified in Phase 0:
  - `prysm/atoms` — image atom; heading uses existing H1/H2 theme sizes.
  - `prysm/system` — a minimal `section`/`hero` layout (centered block, larger gap).
  - `rune/rs/lower` — lowering constructors for the new primitives + a `(BAR, COMPONENT)`
    or new sigil route in `dispatch()`.
- **Deliverable:** a visually real landing (hero, image, headline, CTA), edited live.

### Phase 3 — interactivity: buy flow (~1 session)
- Wire button click → Bevy event → host action → load+render the target cell.
  New plumbing: a `CellAction` Bevy event carrying the button target; a system that
  catches it and swaps the rendered cell.
- Add `cyb/cells/checkout.rune` (or `minting.rune`): "your robot is being created" +
  the mind/avatar/body structure + a confirm/back button.
- **Deliverable:** landing → click **buy** → checkout cell. "Mimic selling" is a real,
  live-editable flow, zero Rust per screen.

### Phase 4 — radio-backed cells (true live upgrade, ~1 session)
- `load_cell(name) -> String`: resolve `cell://<name>` to a hash via a name→hash pointer
  (radio doc), fetch bytes from iroh-blobs, decode UTF-8 rune source.
- Publish path: `particle encode landing.rune` → push to radio → update the pointer.
- cyb resolves cells from radio at startup + on a manual "refresh" trigger (commander
  command `reload` or a tray item).
- **Deliverable:** update the running app by **publishing a particle** — no local file,
  no binary change. The production live-upgrade story.

### Phase 5 — live data via inf + cybergraph (full-stack demo, ~1 session)
- Wire the `query` act in the cell host: `query(<inf query>)` runs an inf datalog query
  over cybergraph and returns a noun prysm can render.
- Landing shows a live stat (e.g. robots minted / neurons online) pulled from the graph.
- **Deliverable:** prysm renders ← rune scripts ← inf queries ← cybergraph stores,
  end to end, in a shipped page.

---

## critical path & dependencies

```
P0 contract ─→ P1 cell world+renderer ─→ P2 live reload+atoms ─→ P3 buy flow
                                                                      │
                                       P4 radio-backed cells ─────────┤ (independent of P3,
                                                                      │  can run parallel)
                                       P5 inf/cybergraph live data ───┘ (needs P1 host)
```

P1 is the linchpin: the page renderer + cell world. Everything else hangs off it.
P4 (radio) and P5 (inf) are independent and can be done in either order once P1 lands.

## key decisions to confirm before P1

1. **New WorldState vs. reuse Terminal?** Recommend a dedicated `Cell`/`Landing` world —
   cleaner than overloading the scrollback. Chrome stays on top unchanged.
2. **Page layout depth for MVP.** Recommend minimal centered section/column now; do NOT
   build the proof-grid (`grid.rs`) yet. Revisit if the landing needs 2D placement.
3. **New rune builtins vs. compose from `col`+`anno`+theme.** Prefer composing where
   possible to keep rune's surface small; add `heading`/`image` only if needed.
4. **Button action model.** MVP = "target names a cell, click navigates." Full acts
   (`link`/`seal`/`host`) + ward come later; don't block the milestone on them.

## risks

- **Button→host round-trip** is the one genuinely new piece of plumbing (Bevy event →
  host → re-render). Prototype it early in P3; it's small but central to interactivity.
- **rune eval must stay non-blocking** (the terminal already guarantees instant-start);
  reuse that path, don't introduce a blocking compile on the render thread.
- **Scope creep into full prysm grid / full act+ward system.** Both are explicitly
  out of scope for this milestone — note them as follow-ups.

## the one-sentence outcome

After this milestone, cyb is a frozen native shell that renders a robot-sales landing
authored in rune and stored as a particle, and you change the entire app by editing a
cell and publishing it — never by rebuilding cyb.
