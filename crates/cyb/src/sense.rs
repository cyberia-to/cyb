//! Sense bridge — money events → NOTIFY-shaped observations (WP5).
//!
//! Sense is how the robot notices the world. Money credits are one stream:
//! each `MoneyEvent` that a neuron should feel becomes a sense notification
//! particle payload, keyed by the well-known `intent/notify` token.

use bbg::Particle;
use cybergraph::NeuronId;

use crate::intent::NOTIFY;
use crate::money::{ClockKind, MoneyEvent};

/// A sense notification ready for UI / chroma delivery.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SenseNotify {
    /// Who should see this (local neuron filter).
    pub audience: NeuronId,
    /// Well-known intent particle (NOTIFY).
    pub intent: Particle,
    /// Human-readable kind.
    pub kind: &'static str,
    /// Linked reason (signal id / link id).
    pub reason: Particle,
    pub amount: u64,
    pub token: Particle,
    pub clock: Option<ClockKind>,
}

/// Map money events into sense notifications for `audience`.
pub fn money_to_sense(audience: NeuronId, events: &[MoneyEvent]) -> Vec<SenseNotify> {
    let mut out = Vec::new();
    for e in events {
        match e {
            MoneyEvent::TransferIn {
                to,
                amount,
                token,
                signal,
                ..
            } if *to == audience => {
                out.push(SenseNotify {
                    audience,
                    intent: NOTIFY,
                    kind: "transfer_in",
                    reason: *signal,
                    amount: *amount,
                    token: *token,
                    clock: Some(ClockKind::A),
                });
            }
            MoneyEvent::RewardCredited {
                to,
                amount,
                token,
                reason,
                clock,
            } if *to == audience => {
                out.push(SenseNotify {
                    audience,
                    intent: NOTIFY,
                    kind: match clock {
                        ClockKind::A => "reward_pay",
                        ClockKind::B => "reward_settle",
                    },
                    reason: *reason,
                    amount: *amount,
                    token: *token,
                    clock: Some(*clock),
                });
            }
            MoneyEvent::TransferOut {
                from,
                amount,
                token,
                signal,
                ..
            } if *from == audience => {
                out.push(SenseNotify {
                    audience,
                    intent: NOTIFY,
                    kind: "transfer_out",
                    reason: *signal,
                    amount: *amount,
                    token: *token,
                    clock: Some(ClockKind::A),
                });
            }
            MoneyEvent::Finalized { signal, .. } => {
                out.push(SenseNotify {
                    audience,
                    intent: NOTIFY,
                    kind: "finalized",
                    reason: *signal,
                    amount: 0,
                    token: [0u8; 32],
                    clock: Some(ClockKind::A),
                });
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::MoneyEvent;

    #[test]
    fn maps_transfer_in_and_reward() {
        let me = [1u8; 32];
        let ev = vec![
            MoneyEvent::TransferIn {
                to: me,
                from: [2u8; 32],
                token: [7u8; 32],
                amount: 50,
                signal: [9u8; 32],
            },
            MoneyEvent::RewardCredited {
                to: me,
                amount: 50,
                token: [7u8; 32],
                reason: [9u8; 32],
                clock: ClockKind::A,
            },
        ];
        let n = money_to_sense(me, &ev);
        assert_eq!(n.len(), 2);
        assert_eq!(n[0].kind, "transfer_in");
        assert_eq!(n[0].intent, NOTIFY);
        assert_eq!(n[1].kind, "reward_pay");
    }
}
