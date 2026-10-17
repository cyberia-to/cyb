//! Referral — Dunbar-decay referrer share on settle rewards.
//!
//! Implements cyber/specs/referral.md: bind-once referrer, hyperbolic
//! share curve from the 10% pool for a newcomer to the 1% floor at
//! Dunbar scale, witness gating, activity window. The split applies to
//! a neuron's settled share in [`crate::MoneyWallet::apply_settle_receipt`].

use std::collections::{BTreeMap, BTreeSet};

use cybergraph::NeuronId;

/// Referral pool — newcomer referrer share, in micros (10^6 = 100%).
pub const POOL_MICROS: u64 = 100_000;
/// Floor share a referrer converges to at Dunbar scale (micros).
pub const FLOOR_MICROS: u64 = 10_000;
/// Dunbar scale: active referees at which the curve meets the floor exactly.
pub const DUNBAR: u64 = 150;
/// Micros denominator.
pub const MICROS: u64 = 1_000_000;
/// Activity window in epochs: a referee counts while its last settled
/// reward is at most this many epochs old.
pub const DEFAULT_WINDOW: u64 = 30;
/// Curve slope POOL/FLOOR − 1, forced by s(0) = POOL and s(DUNBAR) = FLOOR.
const SLOPE: u64 = POOL_MICROS / FLOOR_MICROS - 1;

/// Witnessed-referrer share for `n` active referees (micros):
/// `s(n) = max(FLOOR, POOL·DUNBAR / (DUNBAR + SLOPE·n))`.
///
/// Hyperbolic decay keeps referrer income `n·s(n)` strictly increasing
/// (win-win), reaches the floor exactly at `n = DUNBAR` and stays flat.
pub fn share_micros(active_referees: u64) -> u64 {
    let denom = DUNBAR + SLOPE.saturating_mul(active_referees);
    (POOL_MICROS * DUNBAR / denom).max(FLOOR_MICROS)
}

/// Binding rejections (specs/referral.md §6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferralError {
    SelfReferral,
    AlreadyBound,
    Cycle,
}

/// Result of splitting a settled reward between referee and referrer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReferralCut {
    pub referrer: NeuronId,
    /// Referrer cut (rounds down).
    pub cut: u64,
    /// Referee net share.
    pub net: u64,
    /// Share applied, in micros.
    pub share_micros: u64,
}

/// Referral registry for one cell: bind-once edges, activity marks,
/// witness set. BTree maps keep iteration deterministic.
#[derive(Clone, Debug, Default)]
pub struct Referral {
    /// referee → referrer. First binding wins, forever.
    bound: BTreeMap<NeuronId, NeuronId>,
    /// referrer → direct referees.
    referees: BTreeMap<NeuronId, BTreeSet<NeuronId>>,
    /// neuron → epoch of its last settled reward.
    last_active: BTreeMap<NeuronId, u64>,
    /// Witnessed neurons — the full curve; everyone else earns the floor.
    witnessed: BTreeSet<NeuronId>,
    /// Activity window in epochs.
    pub window: u64,
}

impl Referral {
    pub fn new() -> Self {
        Self {
            window: DEFAULT_WINDOW,
            ..Default::default()
        }
    }

    /// Bind `referee` to `referrer` — first-sync semantics: once set, the
    /// edge never changes. Rebinding to the same referrer is a no-op.
    pub fn bind(&mut self, referee: NeuronId, referrer: NeuronId) -> Result<(), ReferralError> {
        if referee == referrer {
            return Err(ReferralError::SelfReferral);
        }
        if let Some(existing) = self.bound.get(&referee) {
            return if *existing == referrer {
                Ok(())
            } else {
                Err(ReferralError::AlreadyBound)
            };
        }
        // Walk the upline from the referrer; meeting the referee closes a loop.
        let mut cur = Some(&referrer);
        while let Some(node) = cur {
            if *node == referee {
                return Err(ReferralError::Cycle);
            }
            cur = self.bound.get(node);
        }
        self.bound.insert(referee, referrer);
        self.referees.entry(referrer).or_default().insert(referee);
        Ok(())
    }

    pub fn referrer_of(&self, referee: &NeuronId) -> Option<NeuronId> {
        self.bound.get(referee).copied()
    }

    /// Record a settled reward for `neuron` at `epoch` — the activity mark.
    pub fn mark_active(&mut self, neuron: NeuronId, epoch: u64) {
        let e = self.last_active.entry(neuron).or_insert(epoch);
        if *e < epoch {
            *e = epoch;
        }
    }

    /// Direct referees of `referrer` active within the window at `epoch`.
    pub fn active_referees(&self, referrer: &NeuronId, epoch: u64) -> u64 {
        let Some(set) = self.referees.get(referrer) else {
            return 0;
        };
        set.iter()
            .filter(|r| {
                self.last_active
                    .get(*r)
                    .is_some_and(|last| epoch.saturating_sub(*last) <= self.window)
            })
            .count() as u64
    }

