---
title: cyb clean-checkout gate
tags: cyb, audit, launch
---
# cyb clean-checkout gate

Property #39 — every phase-1 component builds and tests from a clean
checkout of its default branch against the default branches of its
siblings. Measured for cyb at revision `6c205592` (origin/master), worktree
only.

## first run: a real red test, not a flake

```
$ cargo test
...
Running unittests src/lib.rs (target/debug/deps/cyb_core-cc72e0bcc833f7c3)
test money::tests::light_fold_tip_open_and_advance ... FAILED
thread 'money::tests::light_fold_tip_open_and_advance' panicked at core/src/money.rs:1040:71:
called `Result::unwrap()` on an `Err` value: OpeningUnverified
```

Deterministic across three consecutive runs, not timing-dependent.

## root cause

`MoneyWallet::open_balance` called `bbg::prove_balances` then
`bbg::verify_query`. In bbg's current design (`rs/src/proof.rs` — "Private
balance/A openings stay contextless and disclose no additional table"),
`prove_balances` never sets `QueryProof::context`, and `verify_query`'s
`authenticated()` rejects any contextless proof outright
(`proof.context.as_ref()?`). So `open_balance` could never succeed against
current bbg — not just in this test, for any caller. This is a genuine
API mismatch, not a bad test: bbg exposes a separate opt-in disclosure
pair for exactly this light-client case, `prove_public_balance` /
`verify_public_balance` (`rs/src/proof.rs:263`, `rs/src/query_auth.rs:186`),
which `open_balance` was not using.

## fix

`core/src/money.rs`: `open_balance` now calls `prove_public_balance` and
verifies with `verify_public_balance(&proof, &self.tip.root, owner, token)`,
returning the amount `verify_public_balance` itself discloses instead of
reading local state directly — the point of a light-client-verified
opening. No other caller of `open_balance` exists outside its own tests.

## verified, cyb at 6c205592 + the fix above

```
$ cargo check --tests
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.70s
$ cargo test
test result: ok. 27 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out   # cyb (shell)
test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out   # cyb-core
$ bash harness/fleet.sh
fleet: 21 ok, 0 failed
fleet: GREEN
```

No warnings, no errors, beyond the pre-existing future-incompatibility
notices from `block v0.1.6` and `proc-macro-error2 v2.0.1` (transitive,
unrelated to this change).

## remains

This closes cyb's own slice of row 39. `bbg::proof::prove_public_balance`
and `bbg::query_auth::verify_public_balance` are not re-exported from
`bbg`'s crate root (`rs/src/lib.rs`); this PR reaches them by module path
instead of asking bbg to change its public surface, which would be a
separate, bbg-owned decision.
