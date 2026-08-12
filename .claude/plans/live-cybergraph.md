# Milestone: the living cybergraph — rendered from real interaction

## goal

The Graph world stops showing a synthetic 28-edge demo. It shows a **real cybergraph
that grows from what the user does** — every terminal command, its output, every
world/cell navigation becomes particles and cyberlinks in an actual `Cybergraph`
(bbg-backed, in-memory), which `mir` lays out and renders. Interact → the graph fills
with your session.

## the idea

```
interaction (terminal cmd+output, world/cell nav)
   → emit a cyberlink into a shared live Cybergraph (real bbg state, no proofs)
   → enumerate graph → build mir Csr + ParticleIndex (+ hash→label map)
   → mir lays out (spectral) and renders
```

Because the store is a real `Cybergraph`, the same graph is **inf-queryable** for free
(`graph.query("?[cid,energy] := particles{cid,energy}")`) — P5 of the cell milestone
lands almost as a side effect.

## what exists (verified, file:line)

- `mir` render: `GraphWorldConfig { graph: Arc<Csr> }` (`mir/src/bevy/resources.rs:20`),
  consumed in `on_enter_graph` (`mir/src/bevy/world.rs:23`) which spawns a **one-shot**
  `EpochWorker` (`mir/src/epoch/mod.rs:40`). CSR is immutable.
- mir graph types: `Cyberlink{neuron,from,to,token,amount,valence,block}`
  (`mir/src/graph/snapshot.rs:26`), `ParticleIndex` hash→idx (`mir/src/graph/vocab.rs`),
  `Csr::build(links, index)` (`mir/src/graph/adjacency.rs:30`).
- synthetic source: `cyb/shell/src/worlds/graph.rs::build_synthetic_csr()` (lines 40-76).
- cybergraph store: `Cybergraph::new()`, `.link(Signal)`, `.query(inf)`, enumerate via
  `bbg.state.particles / axons_out / axon_edges` (`cybergraph/src/api.rs`, `bbg/rs/src/state.rs`).
- signal build: `cyb_core::SignalBuilder::new(neuron).link(from,to,token,amount,valence).build()`;
  particle id = `hemera::hash(bytes)`.
- interaction hooks: terminal `poll_eval_results` (`terminal/mod.rs:420`, has chunks +
  done_error + command history), `process_keyboard_input` Enter (~541); nav `nav.rs:12`,
  `chrome.rs::handle_chrome_input` (~360), `hotkeys.rs`.

## what's missing

- a shared **live graph resource** in the shell.
- an **interaction → cyberlink** emitter (Bevy Event) + a drain system that applies links.
- a **graph → Csr** builder over the real cybergraph (replacing the synthetic one) + a
  **hash→label** map for node labels.
- **live recompute** in mir (it lays out once) — for "grows before your eyes".

## decisions (recommend)

1. **Store = real `Cybergraph::new()`** (bbg-backed, in-memory, `proof:None`), not a
   render-only adjacency. Keeps it genuinely "the cybergraph" and inf-queryable. Carry a
   side `HashMap<Particle,String>` for display labels (bbg stores hashes, not text).
2. **Session semantics — which links** (small, legible set):
   - terminal: `prev-cmd → cmd` (temporal chain), `cwd → cmd`, `cmd → output` (summary node)
   - nav: `from-world → to-world`, `world → cell`
   amount = repetition/conviction (accumulates on repeat); valence = +1.
3. **G1 before G2.** G1 (rebuild-on-enter) needs **zero mir changes** — every time you
   switch to the Graph world it rebuilds the CSR from the accumulated cybergraph and
   re-runs layout, so you see your session graph. G2 adds a recompute channel to mir's
   EpochWorker so it grows while you watch. Ship G1 first (de-risks the mir surgery).

## phases

### G1 — session graph, rebuilt on enter (no mir changes)
- Add resource `LiveGraph { graph: Cybergraph, labels: HashMap<Particle,String> }`
  (shell), inserted at Startup with a couple of seed nodes (so it's never empty).
- Add `#[derive(Event)] Link { from: String, to: String }` (label-addressed). Emit from:
  - `poll_eval_results`: `cmd → output`, `prev-cmd → cmd`, `cwd → cmd`
  - `nav` / `chrome` / `hotkeys`: `from-world → to-world`, `world → cell`
- Drain system: for each `Link`, hash labels → particles, record labels, build a Signal,
  `graph.link(signal)`.
- Replace `build_synthetic_csr()` with `build_csr(&LiveGraph)`: enumerate axon_edges →
  mir `Cyberlink`s → `ParticleIndex` + `Csr`. Rebuild `GraphWorldConfig.graph` at
  `OnEnter(Graph)` from `LiveGraph` so each entry shows the latest.
- **Deliverable:** run commands + switch worlds, press Cmd+1 → graph shows your session.

### G2 — live recompute (grows before your eyes) [mir change]
- Add a recompute channel to `EpochWorker` (`recompute_tx: Sender<Arc<Csr>>`); worker
  loops on rx and re-runs `epoch_pipeline` on each new CSR (Procrustes already aligns to
  the previous epoch, so nodes animate into place).
- Debounced shell system: when `LiveGraph` changed, rebuild CSR, send to mir.
- **Deliverable:** type in the terminal, watch nodes/edges appear in the Graph world live.

### G3 — labels + polish (optional, after G1/G2)
- Render node labels (hash→label) in mir or as prysm overlays; color by kind
  (command/output/world/cell); edge weight = repetition. Click a node → inf query.

## risks
- **mir is one-shot** — G2's recompute channel is the one real engine change; keep it
  isolated to `EpochWorker`. G1 sidesteps it entirely.
- **graph can grow unbounded** — cap/age out nodes for the live view (note any cap).
- **bbg Signal ordering** — links are per-neuron ordered; use a single synthetic session
  neuron and a monotonic step so `graph.link` accepts them.
- **labels are not in bbg** — must carry the hash→label map ourselves; it is the only
  source of human-readable node names.

## outcome
The graph becomes a mirror of the session: a real, inf-queryable cybergraph built from
the user's own terminal and app actions, laid out and rendered by mir.
