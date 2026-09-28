---
title: cells
tags: cyb, core
alias: cell runtime, rune cells, live-loaded pages
crystal-type: entity
crystal-domain: cyb
status: merged into neuron
---
# cells — merged into neuron

cells were the robot's organ-extensions: live-loaded rune programs that grew a new ability without rebuilding the binary, each one a page authored in rune, decoded by [[prysm]], rendered by [[mir]]. on 2026-09-12 the standalone cell subject was folded into [[neuron]]: one protocol subject with its optional durable execution model, the bounded rune machine, the canonical data codecs and the graph transaction port. the decision and the migration ledger are in [[soft3/roadmap/neuron-cell-convergence|neuron + cell]]; the contract that replaced the runtime cell is [[neuron/specs/runtime-v1|runtime v1]].

what survives, and where it lives now:

- a live-loaded rune program is a neuron program: data IDs, independent state, no second signing identity ([[neuron]]).
- a rune expression that evaluates to a chunk-noun and renders as a page is the terminal's own pipeline, `rune → chunks → prysm → mir`; the robot keeps it as a way to paint, not as an organ.
- `cell://<name>` addressing, file-watched reload and radio-backed publishing were the cell roadmap's P2–P4; their home is the neuron program lifecycle and [[cy/specs/memory|memory]] for what is rendered.

the word *cell* in the protocol ([[cell]], a 4D particle group; the [[cell]] ladder of [[aos]] and [[oikos]]) is a different thing and is unaffected.
