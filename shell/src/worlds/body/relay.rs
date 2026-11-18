//! relay — the body's signals reach the chain, signed.
//!
//! The local cell is the truth; the network is where that truth becomes
//! common. The relay tails the cell's signal chains and submits every NEW
//! link to the first configured network as a kind-2 cyberlink transaction
//! of soft3/specs/chain.md: authored by this cyb's neuron, NSIG1-signed
//! over the chain id, `POST /tx`. The unsigned `/v1/link` ingress is gone
//! (chain-wire §7 answers 410); a body that lives (casts attention, asks
//! questions, reads particles) IS what moves the chain, and now the chain
//! can tell it was this body.
//!
//! Best-effort by design: the chain mirrors the cell, never gates it. A
//! failed submit is retried on the next pass; the high-water mark
//! (`~/cyb/relayed`: "neuron-hex next-step" lines — the step the next
//! pass starts from) only advances when every link of a step is in. First boot starts from NOW — the past is local history, not
//! a flood to replay; `relay all` (future) can push it deliberately.
//! Nothing is signed until the node has published its chain id in
//! `/status` — a tx bound to a guessed chain is a tx for nobody.
//!
//! What crosses and what does not. The tx carries `(p, q, valence)` and
//! nothing else of the link: its token becomes the chain's native token
//! and its amount 0, so the weight `cast_weighted` records
//! (core/src/cell.rs) stays local — the chain sees the assertion, not
//! its price. And only THIS body's signal chain is relayed: the tx is
//! authored by `signer.neuron`, so another neuron's links would go out
//! over this body's name. They stay in the cell.
//!
//! Toggle: `~/cyb/relay` ("on"/"off", default on).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use bevy::log::{info, warn};
use soft3::wallet::{Client, Signer};

use super::networks::NetHub;
use crate::worlds::SharedCell;

/// How often the tail is checked. Attention casts are seconds apart;
/// the chain does not need microsecond mirroring.
const PASS_EVERY: Duration = Duration::from_secs(5);

#[derive(Clone, Default)]
pub struct Relay {
    /// Links successfully relayed this session.
    pub sent: Arc<AtomicU64>,
    /// Links that failed on the last pass (retried next pass).
    pub pending: Arc<AtomicU64>,
}

fn relay_file() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::Path::new(&home).join("cyb").join("relayed")
}

fn toggle_file() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::Path::new(&home).join("cyb").join("relay")
}

fn wanted() -> bool {
    std::fs::read_to_string(toggle_file())
        .map(|s| s.trim() != "off")
        .unwrap_or(true)
}

