---
tags: cyber, cyb, core, spec
crystal-type: spec
crystal-domain: cyber
alias: .vocab, vocab format, cyb vocab spec
---

# .vocab — particle vocabulary in [[.cyb|format]]

a `.vocab` is the smallest .cyb container: one binary section holding a sorted list of [[hemera]] CIDs. it names a fixed set of particles, in a fixed order, with a fixed CID of its own. snapshots and models reference it by CID; multiple files can share a vocab without copying its bytes.

## why a separate format

vocabularies repeat. every snapshot of the same chain shares almost all of its particles with the previous snapshot. compiling each independently produces models with arbitrary, drifting token id assignments — particle `0xAA` is id 437 in one model, id 612 in the next.

defining vocab as a standalone, content-addressed file lets two snapshots share a vocab CID and produce models with identical id assignments for shared particles. it also lets a private graph compose its own particles on top of a public vocab without forking it.

## required sections

| name | format | what it does |
|------|--------|-------------|
| card | .md | what this vocab covers and where it came from |
| particles | .cids | sorted list of 32-byte hemera CIDs |

two sections. nothing else needed.

## frontmatter

```toml
[cyb]
types = ["vocab"]
name = "bostrom-23000000"

[[files]]
name = "card"
format = "md"

[[files]]
name = "particles"
format = "cids"
size = 93479200
```

`size = particle_count × 32`.

## card

```markdown
~~~card
# bostrom-23000000

vocabulary derived from the bostrom chain at block 23,000,000.
2,921,225 particles total. CID order is the order of first appearance
on chain (signal-block then intra-batch index).
license: cyber license.
```

## particles

raw concatenated CIDs, fixed 32-byte stride. no header, no length prefix — `size` from the frontmatter divided by 32 gives the count.

```
~~~particles
[0..32]      cid 0   (hemera hash, 32 B)
[32..64]     cid 1
[64..96]     cid 2
...
[(n-1)·32 .. n·32]   cid n-1
```

example (truncated CIDs):

```
0x1a2b3c4d...      ← position 0 = vocab id 0
0x5e6f7a8b...      ← position 1 = vocab id 1
0x9c0d1e2f...      ← position 2 = vocab id 2
...
```

position in the file is the vocab id. consumers mmap the section and binary-search by CID for `id ↔ cid` lookup in O(log n).

### ordering

CIDs may appear in any order the publisher chose, but the publisher commits to it: the file CID changes if the order changes. Two valid conventions:

- **first-appearance**: scan signals in chain order, append each unseen CID. Matches CT-1 Pass 1's natural order.
- **lexicographic**: sort CIDs ascending. Faster lookup, but loses any historical signal information.

The publisher picks one and notes it in the card.

## file identity

```
CID(.vocab) = hemera(file bytes)
```

a `.vocab` referenced by CID resolves to exactly one file. updating the vocab (adding CIDs, reordering) produces a new file with a new CID.

## composition

a single `.graph` may reference multiple `.vocab` files in `config.[[vocab]]`, in a declared order:

```toml
[[vocab]]
cid  = "0xaabbccdd..."
name = "bostrom-23000000"

[[vocab]]
cid  = "0xeeff0011..."
name = "mytoken-private"
```

the compiler concatenates them in declared order, deduping (first hit wins). particles found in `signals` but absent from every referenced vocab are appended at the end during compile. this gives composable vocab evolution: a private graph stacks its own particles on top of a public chain vocab without modifying either file.

## relation to .graph and .model

```
.vocab               .graph                       .model
──────               ───────                      ───────
particles[]   ◄──    config.[[vocab]].cid         vocab section (token id)
```

a `.graph` references its source vocab(s) by CID. `mc` reads the vocab(s), then signals, producing a `.model` whose token ids are stable across all compiles that share the same vocab refs.

## why one section, not zero

a vocab without a card is just a blob. the `card` is the only place to describe ordering convention, source, and intended use — without it, the file is opaque to anyone who didn't produce it.

## writing a vocab

```
hemera-cli vocab from-graph bostrom-23000000.graph -o bostrom-23000000.vocab
hemera-cli vocab from-list cids.txt -o custom.vocab
hemera-cli vocab merge a.vocab b.vocab -o ab.vocab
```

(reference CLI; not implemented yet.)

---

see [[.cyb|format]] for the base container. see [[cyb-graph]] for the snapshot format that references vocab. see [[cyb-model]] for the inference checkpoint that embeds the resolved vocab. see [[hemera]] for the hash function whose outputs fill the section.
