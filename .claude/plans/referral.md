# Referral — Dunbar-decay referrer share in the money loop

Date: 2026-08-31. Spec: `~/cyber/cyber/specs/referral.md` (written first,
same session). Survey of prior art: `~/cyber/.claude/plans/2026-08-31-reward-referral-survey.md`.

## Model (agreed in session)

- One referral pool: 10% of a referee's settled reward (matches li's slot).
- Referrer share decays hyperbolically with active referees n:
  `share_micros(n) = max(10_000, 100_000 * 150 / (150 + 9n))`
  (micros, li convention). 10% at n=0, exactly 1% at n=150 (Dunbar), flat after.
- Hyperbola, not linear: keeps referrer income n*share strictly increasing
  (win-win). Linear to 1% at 150 breaks monotonicity past n≈83.
- Witness gate: only witnessed (personhood-attested) referrers get the
  curve; unwitnessed earn the flat floor. Kills the identity-split attack
  (splitting into fresh ids would otherwise pay +82% at n=150).
- Activity: a referee counts while its last settled reward is within
  `window` epochs (default 30).
- Bind-once at first contact (cyb-boot: referrer arrives in boot.dat and
  is registered on first sync). Self-referral and cycles rejected.

## Work plan

1. [x] Spec `cyber/specs/referral.md` — scientific form: model, curve
   derivation, monotonicity + sybil theorems, protocol wiring, parameters.
2. [x] `cyb/core/src/referral.rs` — curve, registry (bind/active/witness),
   split; unit tests (curve endpoints, monotone income, bind rules,
   window, witness gate, rounding).
3. [x] Wire into `cyb/core/src/money.rs::apply_settle_receipt` — the single
   mint point all settle paths share: mark referee active, split, mint net
   to self, credit referrer balance + tok ledger leg, emit
   `MoneyEvent::ReferralAccrued`.
4. [x] Update exhaustive `MoneyEvent` matches: `shell/src/worlds/sigma/mod.rs`,
   `cli/src/main.rs`; explicit arm in `core/src/sense.rs` (referrer
   notification kind "referral").
5. [x] Exports in `core/src/lib.rs`.
6. [x] Integration test in money.rs: bind + attest → link_and_settle →
   referrer credited, net minted; unwitnessed → floor cut.
7. [x] `cargo check -p cyb-core && cargo test -p cyb-core`; `cargo check -p cyb`
   (shell) + cli. Zero warnings.
8. [x] Commits: cyber (docs: spec), cyb (feat: referral). No push (repo rule).

## Out of scope (follow-ups)

- Real witness attestation verify (moon-passport extension / attested-genome
  nullifier) — `attest_witness` is the mount point, trusted-local for now.
- Referrer-side clock-B escrow parity (referrer credit is immediate).
- cyb-boot boot.dat referrer → automatic `bind` on first sync.
- Cashback variant (pool minus referrer share to the referee) — spec §7
  records it as an option; not wired.
