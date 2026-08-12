//! Money loop — balance, send, receive, multi-payee reward-after-link.
//!
//! Implements cyber/specs/money-loop.md (WP1–WP6 library surface).
//! Soft3 only. UI wiring is WP7 (CLI / sense).

use std::collections::VecDeque;

use bbg::{NeuronRecord, Particle, QueryProof, balance_key, prove_balances, verify_query};
use cyber_hemera::hash as hemera_hash;
use cybergraph::{ApiError, NeuronId, Signal};
use foculus::{
    BoxMoveRecord, EpochRunner, FinalityEvidence, GENESIS_PREV, PayStatement, RewardClaim,
    SELF_NETWORK, SettleReceipt, TicketPolicy, Tip, TipProver, VdfProof, challenge_from_hash,
    claim_from_links, prove_pay, settle_epoch, share_of, vdf_evaluate, verify_live_receipt,
    verify_pay, verify_receipt,
};
use tru::{Context, FocusingParams, Fx, Link as TruLink};

/// Per-signal VDF iterations for S_E entropy (P6 rate-limit scale for local settle).
const SIGNAL_VDF_T: u64 = 16;

use crate::cell::Cell;
use crate::signal::SignalBuilder;

/// Certainty grade (money-loop §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Grade {
    LocalAuthor = 0,
    Pending = 1,
    Final = 2,
    SettledReward = 3,
    TipTrusted = 4,
}

