cells — live-loaded application pages

## what a cell is

a cell is one application screen authored in `rune`, decoded by `prysm`, rendered
as native bevy entities. it is the unit of the cyb app that changes without
rebuilding the binary.

```
rune source (a cell)  →  rune parse/lower/eval  →  result chunk-noun  →  noun_to_chunks  →  prysm dispatch  →  bevy UI
```

a cell's rune source evaluates to a **chunk-noun** — normally a `col(...)` list of
elements. the cell world feeds that result through the existing
`rune_prysm::noun_to_chunks` → `prysm::dispatch` pipeline (the same one the
terminal uses for `rune <expr>`), but renders into a centered, non-scrolling
**page container** instead of an append-only scrollback.

the binary is a frozen shell (chrome + renderer + runtimes). the app is the set of
cells. **update the app = edit/publish a cell.**

## the contract

a cell source is a single rune expression. its evaluated result must be a
chunk-noun the prysm bridge recognises:

- a `col(e1, …, en)` list  → a vertical page of elements (the normal case)
- a single element noun     → a one-element page

elements are the chunk-noun constructors already defined in `rune_ast::tag`:

| constructor          | tag      | renders as                    | status |
|----------------------|----------|-------------------------------|--------|
| `text("…")`          | TEXT 1   | body text (HAX/TEXT)          | ✅ now |
| `anno("…")`          | ANNO 2   | dim annotation (SIG/TEXT)     | ✅ now |
| `error("…")`         | ERROR 3  | error line (ZAP/ERROR)        | ✅ now |
| `log("…")`           | LOG 4    | log line (DOT/LOG)            | ✅ now |
| `button(label, tgt)` | BUTTON 5 | action button (ZAP/COMPONENT) | ✅ now |
| `col(…)`             | LIST 0   | vertical list / page          | ✅ now |
| `heading(level,"…")` | (new)    | H1/H2/H3 headline             | ⬜ P2  |
| `image("src")`       | (new)    | image atom                    | ⬜ P2  |
| `section(…)`         | (new)    | spaced block (hero/section)   | ⬜ P2  |

P1 builds the whole page from `text`/`anno`/`button`/`col` only. P2 adds the
heading/image/section primitives (new tag in `rune_ast`, lowering in `rune-lower`,
decode in `rune-prysm`, atom in `prysm`, route in `dispatch`).

## addressing

cells resolve through `cell://<name>`:

- **dev**  → local file `cyb/cells/<name>.rune` (file-watched, instant reload — P2)
- **prod** → radio particle: a `name → hash` pointer; fetch bytes from iroh-blobs,
  decode UTF-8 rune source (P4)

P1 hardwires the path `cyb/cells/landing.rune`. the name→file and name→hash
resolvers come in P2/P4 behind the same `load_cell(name) -> String` seam.

## button semantics (MVP)

a button carries `(label, target)`. for the milestone, `target` names a cell:

- `button("See what's inside", "anatomy")` → click loads `cell://anatomy`
- a click emits a `CellAction(target)` bevy event; the cell world catches it and
  re-renders with the named cell (P3)

general acts (`link`, `seal`, `host`) and the ward permission layer are out of
scope for this milestone — a button only navigates. targets that aren't cell
names are no-ops for now.

## the cell world

a dedicated `WorldState::Cell` (fills the roadmap's reserved "Interface / Cmd+6 /
Native Bevy UI" slot). chrome stays on top unchanged. on enter, the world loads
its current cell, evaluates it, and renders the result into a centered page
container. switching away despawns the page; switching back re-renders.

rendering is one-shot (evaluate → render a static tree), not streaming. the cell
world owns a minimal `Host` whose only job in P1 is to satisfy the interpreter;
the page comes from the **result noun**, not from `emit` acts.

## layout (MVP — 1D centered)

the page container is a single vertically-stacked column, centered horizontally,
max-width bounded, on the prysm `DARK_BASE` background. spacing is multiples of
`g = 8pt`. typography uses the prysm scale (H1 32 / H2 24 / BODY 16 / CAPTION 14).
no 2D grid — `prysm/system/rs/grid.rs` stays stubbed until a later milestone.

## the first cell: landing

`cyb/cells/landing.rune` mirrors `cyb/landing/index.html` ("The Robot."):

- eyebrow:  `// from cyb`
- headline: `The Robot.`
- lead:     `A digital being that works for you. Thinks while you think. Earns
  while you sleep. Remembers what you forget. Yours forever.`
- price:    `from $1 · transferable · one-time`
- buy CTA:  `Get yours — $1`  → target `checkout`
- ghost CTA:`See what's inside` → target `anatomy`

P1 renders it with text/anno/button. P2 makes the headline a real H1, adds the
robot image, and spaces the sections.

## phase map

- **P1** cell world + one-shot renderer + `landing.rune` (text/anno/button)
- **P2** file-watch reload + `heading`/`image`/`section` primitives
- **P3** button → `CellAction` → navigate; add `checkout.rune`
- **P4** radio-backed `load_cell` (publish a particle to update the live app)
- **P5** `query` act → inf/cybergraph live data in a cell
