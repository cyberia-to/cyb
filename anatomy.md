---
title: anatomy
tags: cyb, core
alias: robot anatomy, anatomy of the robot, cyb anatomy
crystal-type: reference
crystal-domain: cyb
---
# anatomy — the source of truth

the robot is one organism with **21 parts in six layers**. every surface — the app's worlds, the docs, [cyb.ai](https://cyb.ai), the crates — names things by this page. a page that cannot be traced to a part is either legacy or component matter.

## I · identity — who

| part | is | today |
|---|---|---|
| **[[cyb/parts/name|name]]** | the NFT resolver — the robot's name resolves through the graph ([[moon-passport]] lineage), owned like a token, not set in a config | ports with the soft3 genesis |
| **[[cyb/parts/avatar|avatar]]** | the robot's visualization — its model, the rendered creature others see and you recognize | to grow; the robot world's creature is its seed |
| **[[cyb/parts/soul|soul]]** | the main config: one file defining processing when the robot is asked — which model, which dialect, what it may do unasked | `~/cyb/soul` to define; soma settings are its embryo |
| **[[cyb/parts/ward|ward]]** | the enforcer of the soul: the permission boundary every runtime's act passes through — holds the caps, performs or refuses, prompts the neuron | live — the effect router; doctrine at [[cyb/parts/ward|ward]] |

## II · mind — what thinks

| part | is | today |
|---|---|---|
| **[[cyb/parts/soma|soma]]** | mind and model: local inference, weights, silicon | live — soma kernel + honeycrisp, `? q` in com |
| **[[cyb/parts/brain|brain]]** | the rendered graph | live as the world currently named *graph* (mir, 100+ fps) → renames to **brain** |
| **[[cyb/parts/memory|memory]]** | files rendered as a file system — table or tiles, tap to read | half-live: tap-to-read pages + [[cyb/parts/fs|fs]] become its spec; a projection of brain, one key flips brain ⇄ memory |

## III · senses and speech — what perceives and says

| part | is | today |
|---|---|---|
| **[[cyb/parts/com|com]]** | the commander — the one line where the user types; the mouth of the robot, reads [[cyb/anatomy|soul]] on every ask | live (com world); becomes omnipresent chrome, not a world |
| **[[cyb/parts/sense|sense]]** | the messenger — interaction with other neurons, files, robots | seed exists (`money_to_sense`, notices); grows into the robot's inbox/outbox |
| **[[cyb/parts/voice|voice]]** | speech — the robot heard and speaking | to grow |
| **[[cyb/parts/vision|vision]]** | sight — camera, screen, world | to grow |
| **[[cyb/parts/state|state]]** | verified perception of external networks: every answer is a value **plus a tier** (T0 proof-verified · T1 anchor-verified · T3 unproven RPC, badged); one query IR, a proof-router, local verification over provider truth | doctrine written — [[cyb/parts/state|state]] |

## IV · value — what it holds

| part | is | today |
|---|---|---|
| **[[cyb/parts/sigma|sigma]]** | not a wallet: the sum of all tokens in possession, and the management of neurons (identities) | live (sigma world); gains the neurons screen; the word *wallet* is banned |
| **[[cyb/parts/vault|vault]]** | secrets and sleeping neurons: keys, mnemonics, TOTP | live — XChaCha20 under mnemonic + TOTP; sigma spends, vault holds and signs |

## V · time — what it did and will do

| part | is | today |
|---|---|---|
| **[[cyb/parts/log|log]]** | the history of every interaction | live in substance: the durable tade `~/cyb/graph.log` *is* the log |
| **[[cyb/parts/now|now]]** | the context — the file the robot stands on, and the hinge of everything: what [[cyb/parts/com|com]] acts on by default, what [[cyb/parts/soma|soma]] packs into the model's window, where casts attach, where [[cyb/parts/brain|brain]] and [[cyb/parts/memory|memory]] stand, the center [[cyb/parts/time|time]] pivots on | partial — the app tracks a current particle; the organ with its many functions is to build |
| **[[cyb/parts/plan|plan]]** | the schedule: standing orders, deferred intents | seed exists (mining standing order); generalizes |
| **[[cyb/parts/time|time]]** | one screen: log ← **now** → plan, the present in the middle | to build; the flagship view |

## VI · flesh — what it is made of

| part | is | today |
|---|---|---|
| **[[cyb/parts/body|body]]** | the physical body: silicon, sensors, energy, mining — telemetry and resources of the machine the robot lives on | live (default world: telemetry, erga child, PUSSY/day) |
| **[[cyb/parts/cells|cell]]** | an organ-extension: a live-loaded program that grows the robot a new ability | live (rune cell runtime). ⚠ distinct from the protocol's [[cell]] (a 4D particle group) — the robot grows *cells as organs* |
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