/// Money events for sense / sigma (money-loop §6).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MoneyEvent {
    TipAdvanced {
        root: Particle,
        height: u64,
        grade4: bool,
    },
    BalanceUpdated {
        neuron: NeuronId,
        token: Particle,
        amount: u64,
        tip_height: u64,
    },
    TransferOut {
        from: NeuronId,
        to: Particle,
        token: Particle,
        amount: u64,
        signal: Particle,
    },
    TransferIn {
        to: NeuronId,
        from: Particle,
        token: Particle,
        amount: u64,
        signal: Particle,
    },
    RewardCredited {
        to: NeuronId,
        amount: u64,
        token: Particle,
        reason: Particle,
        clock: ClockKind,
    },
    FinalityFailed {
        signal: Particle,
        reason: &'static str,
    },
    /// Grade-2 certificate issued for a signal (WP4).
    Finalized {
        signal: Particle,
        evidence: FinalityEvidence,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockKind {
    A,
    B,
}

/// One pay leg inside a multi-payee Intent.
#[derive(Clone, Debug)]
pub struct PayLeg {
    pub from: Particle,
    pub to: Particle,
    pub token: Particle,
    pub amount: u64,
    pub valence: i8,
}

/// Wallet / money loop over a cell for one local neuron.
pub struct MoneyWallet {
    pub neuron: NeuronId,
    tip: Tip,
    /// Optional prover that folds each block for light-export (WP1).
    tip_prover: Option<TipProver>,
    events: VecDeque<MoneyEvent>,
    /// Last finality evidence per signal (WP4).
    finals: Vec<FinalityEvidence>,
    /// Reorg depth for clock B spendability (default 2).
    pub settle_depth: u64,
    /// Pending settle mints waiting for depth (WP6).
    pending_settles: Vec<PendingSettle>,
    /// Local private notes (secret, nonce, amount, token) — WP private path.
    notes: Vec<PrivateNote>,
    /// When true (default), pay must carry a verifiable zheng σ.
    pub require_pay_proof: bool,
    /// Queued reward claims from [`link_for_reward`] (propose window).
    pending_claims: Vec<RewardClaim>,
    /// Base graph for settle (existing structure). Empty = claims-only.
    reward_base: Vec<TruLink>,
    /// Epoch counter for beacon chain.
    pub reward_epoch: u64,
    /// Previous beacon (updated after each settle).
    prev_beacon: [u8; 32],
    /// Default budget for auto settle.
    pub reward_budget: u64,
    /// Default reward token for auto settle.
    pub reward_token: Particle,
    /// Prefer ticket/fold-mining path when settling (default true).
    pub use_tickets: bool,
    /// When true, use live EpochRunner (outer VDF beacon + HyperNova σ).
    pub use_live_epoch: bool,
    /// Optional tok mint ledger (Token conservation). When set, settle mints
    /// also record PLUMB mint legs under conservation clip.
    pub tok_ledger: Option<tok::MintLedger>,
    /// Tokens per Fx::ONE of conserved mass (tok emission scale).
    pub emission_scale: u64,
    /// When true, [`finalize_block`] runs settle if claims are pending.
    pub auto_settle: bool,
    /// Per-signal VDF proofs collected at link time (S_E entropy for beacon).
    signal_vdfs: Vec<VdfProof>,
}

/// Owned private note (local wallet material).
#[derive(Clone, Debug)]
pub struct PrivateNote {
    pub secret: [u8; 32],
    pub nonce: u64,
    pub amount: u64,
    pub token: Particle,
    pub commitment: Particle,
}

#[derive(Clone, Debug)]
struct PendingSettle {
    token: Particle,
    amount: u64,
    reason: Particle,
    mint_height: u64,
}

#[derive(Debug)]
pub enum MoneyError {
    TipNotTrusted,
    EmptyIntent,
    InsufficientBalance {
        have: u64,
        need: u64,
    },
    Graph(ApiError),
    OpeningFailed,
    OpeningUnverified,
    DoubleSpend,
    FinalityRejected,
    SettleNotMature,
    ProofFailed,
    ProofInvalid,
    NoteNotFound,
    /// No pending claims to settle.
    NothingToSettle,
    /// foculus settle failed (empty, no tickets, …).
    SettleFailed,
}

impl MoneyWallet {
    pub fn new(neuron: NeuronId) -> Self {
        Self {
            neuron,
            tip: Tip::untrusted(),
            tip_prover: None,
            events: VecDeque::new(),
            finals: Vec::new(),
            settle_depth: 2,
            pending_settles: Vec::new(),
            notes: Vec::new(),
            require_pay_proof: true,
            pending_claims: Vec::new(),
            reward_base: Vec::new(),
            reward_epoch: 1,
            prev_beacon: GENESIS_PREV,
            reward_budget: 1000,
            reward_token: [7u8; 32],
            use_tickets: true,
            use_live_epoch: true,
            auto_settle: false,
            signal_vdfs: Vec::new(),
            tok_ledger: Some(tok::MintLedger::new()),
            emission_scale: foculus::DEFAULT_EMISSION_SCALE,
        }
    }

    /// Enable / replace the tok mint ledger for conservation tracking.
    pub fn with_tok_ledger(mut self, ledger: tok::MintLedger) -> Self {
        self.tok_ledger = Some(ledger);
        self
    }

    /// Enable TipProver so each finalize_block folds for light clients.
    pub fn with_tip_prover(mut self) -> Self {
        self.tip_prover = Some(TipProver::new());
        self
    }

    pub fn sync_tip_local(&mut self, cell: &Cell) {
        let mut ck = cell.graph.bbg.checkpoint.advance(&cell.graph.bbg.state);
        ck.root = cell.graph.bbg.state.root();
        ck.height = cell.graph.bbg.state.height;
        self.tip = Tip::from_local(&ck);
        self.events.push_back(MoneyEvent::TipAdvanced {
            root: self.tip.root,
            height: self.tip.height,
            grade4: self.tip.grade4(),
        });
    }

    pub fn set_tip(&mut self, tip: Tip) {
        let grade4 = tip.grade4();
        self.events.push_back(MoneyEvent::TipAdvanced {
            root: tip.root,
            height: tip.height,
            grade4,
        });
        self.tip = tip;
    }

    pub fn tip(&self) -> &Tip {
        &self.tip
    }

    pub fn grade4(&self) -> bool {
        self.tip.grade4()
    }

    pub fn drain_events(&mut self) -> Vec<MoneyEvent> {
        self.events.drain(..).collect()
    }

    pub fn finality_of(&self, signal_id: &Particle) -> Option<&FinalityEvidence> {
        self.finals.iter().find(|e| e.signal_id == *signal_id)
    }

    pub fn balance(&self, cell: &Cell, owner: &NeuronId, token: &Particle) -> u64 {
        let key = balance_key(owner, token);
        cell.graph
            .bbg
            .state
            .balances
            .get(&key)
            .copied()
            .unwrap_or(0)
    }

    /// WP2: open balance against grade-4 tip.
    pub fn open_balance(
        &self,
        cell: &Cell,
        owner: &NeuronId,
        token: &Particle,
    ) -> Result<(u64, QueryProof), MoneyError> {
        if !self.tip.grade4() {
            return Err(MoneyError::TipNotTrusted);
        }
        let proof =
            prove_balances(&cell.graph.bbg.state, owner, token).ok_or(MoneyError::OpeningFailed)?;
        if !verify_query(&proof) {
            return Err(MoneyError::OpeningUnverified);
        }
        Ok((self.balance(cell, owner, token), proof))
    }

    /// Genesis fund (tests / bootstrap).
    pub fn fund_for_test(&mut self, cell: &mut Cell, token: Particle, amount: u64) {
        let key = balance_key(&self.neuron, &token);
        *cell.graph.bbg.state.balances.entry(key).or_insert(0) += amount;
        cell.graph
            .bbg
            .state
            .neurons
            .entry(self.neuron)
            .or_insert(NeuronRecord {
                focus: 0,
                karma: 0,
                stake: 0,
            })
            .focus = cell
            .graph
            .bbg
            .state
            .neurons
            .get(&self.neuron)
            .map(|n| n.focus)
            .unwrap_or(0)
            .saturating_add(amount.saturating_mul(2));
        cell.graph.bbg.state.refresh_root();
        cell.graph.bbg.checkpoint.root = cell.graph.bbg.state.root();
        cell.graph.bbg.checkpoint.height = cell.graph.bbg.state.height;
        self.sync_tip_local(cell);
        self.events.push_back(MoneyEvent::BalanceUpdated {
            neuron: self.neuron,
            token,
            amount: self.balance(cell, &self.neuron, &token),
            tip_height: self.tip.height,
        });
    }

    /// Multi-payee pay (WP3). Optional nullifiers for private legs.
    /// Always attaches zheng σ when `require_pay_proof` (default).
    /// Issues FinalityEvidence on success (WP4+ nullifier binding).
    pub fn pay(
        &mut self,
        cell: &mut Cell,
        legs: &[PayLeg],
        box_moves: &[BoxMoveRecord],
    ) -> Result<(Particle, FinalityEvidence), MoneyError> {
        if !self.tip.grade4() {
            return Err(MoneyError::TipNotTrusted);
        }
        if legs.is_empty() {
            return Err(MoneyError::EmptyIntent);
        }
        for leg in legs {
            if leg.from == self.neuron {
                let have = self.balance(cell, &self.neuron, &leg.token);
                if have < leg.amount {
                    return Err(MoneyError::InsufficientBalance {
                        have,
                        need: leg.amount,
                    });
                }
            }
        }

        let (step, prev) = cell_tip(cell, &self.neuron);
        let mut builder = SignalBuilder::new(self.neuron);
        for leg in legs {
            builder = builder.link(leg.from, leg.to, leg.token, leg.amount, leg.valence);
        }
        let mut sig = builder.build();
        sig.step = step;
        sig.prev = prev;
        sig.network = SELF_NETWORK;
        sig.box_moves = box_moves.to_vec();
        sig.height = cell.graph.bbg.state.height;

        // content_id before proof (proof not in content_id hash today)
        let content_id = sig.content_id();
        let total_out: u64 = legs.iter().map(|l| l.amount).sum();
        let pay_stmt = PayStatement {
            content_id,
            total_out,
            leg_count: legs.len() as u32,
        };
        if self.require_pay_proof {
            let proof = prove_pay(&pay_stmt).map_err(|_| MoneyError::ProofFailed)?;
            if !verify_pay(&proof, &pay_stmt) {
                return Err(MoneyError::ProofInvalid);
            }
            sig.proof = Some(proof);
        }

        let nullifiers: Vec<_> = box_moves.iter().map(|m| m.nullifier).collect();

        let signal_id = match cell.commit_public(sig) {
            Ok(id) => id,
            Err(ApiError::BbgRejected(bbg::InsertError::DoubleSpend)) => {
                self.events.push_back(MoneyEvent::FinalityFailed {
                    signal: content_id,
                    reason: "double spend",
                });
                return Err(MoneyError::DoubleSpend);
            }
            Err(e) => {
                self.events.push_back(MoneyEvent::FinalityFailed {
                    signal: content_id,
                    reason: "commit failed",
                });
                return Err(MoneyError::Graph(e));
            }
        };

        // Advance tip; optional fold for light export.
        self.after_state_change(cell);

        let evidence = FinalityEvidence::issue_local(content_id, &self.tip, &nullifiers);
        if !evidence.verify(&self.tip) {
            return Err(MoneyError::FinalityRejected);
        }
        self.finals.push(evidence.clone());
        self.events.push_back(MoneyEvent::Finalized {
            signal: content_id,
            evidence: evidence.clone(),
        });

        for leg in legs {
            if leg.from == self.neuron {
                self.events.push_back(MoneyEvent::TransferOut {
                    from: self.neuron,
                    to: leg.to,
                    token: leg.token,
                    amount: leg.amount,
                    signal: signal_id,
                });
            }
            if leg.to == self.neuron {
                self.events.push_back(MoneyEvent::TransferIn {
                    to: self.neuron,
                    from: leg.from,
                    token: leg.token,
                    amount: leg.amount,
                    signal: signal_id,
                });
                self.events.push_back(MoneyEvent::RewardCredited {
                    to: self.neuron,
                    amount: leg.amount,
                    token: leg.token,
                    reason: signal_id,
                    clock: ClockKind::A,
                });
            }
            self.events.push_back(MoneyEvent::BalanceUpdated {
                neuron: self.neuron,
                token: leg.token,
                amount: self.balance(cell, &self.neuron, &leg.token),
                tip_height: self.tip.height,
            });
        }

        Ok((content_id, evidence))
    }

    /// Mint a private note locally (commitment only on-chain via next spend).
    pub fn mint_private_note(
        &mut self,
        token: Particle,
        amount: u64,
        secret: [u8; 32],
        nonce: u64,
    ) -> PrivateNote {
        let commitment = Self::note_commitment(&secret, nonce, amount, &token);
        let note = PrivateNote {
            secret,
            nonce,
            amount,
            token,
            commitment,
        };
        self.notes.push(note.clone());
        note
    }

    pub fn note_commitment(
        secret: &[u8; 32],
        nonce: u64,
        amount: u64,
        token: &Particle,
    ) -> Particle {
        let mut buf = [0u8; 32 + 8 + 8 + 32];
        buf[..32].copy_from_slice(secret);
        buf[32..40].copy_from_slice(&nonce.to_le_bytes());
        buf[40..48].copy_from_slice(&amount.to_le_bytes());
        buf[48..].copy_from_slice(token);
        let h = hemera_hash(&buf);
        *h.as_bytes().first_chunk::<32>().unwrap_or(&[0u8; 32])
    }

    /// Spend a private note: nullifier + optional change commitment (WP private).
    pub fn spend_private_note(
        &mut self,
        cell: &mut Cell,
        commitment: &Particle,
        to: Particle,
        send_amount: u64,
    ) -> Result<(Particle, FinalityEvidence), MoneyError> {
        let idx = self
            .notes
            .iter()
            .position(|n| n.commitment == *commitment)
            .ok_or(MoneyError::NoteNotFound)?;
        let note = self.notes.remove(idx);
        if note.amount < send_amount {
            let have = note.amount;
            self.notes.insert(idx, note);
            return Err(MoneyError::InsufficientBalance {
                have,
                need: send_amount,
            });
        }
        let nullifier = Self::nullifier(&note.secret, note.nonce);
        let change = note.amount - send_amount;
        let mut moves = vec![BoxMoveRecord {
            nullifier,
            commitment: None,
        }];
        if change > 0 {
            let new_nonce = note.nonce + 1;
            let new_note = self.mint_private_note(note.token, change, note.secret, new_nonce);
            moves[0].commitment = Some((new_note.commitment, change));
        }
        // Public balance leg: credit recipient from a zero-from public mint path
        // is not used — private spend only gates nullifier; amount also moves public
        // box from self for observability of the payment amount.
        self.pay(
            cell,
            &[PayLeg {
                from: self.neuron,
                to,
                token: note.token,
                amount: send_amount,
                valence: 0,
            }],
            &moves,
        )
    }

    pub fn private_notes(&self) -> &[PrivateNote] {
        &self.notes
    }

    pub fn send(
        &mut self,
        cell: &mut Cell,
        to: Particle,
        token: Particle,
        amount: u64,
    ) -> Result<(Particle, FinalityEvidence), MoneyError> {
        self.pay(
            cell,
            &[PayLeg {
                from: self.neuron,
                to,
                token,
                amount,
                valence: 0,
            }],
            &[],
        )
    }

    /// Derive a deterministic nullifier for a private spend (WP3 helper).
    pub fn nullifier(secret: &[u8; 32], nonce: u64) -> Particle {
        let mut buf = [0u8; 40];
        buf[..32].copy_from_slice(secret);
        buf[32..].copy_from_slice(&nonce.to_le_bytes());
        let h = hemera_hash(&buf);
        *h.as_bytes().first_chunk::<32>().unwrap_or(&[0u8; 32])
    }

    /// Receive path: observe applied signal with optional finality check (WP4).
    pub fn observe_signal(
        &mut self,
        cell: &Cell,
        signal: &Signal,
        evidence: Option<&FinalityEvidence>,
    ) -> Result<(), MoneyError> {
        if !self.tip.grade4() {
            return Err(MoneyError::TipNotTrusted);
        }
        if let Some(ev) = evidence {
            if !ev.verify(&self.tip) {
                return Err(MoneyError::FinalityRejected);
            }
            self.finals.push(ev.clone());
        }
        let signal_id = signal.content_id();
        for leg in &signal.links {
            if leg.to == self.neuron && leg.amount > 0 {
                self.events.push_back(MoneyEvent::TransferIn {
                    to: self.neuron,
                    from: leg.from,
                    token: leg.token,
                    amount: leg.amount,
                    signal: signal_id,
                });
                self.events.push_back(MoneyEvent::RewardCredited {
                    to: self.neuron,
                    amount: leg.amount,
                    token: leg.token,
                    reason: signal_id,
                    clock: ClockKind::A,
                });
                self.events.push_back(MoneyEvent::BalanceUpdated {
                    neuron: self.neuron,
                    token: leg.token,
                    amount: self.balance(cell, &self.neuron, &leg.token),
                    tip_height: self.tip.height,
                });
            }
        }
        Ok(())
    }

    /// WP6: mint attribution reward into balance; spendable after settle_depth blocks.
    /// Prefer [`apply_settle_receipt`] when a foculus settle ran — this is the
    /// low-level credit after the amount is known.
    pub fn mint_settle_reward(
        &mut self,
        cell: &mut Cell,
        token: Particle,
        amount: u64,
        reason: Particle,
    ) -> Result<(), MoneyError> {
        if !self.tip.grade4() {
            return Err(MoneyError::TipNotTrusted);
        }
        if amount == 0 {
            return Ok(());
        }
        let key = balance_key(&self.neuron, &token);
        *cell.graph.bbg.state.balances.entry(key).or_insert(0) += amount;
        cell.graph.bbg.state.refresh_root();
        self.after_state_change(cell);
        let h = self.tip.height;
        self.pending_settles.push(PendingSettle {
            token,
            amount,
            reason,
            mint_height: h,
        });
        self.events.push_back(MoneyEvent::RewardCredited {
            to: self.neuron,
            amount,
            token,
            reason,
            clock: ClockKind::B,
        });
        self.events.push_back(MoneyEvent::BalanceUpdated {
            neuron: self.neuron,
            token,
            amount: self.balance(cell, &self.neuron, &token),
            tip_height: h,
        });
        Ok(())
    }

    /// Apply a foculus [`SettleReceipt`]: verify hash, mint this neuron's share
    /// with clock-B escrow. `reason` defaults to receipt_hash.
    pub fn apply_settle_receipt(
        &mut self,
        cell: &mut Cell,
        receipt: &SettleReceipt,
        token: Particle,
    ) -> Result<u64, MoneyError> {
        if !self.tip.grade4() {
            return Err(MoneyError::TipNotTrusted);
        }
        if !verify_receipt(receipt) {
            return Err(MoneyError::ProofInvalid);
        }
        let amount = share_of(receipt, &self.neuron);
        if amount == 0 {
            return Ok(0);
        }
        // Tok PLUMB ledger: this neuron's mint leg only (multi-wallet safe).
        if let Some(led) = self.tok_ledger.as_mut() {
            let _ = led.mint_batch(token, &[(self.neuron, amount)]);
        }
        self.mint_settle_reward(cell, token, amount, receipt.receipt_hash)?;
        Ok(amount)
    }

    /// Seed the base graph used by settle (existing structure before claims).
    pub fn set_reward_base(&mut self, base: Vec<TruLink>) {
        self.reward_base = base;
    }

    /// Claims queued from local links (propose window).
    pub fn pending_claims(&self) -> &[RewardClaim] {
        &self.pending_claims
    }

    /// Ingest a peer's claim into the local propose window.
    pub fn ingest_claim(&mut self, claim: RewardClaim) {
        self.pending_claims.push(claim);
    }

    /// Submit a staked cyberlink and queue a reward claim (reward-after-link).
    ///
    /// Commits a signal with the structural edge, builds a [`RewardClaim`]
    /// bound to the signal id, and enqueues it for the next settle.
    pub fn link_for_reward(
        &mut self,
        cell: &mut Cell,
        from: Particle,
        to: Particle,
        amount: u64,
        valence: i8,
    ) -> Result<Particle, MoneyError> {
        if !self.tip.grade4() {
            // Allow tip bootstrap via fund_for_test; otherwise require grade 4.
            return Err(MoneyError::TipNotTrusted);
        }
        let (step, prev) = match cell.graph.chains.get(&self.neuron) {
            Some(chain) if !chain.entries.is_empty() => {
                let step = chain.entries.len() as u64;
                let prev = chain.entries[&(step - 1)].hash();
                (step, prev)
            }
            _ => (0, [0u8; 32]),
        };
        let mut sig = SignalBuilder::new(self.neuron)
            .link(from, to, self.reward_token, amount, valence)
            .build();
        sig.step = step;
        sig.prev = prev;
        let signal_id = cell.commit_public(sig).map_err(MoneyError::Graph)?;
        self.after_state_change(cell);

        // Tru link for attribution (neuron = author).
        let mut tlink = TruLink::stake(from, to, amount as u128);
        tlink.neuron = self.neuron;
        tlink.valence = valence;
        tlink.price = Fx::ONE;
        let claim = claim_from_links(signal_id, self.neuron, vec![tlink], valence);
        self.pending_claims.push(claim);
        // Per-signal VDF entropy for live beacon S_E (specs/beacon.md).
        let vdf = vdf_evaluate(challenge_from_hash(&signal_id), SIGNAL_VDF_T);
        self.signal_vdfs.push(vdf);
        // RewardCredited (clock B) fires on apply_settle_receipt / mint, not at link.
        Ok(signal_id)
    }

    /// Settle all pending claims (plus optional peer claims), mint local share,
    /// advance epoch beacon, clear settled local claims.
    ///
    /// Default (`use_live_epoch`): real EpochRunner —
    /// freeze → outer VDF beacon over signal VDFs → tickets → HyperNova σ → mint.
    pub fn settle_pending_rewards(
        &mut self,
        cell: &mut Cell,
        extra_claims: &[RewardClaim],
    ) -> Result<(SettleReceipt, u64), MoneyError> {
        if !self.tip.grade4() {
            return Err(MoneyError::TipNotTrusted);
        }
        let mut claims = self.pending_claims.clone();
        claims.extend(extra_claims.iter().cloned());
        if claims.is_empty() {
            return Err(MoneyError::NothingToSettle);
        }
        let ctx = Context::none();
        let params = FocusingParams::default();
        let epoch = self.reward_epoch;
        let budget = self.reward_budget;
        let token = self.reward_token;

        let receipt = if self.use_live_epoch {
            let mut runner = EpochRunner::new(epoch, self.prev_beacon);
            runner.budget = budget;
            for c in &claims {
                runner
                    .propose(c.clone())
                    .map_err(|_| MoneyError::SettleFailed)?;
            }
            let policy = TicketPolicy {
                want: 4.min(32),
                max_attempts: 128,
                miner: self.neuron,
                ..TicketPolicy::default()
            };
            let vdfs = self.signal_vdfs.clone();
            runner
                .run_to_settle(&self.reward_base, &ctx, &params, &vdfs, &policy)
                .map_err(|_| MoneyError::SettleFailed)?
                .clone()
        } else if self.use_tickets {
            // Fallback: ticket path without full epoch runner (still VDF beacon + σ).
            foculus::settle_epoch_tickets(
                epoch,
                &self.prev_beacon,
                &self.reward_base,
                &claims,
                &ctx,
                &params,
                budget,
                &TicketPolicy {
                    want: 4,
                    max_attempts: 128,
                    miner: self.neuron,
                    ..TicketPolicy::default()
                },
            )
            .map_err(|_| MoneyError::SettleFailed)?
        } else {
            settle_epoch(
                epoch,
                &self.prev_beacon,
                &self.reward_base,
                &claims,
                &ctx,
                &params,
                32,
                budget,
            )
            .map_err(|_| MoneyError::SettleFailed)?
        };

        // Live receipts must verify VDF + HyperNova seals.
        if self.use_live_epoch && !verify_live_receipt(&receipt) {
            return Err(MoneyError::ProofInvalid);
        }
        if !verify_receipt(&receipt) {
            return Err(MoneyError::ProofInvalid);
        }

        let minted = self.apply_settle_receipt(cell, &receipt, token)?;
        // Clear local claims that appear in this settle (by id).
        let settled_ids: std::collections::HashSet<_> = claims.iter().map(|c| c.id).collect();
        self.pending_claims.retain(|c| !settled_ids.contains(&c.id));
        self.signal_vdfs.clear();
        self.prev_beacon = receipt.beacon;
        self.reward_epoch = self.reward_epoch.saturating_add(1);
        Ok((receipt, minted))
    }

    /// One-shot product path: link → ticket settle → mint local share.
    pub fn link_and_settle(
        &mut self,
        cell: &mut Cell,
        from: Particle,
        to: Particle,
        amount: u64,
        valence: i8,
    ) -> Result<(Particle, SettleReceipt, u64), MoneyError> {
        let sid = self.link_for_reward(cell, from, to, amount, valence)?;
        let (rec, minted) = self.settle_pending_rewards(cell, &[])?;
        Ok((sid, rec, minted))
    }

    /// Mature settle rewards once tip height ≥ mint_height + settle_depth.
    pub fn mature_settles(&mut self) -> Vec<(Particle, u64, Particle)> {
        let h = self.tip.height;
        let depth = self.settle_depth;
        let mut ready = Vec::new();
        self.pending_settles.retain(|p| {
            if h >= p.mint_height.saturating_add(depth) {
                ready.push((p.token, p.amount, p.reason));
                false
            } else {
                true
            }
        });
        ready
    }

    /// Whether a clock-B credit of this reason is mature for spend (WP6).
    pub fn settle_mature(&self, reason: &Particle) -> bool {
        !self.pending_settles.iter().any(|p| p.reason == *reason)
    }

    /// Finalize a block: bump height, fold tip prover.
    /// When [`auto_settle`](Self::auto_settle) and claims are pending, settles.
    /// Call [`mature_settles`] afterward to harvest clock-B credits.
    pub fn finalize_block(&mut self, cell: &mut Cell) {
        cell.graph.bbg.finalize_block();
        self.after_state_change(cell);
        if self.auto_settle && !self.pending_claims.is_empty() {
            let _ = self.settle_pending_rewards(cell, &[]);
        }
    }

    /// Export light-joinable tip if prover is enabled (WP1).
    pub fn export_light_tip(&mut self) -> Option<Tip> {
        let prover = self.tip_prover.as_ref()?;
        prover.seal_tip().ok()
    }

    fn after_state_change(&mut self, cell: &Cell) {
        let root = cell.graph.bbg.state.root();
        let height = cell.graph.bbg.state.height;
        if let Some(prover) = self.tip_prover.as_mut() {
            // Bind BBG root-leaves hash into the fold (production-shaped tip).
            let leaves = root_leaves_hash(cell);
            let _ = prover.fold_block(height, root, leaves);
            if let Ok(t) = prover.seal_tip() {
                self.tip = t;
            } else {
                self.sync_tip_local(cell);
            }
        } else {
            self.sync_tip_local(cell);
        }
    }
}

/// Hemera over zheng RootLeaves bytes — stable block identity for tip fold.
fn root_leaves_hash(cell: &Cell) -> Particle {
    use cyber_hemera::hash as hemera_hash;
    let leaves = cell.graph.bbg.state.root_leaves();
    // RootLeaves is a typed structure; hash its debug-stable field layout via root again
    // plus height to distinguish empty graphs that share roots.
    let mut buf = [0u8; 40];
    buf[..32].copy_from_slice(&cell.graph.bbg.state.root());
    buf[32..].copy_from_slice(&cell.graph.bbg.state.height.to_le_bytes());
    let _ = leaves; // reserved: encode full leaf vector when wire format freezes
    let h = hemera_hash(&buf);
    *h.as_bytes().first_chunk::<32>().unwrap_or(&[0u8; 32])
}

fn cell_tip(cell: &Cell, neuron: &NeuronId) -> (u64, Particle) {
    match cell.graph.chains.get(neuron) {
        Some(chain) if !chain.entries.is_empty() => {
            let step = chain.entries.len() as u64;
            let prev = chain.entries[&(step - 1)].hash();
            (step, prev)
        }
        _ => (0, [0u8; 32]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use foculus::{TipTrust, join_with_demo_fold};

    fn token() -> Particle {
        [7u8; 32]
    }
    fn alice() -> NeuronId {
        [1u8; 32]
    }
    fn bob() -> NeuronId {
        [2u8; 32]
    }

    #[test]
    fn send_receive_multi_payee_finality() {
        let mut cell = Cell::ephemeral();
        let mut w_alice = MoneyWallet::new(alice());
        let mut w_bob = MoneyWallet::new(bob());

        w_alice.fund_for_test(&mut cell, token(), 1_000);
        let carol: Particle = [3u8; 32];
        let (sig, ev) = w_alice
            .pay(
                &mut cell,
                &[
                    PayLeg {
                        from: alice(),
                        to: bob(),
                        token: token(),
                        amount: 300,
                        valence: 0,
                    },
                    PayLeg {
                        from: alice(),
                        to: carol,
                        token: token(),
                        amount: 100,
                        valence: 1,
                    },
                ],
                &[],
            )
            .unwrap();
        assert!(ev.verify(w_alice.tip()));
        assert_eq!(w_alice.balance(&cell, &alice(), &token()), 600);

        w_bob.sync_tip_local(&cell);
        let signal = cell
            .signals()
            .into_iter()
            .find(|s| s.neuron == alice())
            .unwrap();
        w_bob.observe_signal(&cell, signal, Some(&ev)).unwrap();
        assert!(
            w_bob
                .drain_events()
                .iter()
                .any(|e| matches!(e, MoneyEvent::TransferIn { amount: 300, .. }))
        );
        let _ = sig;
    }

    #[test]
    fn double_spend_nullifier_rejected() {
        let mut cell = Cell::ephemeral();
        let mut w = MoneyWallet::new(alice());
        w.fund_for_test(&mut cell, token(), 100);
        let n = MoneyWallet::nullifier(&[9u8; 32], 1);
        let mv = BoxMoveRecord {
            nullifier: n,
            commitment: Some(([8u8; 32], 50)),
        };
        w.pay(
            &mut cell,
            &[PayLeg {
                from: alice(),
                to: bob(),
                token: token(),
                amount: 10,
                valence: 0,
            }],
            &[mv.clone()],
        )
        .unwrap();
        let err = w
            .pay(
                &mut cell,
                &[PayLeg {
                    from: alice(),
                    to: bob(),
                    token: token(),
                    amount: 10,
                    valence: 0,
                }],
                &[mv],
            )
            .unwrap_err();
        assert!(matches!(err, MoneyError::DoubleSpend));
    }

    #[test]
    fn light_fold_tip_open_and_advance() {
        let mut cell = Cell::ephemeral();
        let mut w = MoneyWallet::new(alice()).with_tip_prover();
        w.fund_for_test(&mut cell, token(), 42);
        w.finalize_block(&mut cell);
        let light = w.export_light_tip().expect("light tip");
        assert_eq!(light.trust, TipTrust::FoldDecided);
        let mut w2 = MoneyWallet::new(alice());
        w2.set_tip(light);
        let (amt, proof) = w2.open_balance(&cell, &alice(), &token()).unwrap();
        assert_eq!(amt, 42);
        assert!(verify_query(&proof));
    }

    #[test]
    fn settle_reward_matures_after_depth() {
        let mut cell = Cell::ephemeral();
        let mut w = MoneyWallet::new(alice());
        w.settle_depth = 2;
        w.fund_for_test(&mut cell, token(), 0);
        let reason = [4u8; 32];
        w.mint_settle_reward(&mut cell, token(), 25, reason)
            .unwrap();
        let mint_h = w.tip().height;
        assert!(!w.settle_mature(&reason));
        // need tip.height >= mint_h + settle_depth
        while w.tip().height < mint_h + w.settle_depth {
            w.finalize_block(&mut cell);
        }
        assert!(
            w.tip().height >= mint_h + w.settle_depth,
            "tip {} mint {} depth {}",
            w.tip().height,
            mint_h,
            w.settle_depth
        );
        let ready = w.mature_settles();
        assert_eq!(ready, vec![(token(), 25, reason)]);
        assert!(w.settle_mature(&reason));
        assert_eq!(w.balance(&cell, &alice(), &token()), 25);
    }

    #[test]
    fn open_balance_requires_grade4() {
        let cell = Cell::ephemeral();
        let w = MoneyWallet::new(alice());
        assert!(matches!(
            w.open_balance(&cell, &alice(), &token()),
            Err(MoneyError::TipNotTrusted)
        ));
    }

    #[test]
    fn observe_rejects_bad_finality() {
        let mut cell = Cell::ephemeral();
        let mut w_alice = MoneyWallet::new(alice());
        let mut w_bob = MoneyWallet::new(bob());
        w_alice.fund_for_test(&mut cell, token(), 100);
        let (sig, ev) = w_alice.send(&mut cell, bob(), token(), 10).unwrap();
        w_bob.sync_tip_local(&cell);
        // tamper evidence
        let mut bad = ev.clone();
        bad.binding = [0u8; 32];
        let signal = cell
            .signals()
            .into_iter()
            .find(|s| s.neuron == alice())
            .unwrap();
        assert!(matches!(
            w_bob.observe_signal(&cell, signal, Some(&bad)),
            Err(MoneyError::FinalityRejected)
        ));
        let _ = sig;
    }

    #[test]
    fn demo_join_still_works() {
        let tip = join_with_demo_fold([1u8; 32], 0);
        assert!(tip.grade4());
    }

    #[test]
    fn pay_attaches_verifiable_proof() {
        let mut cell = Cell::ephemeral();
        let mut w = MoneyWallet::new(alice());
        w.fund_for_test(&mut cell, token(), 100);
        let (cid, ev) = w.send(&mut cell, bob(), token(), 10).unwrap();
        assert!(ev.verify(w.tip()));
        // content-bound finality includes empty nullifier set
        assert_eq!(ev.signal_id, cid);
    }

    #[test]
    fn link_and_settle_auto_reward_after_link() {
        use foculus::verify_live_receipt;
        use tru::Link;

        let mut cell = Cell::ephemeral();
        let mut w = MoneyWallet::new(alice());
        w.fund_for_test(&mut cell, token(), 0);
        w.settle_depth = 1;
        w.reward_budget = 400;
        w.reward_token = token();
        w.use_live_epoch = true;
        w.use_tickets = true;
        // base graph so impulse has structure
        fn h(b: u8) -> [u8; 32] {
            let mut x = [0u8; 32];
            x[0] = b;
            x
        }
        w.set_reward_base(vec![
            Link::stake(h(1), h(2), 100),
            Link::stake(h(2), h(3), 100),
            Link::stake(h(3), h(1), 100),
        ]);

        let (sid, rec, minted) = w
            .link_and_settle(&mut cell, h(2), h(1), 8000, 1)
            .expect("link_and_settle");
        assert_ne!(sid, [0u8; 32]);
        assert!(verify_receipt(&rec));
        assert!(
            verify_live_receipt(&rec),
            "live path must carry VDF beacon + HyperNova seals"
        );
        assert!(rec.beacon_artifact.is_some());
        assert!(rec.ticket_seal.is_some());
        assert!(rec.fold_seal.is_some());
        assert_eq!(minted, 400);
        assert_eq!(w.balance(&cell, &alice(), &token()), 400);
        assert!(w.pending_claims().is_empty());
        // clock B maturity
        let mint_h = w.tip().height;
        while w.tip().height < mint_h + w.settle_depth {
            w.finalize_block(&mut cell);
        }
        let ready = w.mature_settles();
        assert_eq!(ready[0].1, 400);
        // tok ledger recorded the mint
        let led = w.tok_ledger.as_ref().unwrap();
        assert!(led.check_token(token()));
        assert_eq!(led.balance(&alice(), &token()), 400);
    }

    #[test]
    fn auto_settle_on_finalize() {
        use tru::Link;

        let mut cell = Cell::ephemeral();
        let mut w = MoneyWallet::new(alice());
        w.fund_for_test(&mut cell, token(), 0);
        w.auto_settle = true;
        w.use_tickets = true;
        w.reward_budget = 250;
        w.reward_token = token();
        fn h(b: u8) -> [u8; 32] {
            let mut x = [0u8; 32];
            x[0] = b;
            x
        }
        w.set_reward_base(vec![
            Link::stake(h(1), h(2), 100),
            Link::stake(h(2), h(3), 100),
            Link::stake(h(3), h(1), 100),
        ]);
        w.link_for_reward(&mut cell, h(2), h(1), 5000, 1)
            .expect("link");
        assert_eq!(w.pending_claims().len(), 1);
        // finalize triggers auto settle
        w.finalize_block(&mut cell);
        assert!(w.pending_claims().is_empty());
        assert_eq!(w.balance(&cell, &alice(), &token()), 250);
    }

    #[test]
    fn settle_receipt_mints_from_shapley() {
        use foculus::{GENESIS_PREV, claim_from_links, settle_epoch, verify_receipt};
        use tru::{Context, FocusingParams, Link};

        let mut cell = Cell::ephemeral();
        let mut w = MoneyWallet::new(alice());
        w.fund_for_test(&mut cell, token(), 0);
        w.settle_depth = 1;

        // Base graph (existing structure) + alice's contributing link.
        fn h(b: u8) -> [u8; 32] {
            let mut x = [0u8; 32];
            x[0] = b;
            x
        }
        let base = vec![
            Link::stake(h(1), h(2), 100),
            Link::stake(h(2), h(3), 100),
            Link::stake(h(3), h(1), 100),
        ];
        let claim = claim_from_links(
            [0xAAu8; 32],
            alice(),
            vec![Link::stake(h(2), h(1), 8000)],
            1,
        );
        let rec = settle_epoch(
            1,
            &GENESIS_PREV,
            &base,
            &[claim],
            &Context::none(),
            &FocusingParams::default(),
            16,
            500,
        )
        .expect("settle_epoch");
        assert!(verify_receipt(&rec));
        assert_eq!(foculus::share_of(&rec, &alice()), 500);

        let minted = w
            .apply_settle_receipt(&mut cell, &rec, token())
            .expect("apply_settle_receipt");
        assert_eq!(minted, 500);
        assert_eq!(w.balance(&cell, &alice(), &token()), 500);

        // Clock-B escrow: not mature until settle_depth blocks pass.
        assert!(!w.settle_mature(&rec.receipt_hash));
        let mint_h = w.tip().height;
        while w.tip().height < mint_h + w.settle_depth {
            w.finalize_block(&mut cell);
        }
        let ready = w.mature_settles();
        assert_eq!(ready, vec![(token(), 500, rec.receipt_hash)]);
        assert!(w.settle_mature(&rec.receipt_hash));

        // RewardCredited(clock B) was emitted on mint.
        assert!(w.drain_events().iter().any(|e| matches!(
            e,
            MoneyEvent::RewardCredited {
                amount: 500,
                clock: ClockKind::B,
                ..
            }
        )));
    }

    #[test]
    fn private_note_spend_uses_nullifier() {
        let mut cell = Cell::ephemeral();
        let mut w = MoneyWallet::new(alice());
        w.fund_for_test(&mut cell, token(), 100);
        let note = w.mint_private_note(token(), 40, [9u8; 32], 1);
        assert_eq!(w.private_notes().len(), 1);
        let (cid, ev) = w
            .spend_private_note(&mut cell, &note.commitment, bob(), 15)
            .unwrap();
        assert!(ev.verify(w.tip()));
        // change note remains
        assert_eq!(w.private_notes().len(), 1);
        assert_eq!(w.private_notes()[0].amount, 25);
        // replay same nullifier fails
        w.notes.push(PrivateNote {
            secret: [9u8; 32],
            nonce: 1,
            amount: 40,
            token: token(),
            commitment: note.commitment,
        });
        let err = w
            .spend_private_note(&mut cell, &note.commitment, bob(), 5)
            .unwrap_err();
        assert!(matches!(err, MoneyError::DoubleSpend));
        let _ = cid;
    }
}
