---
title: live
tags: cyb, core
alias: non-blocking, never wait, last known
crystal-type: principle
crystal-domain: cyb
---
# live — never wait on the network to paint

a world opens on **what it already knows**. a fetch runs in the background.
the neuron can leave before the answer arrives. the surface never blanks,
never says "loading", never holds the frame for a socket.

this is the same lesson as hung GETs freezing every chain: a slow neighbour
may only stall its own thread. here it is a law of the shell, not just of
probes.

## the rule

1. **paint last known** — a number, a table, a graph, a page. zero is a
   number. an empty page is a page. "querying…" is a lock.
2. **fetch off the frame** — another thread, a timeout, one in-flight at
   a time. the UI thread never calls out.
3. **tabs always work** — entering a world does not own the input. leaving
   cancels nothing the user can see; the request may finish in the dark
   and the next visit shows it.
4. **errors are receipts, not curtains** — a failed fetch writes a dim
   line under the last good value. it does not replace the value with
   the error.

sigma's balance is the first instance: cached `~/cyb/sigma-chain.json` on
the way in, a silent `/balance` behind it. oracle's block list, body's
network rows, soma's answer stream follow the same shape.

see [[cy/specs/sigma]], [[cy/specs/state]] (T3 never pretends to be
proof), body's `networks.rs` (timeouts, one thread per chain).
