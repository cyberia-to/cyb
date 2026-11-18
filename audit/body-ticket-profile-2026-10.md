---
tags: cyb, audit, zheng, body, tickets, proofs
date: 2026-10-14
---
# body tickets on zheng 0.4: recomputation or a profile-1 proof

Proof-latency gate follow-up (soft3 `audit/proof-latency-2026-10.md`,
row 1). That audit read `master`, where `shell/src/worlds/body/prover.rs`
imports zheng's legacy `prove_spmv` without the `legacy` feature and does
not build against zheng 0.4. cyb#1407 (`feat/sound-proof-consumers`, this
branch's base) replaced it: a ticket is `cyb_core::ticket` — graph
commitment, seed, number, `digest(A·x)`, 80 bytes, checked by
recomputation, `TICKET_DIFFICULTY = 170`.

Stand `~/cyber/.stands/PW`: cyb `feat/sound-proof-consumers` 26efc5b0 +
this branch, zheng `release/0.4` aa54bdd, lens `release/0.2` d6dcc3f,
nox / joy / trident `release/0.4`, foculus `feat/device-profile-choice`
(foculus#66 on #61), cybergraph `feat/sound-proof-consumers`, bbg
`chore/coordinated-release-20260916`, honeycrisp `feat/tip5` (mir needs
it); other siblings at their main checkouts' HEADs. Apple M4 Max, rustc
1.95.0. **Load average 126–130 during the measurement** (other agents);
absolute times are inflated, the ratios are what the choice rests on.

## builds against zheng 0.4, no legacy

- `cargo tree -e features -i zheng` (whole workspace): zheng 0.4.0, feature
  `default` only. It reaches cyb through bbg, cybergraph and foculus; cyb
  itself has no zheng dependency (cyb-core takes it as a dev-dependency
  for the measurement below, default features).
- `cargo build -p cyb` (debug): builds.
- `Cargo.lock` gains two hunks, both forced by the stand: `zheng` under
  `cyb-core` (the dev-dependency of the measurement below), and
  `cyber-lens-rspcs` with `cyber-lens` depending on it. The second is not
  this branch's doing: lens `release/0.2` d6dcc3f adds `rspcs` as a
  workspace member and `src/Cargo.toml:30` makes `cyber-lens` depend on
  `cyber-lens-rspcs = { version = "0.1.0", path = "../rspcs" }`. Since the
  gate is `cargo check --tests --locked`, the lock has to carry it as long
  as lens is pinned at `release/0.2`; dropping the hunk red-lights the gate.
- `make fleet`: **GREEN, 21 ok, 0 failed** — 3 bodies, 38,497 / 39,052 /
  40,692 verified tickets in 45 s, chain synced past the blackhole, vault
  sealed, no orphans.

## should the subsidy carry a profile-1 ticket proof?

`cargo test --release -p cyb-core ticket_profile_one_lower_bound --
--include-ignored --nocapture` (CPU). The profile-1 row is a lower bound:
the SpMV relation alone, `y` committed, `x` pinned, `digest(y)` not bound
in the circuit.

| graph | recompute (prove = verify) | profile 1, relation only: ℓ · prove · verify · bytes |
|---|---|---|
| n=473, nnz=255 (default body graph) | 0.20 ms · 80 B | 2^9 · 58 ms · 4.2 ms · 12,761 B |
| n=18,571, nnz=10,000 | 7.5 ms · 80 B | 2^15 · 944 ms · 124 ms · 41,701 B |

The statistics are not the same on both sides of the table: the recompute
cell is the **mean of 5** calls with no warmup, while the profile-1 prove
and verify cells are **medians of 5**. Read the ratios below as
order-of-magnitude, not as a benchmark.

Choice: **no ticket proof — recomputation stays.**

1. The statement is fully public (graph, beacon seed, ticket number).
   A profile-1 verifier rebuilds the relation from the graph, so it needs
   the graph exactly as recomputation does. It verifies nothing a node
   without the graph could not verify.
2. Even the lower bound is 17–21× the cost of recomputation to verify
   (4.2 / 0.20 and 124 / 7.5 from the table, and the profile-1 verifier
   pays the relation rebuild on top), and 125–300× to prove.
3. A real ticket proof must bind `digest(y)` (the win test reads it) in
   the circuit: about 2 hemera permutations per element of `y`
   (~1,500 constrained wires each). That reaches zheng's 2^20 relation
   cap at a few hundred nodes, so profile 1 cannot hold a body graph's
   ticket at all.

### the recompute cell disagrees with TICKET_DIFFICULTY

0.20 ms above is **not** the 0.58 ms that `core/src/ticket.rs:178` cites
for the same default body graph on the same M4 Max when it calibrates
`TICKET_DIFFICULTY = 170` (97.6 ms legacy / 0.58 ms recomputed, cyb#1407).
Both figures call the same `prove` in release mode on the same graph. What
differs between them is only the sampling: cyb#1407's
`ticket_cost_by_graph_size` averages 784 warmed iterations, while
`ticket_profile_one_lower_bound` here averages 5 with no warmup — and the
newer figure is the one taken at load average 126–130, which can only
inflate a time, yet it is 2.9× faster. Neither difference explains the gap
in that direction, so one of the two numbers is wrong and this page does
not claim to know which.

Why it matters: the ratio sets the subsidy a body earns per minute and so
PUSSY issuance on the test chain. If 0.20 ms is the true recomputation
cost, the legacy ratio is 97.6 / 0.20 ≈ 490, not 170; a counted ticket is
then ~2.9× cheaper than the retired zheng ticket and a body earns its
subsidy ~2.9× faster than before the switch. **`TICKET_DIFFICULTY` is
flagged for recalibration** on an unloaded machine, with the same
statistic on both measurements, before the per-minute subsidy is trusted.
That recalibration is its own change; it does not touch the choice above,
which rests on arguments 1 and 3 and on a verify ratio that stays above 1
under either figure (4.2 / 0.58 ≈ 7× at the low end).

What the subsidy actually lacks is different: the node credits
`zheng -> pussy` with weight = tickets on the body's word. Making that
checkable needs the graph committed in state, with tickets verified
against that commitment. That means a state statement (`prove_state` with
look evidence), or accumulation of SpMV steps (proof repair, phase 3),
not a one-relation proof. Owner decision, unchanged by this branch.

## harness fix

`harness/fleet.sh` cleanup ran `pkill -f "cyb-fleet"`. That kills every
fleet on the machine, including other worktrees' and other agents'. It now
kills only processes whose command line names this run's unique work dir.