fn load_marks() -> HashMap<String, u64> {
    std::fs::read_to_string(relay_file())
        .map(|text| {
            text.lines()
                .filter_map(|l| {
                    let (n, s) = l.split_once(' ')?;
                    Some((n.to_string(), s.trim().parse().ok()?))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn save_marks(marks: &HashMap<String, u64>) {
    if marks.is_empty() {
        // An empty file would still make the next boot not a first run:
        // the mark would fall back to 0 and the whole local past would be
        // queued for the chain — the flood this module promises not to
        // replay. Nothing to record is recorded as nothing.
        return;
    }
    let mut text = String::new();
    for (n, s) in marks {
        text.push_str(&format!("{n} {s}\n"));
    }
    let _ = std::fs::write(relay_file(), text);
}

fn hex32(b: &[u8; 32]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// One signal the cell made, as the chain will carry it: `Cell::cast`
/// writes many links under one step, so the step is the unit here too —
/// the mark may only advance once EVERY link of it is in.
struct Fresh {
    neuron_key: String,
    step: u64,
    /// `(p, q, valence)` per link, in the signal's own order.
    links: Vec<([u8; 32], [u8; 32], i8)>,
}

impl Relay {
    pub fn start(shared: SharedCell, hub: NetHub, signer: Signer) -> Self {
        let relay = Relay::default();
        let (sent, pending) = (relay.sent.clone(), relay.pending.clone());

        std::thread::Builder::new()
            .name("body-relay".into())
            .spawn(move || {
                let mut marks = load_marks();
                // Said on the way into the state, not for every pass it
                // holds: the default network publishes no genesis, and that
                // is a line a second otherwise.
                let mut said_no_chain = false;
                let first_run = !relay_file().exists();
                if first_run {
                    // The past stays local; relay from now. Seeded at thread
                    // start, not at the first pass, so a body whose boot casts
                    // land inside the first five seconds still relays them.
                    let cell = shared.cell.lock().expect("shared cell poisoned");
                    for (neuron, chain) in cell.graph.chains.iter() {
                        // The mark is the next step to relay, so the seed is
                        // one past the tip. Steps start at 0 (core
                        // cell.rs `tip`), and an empty chain is marked 0 —
                        // its first signal still goes out.
                        let next = chain.entries.keys().next_back().map_or(0, |last| last + 1);
                        marks.insert(hex32(neuron), next);
                    }
                }
                loop {
                    std::thread::sleep(PASS_EVERY);
                    if !wanted() {
                        continue;
                    }
                    // The first network is where the body publishes.
                    let Some((net_name, url, chain_id, token)) = hub.first_chain() else {
                        continue;
                    };

                    // Collect links past each chain's high-water mark.
                    let fresh: Vec<Fresh> = {
                        let cell = shared.cell.lock().expect("shared cell poisoned");
                        let mut out = Vec::new();
                        for (neuron, chain) in cell.graph.chains.iter() {
                            if *neuron != signer.neuron {
                                // Signed as us or not at all: another
                                // neuron's links are not ours to publish.
                                continue;
                            }
                            let key = hex32(neuron);
                            // A chain first seen after the seed relays from
                            // step 0: it is new, so is everything it says.
                            let mark = *marks.entry(key.clone()).or_insert(0);
                            for (step, sig) in chain.entries.range(mark..) {
                                out.push(Fresh {
                                    neuron_key: key.clone(),
                                    step: *step,
                                    links: sig
                                        .links
                                        .iter()
                                        .map(|l| (l.from, l.to, l.valence.clamp(-1, 1)))
                                        .collect(),
                                });
                            }
                        }
                        out
                    };
                    if fresh.is_empty() {
                        if first_run && !marks.is_empty() && !relay_file().exists() {
                            save_marks(&marks);
                        }
                        pending.store(0, Ordering::Relaxed);
                        continue;
                    }
                    let total: u64 = fresh.iter().map(|f| f.links.len() as u64).sum();
                    // A chain that has not said who it is gets nothing signed.
                    let Some(chain_id) = chain_id else {
                        pending.store(total, Ordering::Relaxed);
                        if !said_no_chain {
                            warn!("relay: {net_name}: no chain id in /status yet");
                            said_no_chain = true;
                        }
                        continue;
                    };
                    said_no_chain = false;
                    let client = match Client::new(&url) {
                        Ok(c) => c,
                        Err(e) => {
                            warn!("relay: {e}");
                            continue;
                        }
                    };
                    // One nonce read per pass (mempool-aware), then counted
                    // up locally: the txs of a pass are ordered by nonce.
                    let mut nonce = match client.nonce(&signer.neuron) {
                        Ok(n) => n,
                        Err(e) => {
                            pending.store(total, Ordering::Relaxed);
                            warn!("relay: nonce: {e}");
                            continue;
                        }
                    };
                    let mut done = 0u64;
                    'pass: for f in &fresh {
                        for (p, q, valence) in &f.links {
                            if p == q {
                                // A self-link is local vocabulary; the chain refuses p == q.
                                done += 1;
                                continue;
                            }
                            let signed =
                                signer.cyberlink(&chain_id, nonce, *p, *q, *valence, token, 0, None);
                            let outcome = signed
                                .map_err(|e| e.to_string())
                                .and_then(|tx| client.submit(&tx).map_err(|e| e.to_string()));
                            match outcome {
                                Ok(txid) => {
                                    info!("relay: {net_name} took cyberlink nonce {nonce} txid {}", soft3::wallet::hex(&txid));
                                    nonce += 1;
                                    done += 1;
                                    sent.fetch_add(1, Ordering::Relaxed);
                                }
                                Err(e) => {
                                    warn!("relay: {e}");
                                    // Stop the pass; the next one retries from
                                    // the mark — this step is not marked done,
                                    // so the links left of it are not lost.
                                    break 'pass;
                                }
                            }
                        }
                        // Every link of this step is in (or was skipped as
                        // local): only now is the step behind the mark.
                        bump_mark(&mut marks, &f.neuron_key, f.step + 1);
                    }
                    let failed = total - done;
                    if done > 0 {
                        // The page learns the new height NOW, not on the next probe.
                        if let Ok(st) = client.status() {
                            hub.note_block(&net_name, st.height, &st.root);
                        }
                    }
                    pending.store(failed, Ordering::Relaxed);
                    save_marks(&marks);
                }
            })
            .expect("spawn body-relay");
        relay
    }
}

fn bump_mark(marks: &mut HashMap<String, u64>, key: &str, step: u64) {
    let m = marks.entry(key.to_string()).or_insert(0);
    if step > *m {
        *m = step;
    }
}

