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
//! submit that failed on transport is retried on the next pass; one the
//! node REFUSED is skipped, because the next pass would earn the same no
//! and a relay that waits for it reaches the chain never again. Either
//! way the body page says so (the relay row of the process table), so a
//! body that stopped publishing can be seen to have stopped.
//!
//! The high-water mark (`~/cyb/relayed`: a `format next-step` line, then
//! "neuron-hex next-step" lines — the step the next pass starts from)
//! only advances when every link of a step is in or was skipped. A file
//! without that format line was written by the version whose number
//! meant the LAST step relayed, and is read as such. First boot starts
//! from NOW — the past is local history, not a flood to replay;
//! `relay all` (future) can push it deliberately.
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
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bevy::log::{info, warn};
use soft3::wallet::{self, Client, Signer};

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
    /// Why the relay is not reaching the chain, in the words the body
    /// page prints: the last refusal or transport failure. Cleared by a
    /// pass that went out whole.
    pub stall: Arc<Mutex<String>>,
}

/// The first line of the mark file. Without it the numbers below mean the
/// last step relayed, not the next one to relay — see [`load_marks_from`].
const MARK_FORMAT: &str = "format next-step";

fn relay_file() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    Path::new(&home).join("cyb").join("relayed")
}

fn toggle_file() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    Path::new(&home).join("cyb").join("relay")
}

fn wanted() -> bool {
    std::fs::read_to_string(toggle_file())
        .map(|s| s.trim() != "off")
        .unwrap_or(true)
}

fn load_marks() -> HashMap<String, u64> {
    load_marks_from(&relay_file())
}

/// The marks in `path`, as "the next step to relay".
///
/// A file whose first line is not [`MARK_FORMAT`] was written by the
/// version where the number meant the HIGHEST step already relayed and
/// the pass iterated `mark + 1..`. Read as-is it would re-sign every link
/// of that step onto the chain on the first pass after an upgrade, so it
/// is lifted by one: the file keeps its name and the upgraded body keeps
/// its place in the sequence.
fn load_marks_from(path: &Path) -> HashMap<String, u64> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return HashMap::new();
    };
    let next_step = text.lines().next().map(str::trim) == Some(MARK_FORMAT);
    text.lines()
        .filter(|l| l.trim() != MARK_FORMAT)
        .filter_map(|l| {
            let (n, s) = l.split_once(' ')?;
            let step: u64 = s.trim().parse().ok()?;
            Some((n.to_string(), if next_step { step } else { step + 1 }))
        })
        .collect()
}

fn save_marks(marks: &HashMap<String, u64>) {
    save_marks_to(&relay_file(), marks);
}

fn save_marks_to(path: &Path, marks: &HashMap<String, u64>) {
    if marks.is_empty() {
        // An empty file would still make the next boot not a first run:
        // the mark would fall back to 0 and the whole local past would be
        // queued for the chain — the flood this module promises not to
        // replay. Nothing to record is recorded as nothing.
        return;
    }
    let mut text = format!("{MARK_FORMAT}\n");
    for (n, s) in marks {
        text.push_str(&format!("{n} {s}\n"));
    }
    let _ = std::fs::write(path, text);
}

/// The mark a first run seeds for a chain whose highest step is `tip`: one
/// past it, since the mark is the next step to relay. An empty chain
/// (`None`) is seeded 0 — steps start at 0 (core cell.rs `tip`), so a new
/// body's very first signal still goes out.
fn seed_mark(tip: Option<u64>) -> u64 {
    tip.map_or(0, |last| last + 1)
}

/// Would the next pass earn the same answer? A refusal is the node's
/// verdict on these bytes — a duplicate cyberlink, a valence the rules do
/// not take, a fee under a raised floor, a key that cannot sign — and
/// retrying it for ever is how a relay stops relaying. Transport and
/// timeout are the other kind: nothing was judged, so try again.
///
/// A refusal that names the nonce is transient on purpose: the pay in
/// sigma signs from the same sequence, so the number moved under us. The
/// next pass reads `/nonce` again and the link keeps its place.
fn permanent(e: &wallet::Error) -> bool {
    match e {
        wallet::Error::Rejected(why) => !why.contains("nonce"),
        wallet::Error::Rule(_) | wallet::Error::Encoding(_) | wallet::Error::Key(_) => true,
        wallet::Error::Http(_) | wallet::Error::Timeout(_) => false,
    }
}

