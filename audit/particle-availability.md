---
title: particle availability audit
tags: cyb, audit, availability
---
# particle availability audit

property 16 of [[cyber/launch]]: every particle the chain references
resolves in cyb, from the local store, the burial blockstore, or a peer.
the 97.63% baseline from the bostrom burial is the floor; the missing
list is public.

date: 2026-09-21 · revision: 4984387d

## what exists now

`shell/src/worlds/availability.rs` audits a set of referenced particle
hashes against every source cyb can resolve today:

- the local content store (`content::load()`, `~/cyb/particles.jsonl`)
- sigma's ASCII-padded names, which carry their own label with no lookup

`cargo test -p cyb --lib availability` — 5 tests, all pass:

```
running 5 tests
test worlds::availability::tests::empty_set_resolves_fully_by_convention ... ok
test worlds::availability::tests::unresolved_particle_is_missing ... ok
test worlds::availability::tests::store_hit_resolves ... ok
test worlds::availability::tests::ascii_named_particle_resolves_without_store ... ok
test worlds::availability::tests::mixed_set_reports_fraction_and_missing_list ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 28 filtered out
```

`AvailabilityReport::resolved_fraction()` is the number this page's
registry row measures against; `.missing` is the list.

## what this does not measure yet

neither source above is the burial blockstore or a peer. running
`audit_local` over the 3,143,650 particles the bostrom graph references
would show a resolution rate near zero today, not because the bytes are
gone but because cyb has no path to them yet — that path is properties
22 (blob fetch by particle over radio) and 23 (a file store answering by
particle on the three network machines), both still open. this module is
the instrument that will read the real number once those land; it is not
itself the fetch path.

## remains

- wire an `audit_local(&BrainIndex.hashes)` call into a CLI or scheduled
  check once radio blobs (22) and the file store (23) exist, and run it
  over the real burial vocabulary
- extend `Resolution` with a `BurialBlockstore` and `Peer` variant when
  those sources exist, so the report attributes correctly instead of
  lumping every real hit under `local_store`
- publish the resulting missing list per the property's evidence
  requirement
