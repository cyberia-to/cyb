---
title: anatomy
tags: cyb, core
alias: robot anatomy, anatomy of the robot, cyb anatomy
crystal-type: reference
crystal-domain: cyb
---
# anatomy — the source of truth

the robot is cy with a body. the agent's sixteen organs are specified in [[cy/specs/anatomy|cy]]; this page keeps the organs that need a screen, a camera or a speaker, and the parts that belong to neither yet. every surface of the body — the app's worlds, the docs, [cyb.ai](https://cyb.ai), the crates — names things by this page and by [[cy/specs/anatomy|cy anatomy]]. a page that cannot be traced to a part is either legacy or component matter.

## I · identity — who

| part | is | today |
|---|---|---|
| **[[cyb/parts/name|name]]** | the NFT resolver — the robot's name resolves through the graph ([[moon-passport]] lineage), owned like a token, not set in a config | ports with the soft3 genesis |
| **[[cyb/parts/avatar|avatar]]** | the robot's visualization — its model, the rendered creature others see and you recognize | to grow; the robot world's creature is its seed |

## II · mind — what thinks

| part | is | today |
|---|---|---|
| **[[cyb/parts/brain|brain]]** | the rendered graph | live as the world currently named *graph* (mir, 100+ fps) → renames to **brain** |

## III · senses and speech — what perceives and says

| part | is | today |
|---|---|---|
| **[[cyb/parts/vision|vision]]** | sight — camera, screen, world | to grow |

## IV · value — what it holds

| part | is | today |
|---|---|---|

## V · time — what it did and will do

| part | is | today |
|---|---|---|
| **[[cyb/parts/time|time]]** | one screen: log ← **now** → plan, the present in the middle | to build; the flagship view |

## VI · flesh — what it is made of

| part | is | today |
|---|---|---|
| **[[cyb/parts/cells|cells]]** | merged into [[neuron]] on 2026-09-12: a live-loaded rune program is a neuron program; the robot grows abilities as programs of its neuron, not as a separate organ | folded — [[soft3/roadmap/neuron-cell-convergence|neuron + cell]]; the page stays as the record. ⚠ distinct from the protocol's [[cell]] (a 4D particle group) |
| **[[cyb/parts/radio|radio]]** | the physical layer of communication with external networks; [[sense]] speaks over radio | live — the cyber-radio transport; the wire obeys the graph: follow/antenna/socket are cyberlinks |

## deliberately absent

**core.** we looked for a hidden lower layer and found none. the robot's immortality needs no extra organ — it is the backup: *name + soul + vault seed + log*. carry those four to any machine and the robot resurrects. the `cyb-core` crate keeps its name as code, not as anatomy.

## banned words

*wallet* → [[cyb/anatomy|sigma]] · *oracle, portal, old brain* → legacy (the JS era) · *graph (as a world name)* → brain · *face* → avatar

## alignment phases

1. **docs** — [[cyb/product/robot|robot]], [[cyb/product/spec|spec]], [[cyb/product/product|product]], [[cyb/decide/os|os]] restate themselves as expansions of parts; anything untraceable goes legacy (per [[restructure]])
2. **app** — world *graph* → *brain*; com becomes chrome; *now* indicator; *time* world from tade + standing orders
3. **landing** — cyb.ai lists the anatomy, one line per part
4. **code** — crates and modules adopt part names; `~/cyb/soul` file is born

the parts are the contract. the order of growth is the roadmap's business — but the names are settled here.