/// The relay's line in the body page's process table: `(sent, waiting,
/// note)`. A stall belongs on the page, not only in a warn! line nobody
/// is watching.
pub fn page_cells(sent: u64, pending: u64, stall: &str) -> [String; 3] {
    let note = if !stall.is_empty() {
        stall.to_string()
    } else if pending > 0 {
        format!("{pending} waiting")
    } else {
        "relaying".into()
    };
    [format!("{sent}"), format!("{pending}"), note]
}

fn note_stall(stall: &Arc<Mutex<String>>, say: &str) {
    if let Ok(mut s) = stall.lock() {
        s.clear();
        s.push_str(say);
    }
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
        let stall = relay.stall.clone();

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
                        let tip = chain.entries.keys().next_back().copied();
                        marks.insert(hex32(neuron), seed_mark(tip));
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
                        note_stall(&stall, &format!("{net_name}: no chain id yet"));
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
                            note_stall(&stall, &format!("{net_name}: {e}"));
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
                            note_stall(&stall, &format!("{net_name}: nonce: {e}"));
                            continue;
                        }
                    };
                    let mut done = 0u64;
                    // Accepted, as against skipped: only a tx the node took
                    // can have moved the height.
                    let mut took = 0u64;
                    let mut refused = String::new();
                    let mut waiting = String::new();
                    'pass: for f in &fresh {
                        for (p, q, valence) in &f.links {
                            if p == q {
                                // A self-link is local vocabulary; the chain refuses p == q.
                                done += 1;
                                continue;
                            }
                            let outcome = signer
                                .cyberlink(&chain_id, nonce, *p, *q, *valence, token, 0, None)
                                .and_then(|tx| client.submit(&tx));
                            match outcome {
                                Ok(txid) => {
                                    info!("relay: {net_name} took cyberlink nonce {nonce} txid {}", wallet::hex(&txid));
                                    nonce += 1;
                                    done += 1;
                                    took += 1;
                                    sent.fetch_add(1, Ordering::Relaxed);
                                }
                                Err(e) if permanent(&e) => {
                                    // The node judged these bytes and said
                                    // no; it will say no again. Skip the
                                    // link so the mark can pass this step:
                                    // waiting for a no to become a yes is
                                    // how every later signal is lost. The
                                    // nonce is NOT spent — the tx was not
                                    // taken — so the next link reuses it.
                                    warn!(
                                        "relay: {net_name} refused step {}: {e} - skipped",
                                        f.step
                                    );
                                    refused =
                                        format!("{net_name} refused step {}: {e}", f.step);
                                    done += 1;
                                }
                                Err(e) => {
                                    // Nothing was judged: transport, or a
                                    // nonce the pay in sigma spent first.
                                    // Stop the pass; the next one retries
                                    // from the mark — this step is not
                                    // marked done, so its links are not lost.
                                    warn!("relay: {e}");
                                    waiting = format!("{net_name}: {e} - retrying");
                                    break 'pass;
                                }
                            }
                        }
                        // Every link of this step is in, or was skipped —
                        // as local vocabulary, or as bytes the node will
                        // refuse for ever. Only now is the step behind the
                        // mark; a transient failure above broke the pass
                        // before this line.
                        bump_mark(&mut marks, &f.neuron_key, f.step + 1);
                    }
                    let failed = total - done;
                    if took > 0 {
                        // The page learns the new height NOW, not on the next probe.
                        if let Ok(st) = client.status() {
                            hub.note_block(&net_name, st.height, &st.root);
                        }
                    }
                    // What the page says: what is still waiting first, then
                    // the last refusal — a skipped link is a hole in what
                    // this body published and must not pass unsaid.
                    let say = if !waiting.is_empty() {
                        waiting
                    } else if !refused.is_empty() {
                        refused
                    } else {
                        String::new()
                    };
                    note_stall(&stall, &say);
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