    /// Admit `neuron` to the full curve. Mount point for a witness
    /// attestation verify (moon-passport extension / attested-genome
    /// nullifier); trusted-local until that lands (specs/referral.md §6).
    pub fn attest_witness(&mut self, neuron: NeuronId) {
        self.witnessed.insert(neuron);
    }

    pub fn is_witnessed(&self, neuron: &NeuronId) -> bool {
        self.witnessed.contains(neuron)
    }

    /// Effective share for `referrer` at `epoch`: the Dunbar curve when
    /// witnessed, the flat floor otherwise — splitting a branch across
    /// fresh unwitnessed identities earns nothing extra (theorem 2).
    pub fn share_micros_of(&self, referrer: &NeuronId, epoch: u64) -> u64 {
        if self.is_witnessed(referrer) {
            share_micros(self.active_referees(referrer, epoch))
        } else {
            FLOOR_MICROS
        }
    }

    /// Split a settled reward of `referee`: referrer cut rounds down,
    /// referee keeps the remainder. `None` when no referrer is bound.
    pub fn split(&self, referee: &NeuronId, epoch: u64, amount: u64) -> Option<ReferralCut> {
        let referrer = self.referrer_of(referee)?;
        let share = self.share_micros_of(&referrer, epoch);
        let cut = ((amount as u128 * share as u128) / MICROS as u128) as u64;
        Some(ReferralCut {
            referrer,
            cut,
            net: amount - cut,
            share_micros: share,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(b: u8) -> NeuronId {
        [b; 32]
    }

    #[test]
    fn curve_endpoints() {
        assert_eq!(share_micros(0), POOL_MICROS);
        assert_eq!(share_micros(DUNBAR), FLOOR_MICROS);
        assert_eq!(share_micros(DUNBAR * 10), FLOOR_MICROS);
        // Halves near the small-group scale.
        assert!(share_micros(17) < POOL_MICROS / 2 + 5_000);
    }

    #[test]
    fn curve_share_falls_income_rises() {
        for k in 1..=300u64 {
            assert!(share_micros(k) <= share_micros(k - 1), "share up at {k}");
            assert!(
                k * share_micros(k) > (k - 1) * share_micros(k - 1),
                "income down at {k}"
            );
        }
    }

    #[test]
    fn bind_once_no_self_no_cycle() {
        let mut r = Referral::new();
        assert_eq!(r.bind(n(1), n(1)), Err(ReferralError::SelfReferral));
        r.bind(n(1), n(2)).unwrap();
        // Same referrer again: no-op.
        r.bind(n(1), n(2)).unwrap();
        assert_eq!(r.bind(n(1), n(3)), Err(ReferralError::AlreadyBound));
        // 2 → 3 → 1 would close a loop through 1 → 2.
        r.bind(n(2), n(3)).unwrap();
        assert_eq!(r.bind(n(3), n(1)), Err(ReferralError::Cycle));
        assert_eq!(r.referrer_of(&n(1)), Some(n(2)));
        assert_eq!(r.referrer_of(&n(9)), None);
    }

    #[test]
    fn activity_window_filters() {
        let mut r = Referral::new();
        r.window = 5;
        r.bind(n(1), n(9)).unwrap();
        r.bind(n(2), n(9)).unwrap();
        r.mark_active(n(1), 10);
        r.mark_active(n(2), 3);
        assert_eq!(r.active_referees(&n(9), 10), 1); // n(2) is 7 epochs stale
        assert_eq!(r.active_referees(&n(9), 3), 2);
        assert_eq!(r.active_referees(&n(7), 10), 0);
        // mark_active keeps the max epoch.
        r.mark_active(n(1), 4);
        assert_eq!(r.active_referees(&n(9), 10), 1);
    }

    #[test]
    fn witness_gates_the_curve() {
        let mut r = Referral::new();
        r.bind(n(1), n(9)).unwrap();
        // Unwitnessed referrer: flat floor even with zero referees active.
        assert_eq!(r.share_micros_of(&n(9), 1), FLOOR_MICROS);
        r.attest_witness(n(9));
        assert_eq!(r.share_micros_of(&n(9), 1), POOL_MICROS);
        r.mark_active(n(1), 1);
        assert_eq!(r.share_micros_of(&n(9), 1), share_micros(1));
    }

    #[test]
    fn split_rounds_down_and_conserves() {
        let mut r = Referral::new();
        r.bind(n(1), n(9)).unwrap();
        r.attest_witness(n(9));
        r.mark_active(n(1), 1);
        let cut = r.split(&n(1), 1, 400).unwrap();
        // s(1) = 15_000_000 / 159 = 94_339 micros; 400 · s / 1e6 = 37.
        assert_eq!(cut.share_micros, 94_339);
        assert_eq!(cut.cut, 37);
        assert_eq!(cut.net, 363);
        assert_eq!(cut.cut + cut.net, 400);
        assert!(r.split(&n(9), 1, 400).is_none());
    }
}
