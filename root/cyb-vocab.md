---
tags: cyber, cyb, core, spec
crystal-type: spec
crystal-domain: cyber
alias: .vocab, vocab format, cyb vocab spec
---

# .vocab — particle dictionary in [[.cyb|format]]

a `.vocab` is the canonical [[particle]] → data dictionary in cyber. each entry is one particle (its hemera hash) paired with the bytes that hash to it. the file is itself a particle: its identity is `hemera(file bytes)`. snapshots and models reference `.vocab` files by particle to share content without copying it.

## what .vocab does

a particle is a content-addressed identifier — 32 bytes of hemera hash. on its own, a particle tells you nothing about what it points to. `.vocab` is the resolution: given a particle, you get back the data it digests.

```
particle 0x1a2b3c4d…   ─►  .vocab  ─►  raw bytes  ─►  hemera(bytes) = 0x1a2b3c4d…
```

a .vocab packs many such mappings into one file so consumers don't fetch them one at a time, and so a snapshot or model can reference an entire dictionary by one particle (the .vocab's own).

## why a separate format

dictionaries repeat. every snapshot of the same chain shares almost all of its particles with the previous snapshot. inlining the same data into every snapshot copies the same bytes for nothing.

defining `.vocab` as a standalone content-addressed file lets two `.graph` snapshots reference the same vocab particle and share its bytes. it also lets a private graph stack its own particles on top of a public chain vocab without forking either.

## required sections

| name | format | what it does |
|------|--------|-------------|
| card | .md | what this vocab covers and where it came from |
| particles | .particles | particle → data entries |

two sections.

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
format = "particles"
size = 1832947104
```

## card

```markdown
~~~card
# bostrom-23000000

particle dictionary derived from the bostrom chain at block 23,000,000.
2,921,225 particles, 1.7 GB of content. ordering: first-appearance on
chain (signal-block then intra-batch index).
license: cyber license.
```

## particles

variable-length entries. each entry is one particle followed by its data.

```
~~~particles:
  [0..4]                  n  particle count (u32 LE)
  repeated n times:
    [0..32]               particle  (hemera hash, 32 B)
    [32..40]              len       (u64 LE — bytes of data following)
    [40..40+len]          data      (raw bytes; hemera(data) = particle)
```

`len = 0` is valid — a length-zero entry registers the particle's existence in the vocab without bundling its data. consumers wanting the data fetch it from another `.vocab`, an external content store, or a `~~~particles` extension in a `.graph`.

example (truncated):

```
n = 3

particle 0 = 0x1a2b3c4d...
len 0      = 27
data 0     = "wiki: a collaborative…"

particle 1 = 0x5e6f7a8b...
len 1      = 4
data 1     = "BOOT"

particle 2 = 0x9c0d1e2f...
len 2      = 0      ← registered, no data inline
```

position of an entry in the file is its vocab id (0, 1, 2, ...). entry `i` is reachable in O(log n) via a side-table or in O(n) via linear scan.

## ordering

entries appear in any order the publisher chose; the publisher commits to it. the file's particle changes if the order changes. two valid conventions:

- **first-appearance**: scan signals in chain order, append each unseen particle. matches the natural id assignment in CT-1 Pass 1.
- **lexicographic**: sort particles ascending. faster binary search but loses any historical signal information.

the publisher picks one and notes it in the card.

## file identity

```
particle(.vocab) = hemera(file bytes)
```

a `.vocab` referenced by particle resolves to exactly one file. updating the dictionary (adding entries, reordering) produces a new file with a new particle.

## composition

a single `.graph` may reference multiple `.vocab` files in `config.[[vocab]]`, in declared order:

```toml
[[vocab]]
particle = "0xaabbccdd..."
name     = "bostrom-23000000"

[[vocab]]
particle = "0xeeff0011..."
name     = "mytoken-private"
```

the compiler walks them in declared order, deduping (first hit wins). particles found in `signals` but absent from every referenced vocab are appended at the end during compile. composable evolution: a private graph stacks its own particles on top of a public chain vocab without modifying either file.

## relation to .graph and .model

```
.vocab                .graph                       .model
──────                ───────                      ───────
particles[]    ◄──    config.[[vocab]].particle    vocab section (token id + data)
                ┃
                ┗──   ~~~particles (inline)        same layout as .vocab
```

a `.graph` references its source vocabs by particle. it may also include a `~~~particles` extension whose layout is identical to the `.vocab` `particles` section — inline data for entries the publisher wants to ship in-file.

`mc` reads the vocabs (external + inline), then signals, producing a `.model` whose token ids are stable across all compiles that share the same vocab refs.

## why two sections, not one

a vocab without a card is a blob. the card is the only place to declare ordering convention, source, and intended use — without it, the file is opaque to anyone who didn't produce it.

## writing a vocab

```
hemera-cli vocab from-graph bostrom-23000000.graph -o bostrom-23000000.vocab
hemera-cli vocab from-list particles.txt           -o custom.vocab
hemera-cli vocab merge a.vocab b.vocab             -o ab.vocab
```

(reference CLI; not implemented yet.)

---

see [[.cyb|format]] for the base container. see [[cyb-graph]] for the snapshot format that references vocab. see [[cyb-model]] for the inference checkpoint that embeds the resolved vocab. see [[hemera]] for the hash function whose outputs identify particles. see [[particle]] for the cyber-native node concept.