#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("cyb-relay-{name}-{}", std::process::id()));
        let _ = std::fs::remove_file(&p);
        p
    }

    /// The mark is the NEXT step to relay: a body whose chain tops out at
    /// step 7 relays from 8, and a body that has never signalled relays
    /// from 0 — step 0 is a real signal, not one to skip.
    #[test]
    fn the_seed_is_one_past_the_tip() {
        assert_eq!(seed_mark(None), 0);
        assert_eq!(seed_mark(Some(0)), 1);
        assert_eq!(seed_mark(Some(7)), 8);
    }

    #[test]
    fn marks_survive_the_round_trip() {
        let path = tmp("roundtrip");
        let mut marks = HashMap::new();
        marks.insert("aa".repeat(32), 9u64);
        save_marks_to(&path, &marks);
        let text = std::fs::read_to_string(&path).expect("the file was written");
        assert!(text.starts_with(MARK_FORMAT), "no format line: {text}");
        assert_eq!(load_marks_from(&path), marks);
        let _ = std::fs::remove_file(&path);
    }

    /// The file written before the meaning flipped: its number was the
    /// highest step already relayed, and the pass iterated `mark + 1..`.
    /// Read as "the next step" it would re-sign every link of that step
    /// onto the chain on the first pass after an upgrade.
    #[test]
    fn an_old_mark_file_is_lifted_by_one() {
        let path = tmp("v1");
        let neuron = "bb".repeat(32);
        std::fs::write(&path, format!("{neuron} 4\n")).expect("the v1 file was written");
        assert_eq!(load_marks_from(&path).get(&neuron), Some(&5));
        // And once this version has written it, 5 means 5 for good.
        save_marks_to(&path, &load_marks_from(&path));
        assert_eq!(load_marks_from(&path).get(&neuron), Some(&5));
        let _ = std::fs::remove_file(&path);
    }

    /// Nothing to record is recorded as nothing: an empty file would make
    /// the next boot not a first run and queue the whole local past.
    #[test]
    fn an_empty_mark_map_is_not_written() {
        let path = tmp("empty");
        save_marks_to(&path, &HashMap::new());
        assert!(!path.exists(), "an empty map must leave no file");
    }

    #[test]
    fn the_mark_only_moves_forward() {
        let mut marks = HashMap::new();
        bump_mark(&mut marks, "cc", 3);
        assert_eq!(marks.get("cc"), Some(&3));
        bump_mark(&mut marks, "cc", 2);
        assert_eq!(marks.get("cc"), Some(&3), "a late step must not rewind the mark");
        bump_mark(&mut marks, "cc", 4);
        assert_eq!(marks.get("cc"), Some(&4));
    }

    /// A refusal is the node's verdict on these bytes and the next pass
    /// earns it again, so the link is skipped; a transport failure judged
    /// nothing. A nonce the pay in sigma spent first is the second kind:
    /// the sequence moved, not the link.
    #[test]
    fn a_refusal_is_permanent_and_transport_is_not() {
        assert!(permanent(&wallet::Error::Rejected(
            "duplicate cyberlink".into()
        )));
        assert!(permanent(&wallet::Error::Rule("fee below floor".into())));
        assert!(!permanent(&wallet::Error::Rejected(
            "bad nonce 7: expected 9".into()
        )));
        assert!(!permanent(&wallet::Error::Http("connection refused".into())));
        assert!(!permanent(&wallet::Error::Timeout("90s".into())));
    }

    /// The stall is what the page prints, so it is never silent about a
    /// relay that is not moving.
    #[test]
    fn the_page_says_the_stall() {
        assert_eq!(page_cells(4, 0, "")[2], "relaying");
        assert_eq!(page_cells(4, 2, "")[2], "2 waiting");
        let refused = "pussy refused step 3: rejected: duplicate";
        assert_eq!(page_cells(4, 0, refused)[2], refused);
        assert_eq!(page_cells(4, 0, refused)[0], "4");
    }
}
