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
//! (`~/cyb/relayed`: "neuron-hex last-step" lines) only advances on
//! success. First boot starts from NOW — the past is local history, not
//! a flood to replay; `relay all` (future) can push it deliberately.
//! Nothing is signed until the node has published its chain id in
//! `/status` — a tx bound to a guessed chain is a tx for nobody.
//!
//! Toggle: `~/cyb/relay` ("on"/"off", default on).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

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
    /// Why the last pass stopped, if it did ("" when the pass was clean).
    pub last_error: Arc<Mutex<String>>,
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
    let mut text = String::new();
    for (n, s) in marks {
        text.push_str(&format!("{n} {s}\n"));
    }
    let _ = std::fs::write(relay_file(), text);
}

fn hex32(b: &[u8; 32]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// One link the cell made, as the chain will carry it.
struct Fresh {
    neuron_key: String,
    step: u64,
    p: [u8; 32],
    q: [u8; 32],
    valence: i8,
}

impl Relay {
    pub fn start(shared: SharedCell, hub: NetHub, signer: Signer) -> Self {
        let relay = Relay::default();
        let (sent, pending, last_error) =
            (relay.sent.clone(), relay.pending.clone(), relay.last_error.clone());

        std::thread::Builder::new()
            .name("body-relay".into())
            .spawn(move || {
                let mut marks = load_marks();
                let first_run = !relay_file().exists();
                let mut note = |m: String| {
                    if let Ok(mut e) = last_error.lock() {
                        *e = m;
                    }
                };
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
                            let key = hex32(neuron);
                            let mark = marks.get(&key).copied().unwrap_or_else(|| {
                                if first_run {
                                    // The past stays local; relay from now.
                                    chain.entries.keys().next_back().copied().unwrap_or(0)
                                } else {
                                    0
                                }
                            });
                            marks.entry(key.clone()).or_insert(mark);
                            for (step, sig) in chain.entries.range(mark + 1..) {
                                for l in &sig.links {
                                    out.push(Fresh {
                                        neuron_key: key.clone(),
                                        step: *step,
                                        p: l.from,
                                        q: l.to,
                                        valence: l.valence.clamp(-1, 1),
                                    });
                                }
                            }
                        }
                        out
                    };
                    if fresh.is_empty() {
                        if marks_dirty(&marks) {
                            save_marks(&marks);
                        }
                        pending.store(0, Ordering::Relaxed);
                        continue;
                    }
                    // A chain that has not said who it is gets nothing signed.
                    let Some(chain_id) = chain_id else {
                        pending.store(fresh.len() as u64, Ordering::Relaxed);
                        note(format!("{net_name}: no chain id in /status yet"));
                        continue;
                    };
                    let client = match Client::new(&url) {
                        Ok(c) => c,
                        Err(e) => {
                            note(e.to_string());
                            continue;
                        }
                    };
                    // One nonce read per pass (mempool-aware), then counted
                    // up locally: the txs of a pass are ordered by nonce.
                    let mut nonce = match client.nonce(&signer.neuron) {
                        Ok(n) => n,
                        Err(e) => {
                            pending.store(fresh.len() as u64, Ordering::Relaxed);
                            note(format!("nonce: {e}"));
                            continue;
                        }
                    };
                    let total = fresh.len() as u64;
                    let mut done = 0u64;
                    for f in fresh {
                        if f.p == f.q {
                            // A self-link is local vocabulary; the chain refuses p == q.
                            bump_mark(&mut marks, &f.neuron_key, f.step);
                            done += 1;
                            continue;
                        }
                        let signed = signer.cyberlink(&chain_id, nonce, f.p, f.q, f.valence, token, 0, None);
                        let outcome = signed
                            .map_err(|e| e.to_string())
                            .and_then(|tx| client.submit(&tx).map_err(|e| e.to_string()));
                        match outcome {
                            Ok(_txid) => {
                                nonce += 1;
                                done += 1;
                                sent.fetch_add(1, Ordering::Relaxed);
                                bump_mark(&mut marks, &f.neuron_key, f.step);
                            }
                            Err(e) => {
                                note(e);
                                // Stop the pass; the next one retries from
                                // the mark — order within a chain holds.
                                break;
                            }
                        }
                    }
                    let any = done > 0;
                    let failed = total - done;
                    if any {
                        note(String::new());
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

/// Marks always persist after seeding so a restart never re-floods.
fn marks_dirty(marks: &HashMap<String, u64>) -> bool {
    !marks.is_empty() && !relay_file().exists()
}
