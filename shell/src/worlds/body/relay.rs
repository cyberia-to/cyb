//! relay — the body's signals reach the chain.
//!
//! The local cell is the truth; the network is where that truth becomes
//! common. The relay tails the cell's signal chains and submits every NEW
//! link to the first configured network over its JSON bridge
//! (`POST /v1/link`). soft3 canon is one signal, one block — so a body
//! that lives (casts attention, asks questions, reads particles) IS what
//! moves the chain. h grows because you did something, not because a
//! clock ticked.
//!
//! Best-effort by design: the chain mirrors the cell, never gates it. A
//! failed submit is retried on the next pass; the high-water mark
//! (`~/cyb/relayed`: "neuron-hex last-step" lines) only advances on
//! success. First boot starts from NOW — the past is local history, not
//! a flood to replay; `relay all` (future) can push it deliberately.
//!
//! Toggle: `~/cyb/relay` ("on"/"off", default on).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

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
    let mut text = String::new();
    for (n, s) in marks {
        text.push_str(&format!("{n} {s}\n"));
    }
    let _ = std::fs::write(relay_file(), text);
}

fn hex32(b: &[u8; 32]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

impl Relay {
    pub fn start(shared: SharedCell, hub: NetHub) -> Self {
        let relay = Relay::default();
        let (sent, pending) = (relay.sent.clone(), relay.pending.clone());

        std::thread::Builder::new()
            .name("body-relay".into())
            .spawn(move || {
                let mut marks = load_marks();
                let first_run = !relay_file().exists();
                loop {
                    std::thread::sleep(PASS_EVERY);
                    if !wanted() {
                        continue;
                    }
                    // The first network is where the body publishes.
                    let Some(url) = hub
                        .0
                        .lock()
                        .ok()
                        .and_then(|v| v.first().map(|n| n.url.clone()))
                    else {
                        continue;
                    };

                    // Collect links past each chain's high-water mark.
                    let fresh: Vec<(String, u64, serde_json::Value)> = {
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
                                    out.push((
                                        key.clone(),
                                        *step,
                                        serde_json::json!({
                                            "neuron": hex32(&l.neuron),
                                            "from": hex32(&l.from),
                                            "to": hex32(&l.to),
                                            "amount": l.amount,
                                            "valence": l.valence,
                                        }),
                                    ));
                                }
                            }
                        }
                        out
                    };
                    if first_run && !fresh.is_empty() {
                        // Should not happen (marks were seeded above), but
                        // never flood on a logic slip.
                    }
                    if fresh.is_empty() {
                        if marks_dirty(&marks) {
                            save_marks(&marks);
                        }
                        pending.store(0, Ordering::Relaxed);
                        continue;
                    }

                    let mut failed = 0u64;
                    for (neuron_key, step, body) in fresh {
                        let ok = ureq::post(&format!("{url}/v1/link"))
                            .send_json(&body)
                            .is_ok();
                        if ok {
                            sent.fetch_add(1, Ordering::Relaxed);
                            let m = marks.entry(neuron_key).or_insert(0);
                            if step > *m {
                                *m = step;
                            }
                        } else {
                            failed += 1;
                            // Stop the pass on first failure per chain order;
                            // the next pass retries from the mark.
                            break;
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

/// Marks always persist after seeding so a restart never re-floods.
fn marks_dirty(marks: &HashMap<String, u64>) -> bool {
    !marks.is_empty() && !relay_file().exists()
}
