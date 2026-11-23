//! The real money: this body's balance ON THE CHAIN, and the flow that
//! moves it. Everything here talks to the first configured network over
//! the chain's HTTP surface (soft3/specs/chain-wire.md §7):
//! `GET /balance/<neuron>` and `GET /status` for the state, and a signed
//! kind-1 transfer through `POST /tx` for payments — NSIG1 over the chain
//! id, nonce from `/nonce/<neuron>`, fee at the floor. The receipt is the
//! block the transfer was included in, read back from `/tx/<txid>`.
//!
//! The last good snapshot is painted immediately; a fetch never blanks
//! the page. See [[cyb/parts/live]].
//!
//! The subsidy side of the loop (proofs -> testpussy) lives on the node
//! per tru/specs/rewards.md §8; here is where the earned balance becomes
//! visible and spendable.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use bevy::prelude::*;
use soft3::wallet::{self, Client, Signer, TxStatus};

/// How long a pay waits for its block before reporting "submitted".
const INCLUSION_WAIT: Duration = Duration::from_secs(90);

/// Said when the node's `/status` has published no genesis: nothing can be
/// signed against a chain that has not named itself.
const NO_CHAIN_ID: &str = "chain id unknown: the node's /status has no genesis yet";

/// What the chain last told us. Threads write, the page reads.
#[derive(Clone, Debug, Default)]
pub struct ChainMoneyState {
    pub balance: u64,
    pub supply: u64,
    pub height: u64,
    /// The last receipt: "paid N to X - txid .. in block h=.."
    pub receipt: String,
    pub error: String,
    /// A balance refresh is in flight.
    pub busy: bool,
    /// A pay is in flight — from the signing until its block, which is up
    /// to [`INCLUSION_WAIT`]. Separate from `busy` on purpose: a pay must
    /// not freeze the page's own polling, and a second pay must be told.
    pub paying: bool,
    /// Bumped on every update so the page knows to repaint.
    pub version: u64,
}

#[derive(Resource, Clone)]
pub struct ChainMoney(pub Arc<Mutex<ChainMoneyState>>);

impl Default for ChainMoney {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(load_cache())))
    }
}

impl ChainMoney {
    pub fn snapshot(&self) -> ChainMoneyState {
        self.0.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Ask the chain for this neuron's balance, off-thread.
    /// The last snapshot stays on screen; busy never blanks it.
    pub fn refresh(&self, url: String, neuron_hex: String) {
        let slot = self.0.clone();
        if let Ok(mut s) = slot.lock() {
            if s.busy {
                return;
            }
            s.busy = true;
        }
        std::thread::Builder::new()
            .name("sigma-balance".into())
            .spawn(move || {
                // Height from /status (always fast). Balance from /balance
                // with a short timeout — that route hangs on some nodes.
                let status = crate::worlds::body::networks::agent_quick()
                    .get(&format!("{url}/status"))
                    .call()
                    .ok()
                    .and_then(|mut r| r.body_mut().read_to_string().ok())
                    .and_then(|body| wallet::Status::parse(&body).ok());
                let bal = crate::worlds::body::networks::agent_quick()
                    .get(&format!("{url}/balance/{neuron_hex}"))
                    .call()
                    .ok()
                    .and_then(|mut r| {
                        if r.status().is_success() {
                            r.body_mut().read_to_string().ok()
                        } else {
                            None
                        }
                    });
                let field = |body: &str, key: &str| -> u64 {
                    body.lines()
                        .find(|l| l.trim_start().starts_with(key))
                        .and_then(|l| l.split_once(':'))
                        .and_then(|(_, v)| v.trim().parse().ok())
                        .unwrap_or(0)
                };
                let mut s = slot.lock().expect("chain money");
                s.busy = false;
                s.version += 1;
                if let Some(st) = &status {
                    if st.height > 0 {
                        s.height = st.height;
                    }
                    if let Some(supply) = st.field("supply").and_then(|v| v.parse().ok()) {
                        s.supply = supply;
                    }
                    s.error.clear();
                }
                match bal {
                    Some(body) => {
                        s.balance = body
                            .trim()
                            .parse()
                            .unwrap_or_else(|_| field(&body, "balance:"));
                        let supply = field(&body, "supply:");
                        if supply > 0 {
                            s.supply = supply;
                        }
                        let h = field(&body, "height:");
                        if h > 0 {
                            s.height = h;
                        }
                        s.error.clear();
                        save_cache(&s);
                    }
                    None if status.is_none() => {
                        s.error = "chain unreachable".into();
                    }
                    None => {
                        // Chain is up; this neuron's balance just isn't served.
                        s.error.clear();
                    }
                }
            })
            .expect("spawn sigma-balance");
    }

    /// Send `amount` of the chain's native `token` to the 64-hex neuron
    /// `to` as a signed transfer, then re-read the balance. The receipt
    /// names the block the transfer was included in — that is the
    /// finality phase 1 offers (chain.md §5.2: an hour deep, it is final).
    ///
    /// `Err` is a refusal the caller must say out loud: a recipient that
    /// is not an address, a chain that has not said who it is, or a pay
    /// already waiting for its block. Value leaves this body only through
    /// the `Ok` path — and a submit the node never answered is reported
    /// there too, as a receipt naming the txid, because the transfer may
    /// already be on the chain (see [`unanswered`]). A failed pay is only
    /// ever one the node judged.
    pub fn pay(
        &self,
        url: String,
        signer: Signer,
        chain_id: Option<[u8; 32]>,
        token: [u8; 32],
        to: String,
        amount: u64,
    ) -> Result<(), String> {
        // Checked before anything is signed: `wallet::card` turns a label
        // the chain does not know into hemera(label), an address nobody
        // holds, and a signed transfer to it is value burned.
        recipient(&to)?;
        // The default network publishes no genesis, so this is the refusal
        // every pay meets today. It belongs at the prompt that asked for
        // the pay, beside the recipient check — not in sigma's error field
        // after the commander has already said "receipt lands in sigma".
        let chain_id = chain_id.ok_or(NO_CHAIN_ID)?;
        let slot = self.0.clone();
        let me = self.clone();
        if let Ok(mut s) = slot.lock() {
            if s.paying {
                return Err("a pay is already waiting for its block".into());
            }
            s.paying = true;
        }
        std::thread::Builder::new()
            .name("sigma-pay".into())
            .spawn(move || {
                let neuron_hex = signer.neuron_hex();
                let outcome = send(&url, &signer, chain_id, token, &to, amount);
                let good = outcome.is_ok();
                {
                    let mut s = slot.lock().expect("chain money");
                    s.paying = false;
                    s.version += 1;
                    match outcome {
                        Ok(receipt) => {
                            s.receipt = receipt;
                            s.error.clear();
                        }
                        Err(e) => {
                            s.error = crate::worlds::body::networks::short_http_err(&e);
                        }
                    }
                }
                // Only a transfer that happened moved the balance, and a
                // refresh would wipe the error a failed one just wrote.
                if good {
                    me.refresh(url, neuron_hex);
                }
            })
            .expect("spawn sigma-pay");
        Ok(())
    }
}

/// The 32-byte recipient of a transfer, or why the typed word is not one.
/// Only a full address is accepted: labels are resolved by no registry
/// this phase has, and `wallet::card` would hash one into an address
/// nobody can spend from (soft3 wallet/mod.rs `card`).
fn recipient(to: &str) -> Result<[u8; 32], String> {
    wallet::parse32(to.trim())
        .map_err(|_| format!("'{to}' is not an address - pay needs the 64-hex neuron id"))
}

/// Build, sign, submit, wait. A failure the node pronounced is the error
/// the page shows; a submit it never answered is not one — see
/// [`unanswered`].
fn send(
    url: &str,
    signer: &Signer,
    chain_id: [u8; 32],
    token: [u8; 32],
    to: &str,
    amount: u64,
) -> Result<String, String> {
    let to_id = recipient(to)?;
    let client = Client::new(url).map_err(|e| e.to_string())?;
    let sign = |nonce: u64| {
        signer
            .transfer(&chain_id, nonce, token, amount, to_id, None)
            .map_err(|e| e.to_string())
    };
    let nonce = client.nonce(&signer.neuron).map_err(|e| format!("nonce: {e}"))?;
    let mut signed = sign(nonce)?;
    // The relay signs from the same sequence and reads `/nonce` once per
    // pass: a pass in flight may already have spent ours. The node says so,
    // and the person did nothing to cause it — read the chain again and
    // sign once more, rather than show them a bad-nonce error.
    let mut submitted = client.submit(&signed);
    if matches!(&submitted, Err(wallet::Error::Rejected(why)) if why.contains("nonce")) {
        let nonce = client.nonce(&signer.neuron).map_err(|e| format!("nonce: {e}"))?;
        signed = sign(nonce)?;
        submitted = client.submit(&signed);
    }
    let txid = signed.txid_hex();
    let short: String = txid.chars().take(8).collect();
    if let Err(e) = submitted {
        // A refusal is the node's verdict on these bytes: nothing left
        // this body, and the error is the whole truth. A submit that was
        // never answered is the other kind — the node may hold the tx, so
        // reporting a clean failure would send the person to pay again,
        // and that second pay is a second, genuinely distinct transfer.
        if !unanswered(&e) {
            return Err(e.to_string());
        }
        return Ok(maybe_sent(amount, to, &short, &e.to_string()));
    }
    // Past this line the transfer has left this body. Whatever the poll
    // then says — a timeout, a node that restarted under it, a status
    // that is not inclusion — it is not a failed pay: the receipt names
    // the txid so the person can look the transfer up, and the balance is
    // re-read. Every variant is spelled out, so a new one is a compile
    // error here rather than a refusal read as "paid".
    let said = match client.wait(&signed.txid, INCLUSION_WAIT) {
        Ok(TxStatus::Included { height, .. }) => {
            return Ok(format!("paid {amount} to {to} - tx {short}.. in block h={height}"))
        }
        Ok(TxStatus::Mempool) => "admitted, no block yet".to_string(),
        Ok(TxStatus::Unknown) => "the node does not know the txid".to_string(),
        Err(e) => e.to_string(),
    };
    Ok(format!(
        "sent {amount} to {to} - tx {short}.. - no block to show: {said} - look the tx up"
    ))
}

/// Did the node judge the bytes, or did the POST simply go unanswered?
/// `Http` and `Timeout` judged nothing: the node may well have taken the
/// transfer and the body cannot tell. `Encoding`, `Rule` and `Key` never
/// left this body, and a `Rejected` is the mempool's verdict — except the
/// one soft3's client builds out of a non-2xx status (wallet/client.rs
/// `submit`), where a 5xx is a gateway's answer and no verdict at all.
fn unanswered(e: &wallet::Error) -> bool {
    match e {
        wallet::Error::Http(_) | wallet::Error::Timeout(_) => true,
        wallet::Error::Rejected(why) => why.starts_with("HTTP 5"),
        wallet::Error::Encoding(_) | wallet::Error::Rule(_) | wallet::Error::Key(_) => false,
    }
}

/// The receipt of a submit nobody answered: it names the txid, says the
/// transfer may be on the chain, and sends the person to the balance —
/// which the caller re-reads — instead of to a second pay. Never the word
/// "failed": this is the one outcome where the body does not know.
fn maybe_sent(amount: u64, to: &str, short: &str, why: &str) -> String {
    format!(
        "sent {amount} to {to} - tx {short}.. - the node did not answer the submit \
         ({why}) - the transfer may already be on the chain: read the balance \
         and look the tx up before paying again"
    )
}

/// The identity neuron as the chain spells it.
pub fn neuron_hex(who: &crate::worlds::identity::Identity) -> String {
    who.neuron.iter().map(|b| format!("{b:02x}")).collect()
}

fn cache_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::PathBuf::from(home)
        .join("cyb")
        .join("sigma-chain.json")
}

fn load_cache() -> ChainMoneyState {
    let Ok(raw) = std::fs::read_to_string(cache_path()) else {
        return ChainMoneyState::default();
    };
    let mut s = ChainMoneyState::default();
    for line in raw.lines() {
        if let Some(v) = line.strip_prefix("balance:") {
            s.balance = v.trim().parse().unwrap_or(0);
        } else if let Some(v) = line.strip_prefix("supply:") {
            s.supply = v.trim().parse().unwrap_or(0);
        } else if let Some(v) = line.strip_prefix("height:") {
            s.height = v.trim().parse().unwrap_or(0);
        }
    }
    s
}

fn save_cache(s: &ChainMoneyState) {
    let body = format!(
        "balance: {}\nsupply: {}\nheight: {}\n",
        s.balance, s.supply, s.height
    );
    let _ = std::fs::write(cache_path(), body);
}

/// First network's URL, if any is configured.
pub fn chain_url(hub: &crate::worlds::body::networks::NetHub) -> Option<String> {
    hub.states
        .lock()
        .ok()
        .and_then(|v| v.first().map(|n| n.url.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The guard of pay: a word the chain does not know must never be
    /// hashed into an address nobody holds.
    #[test]
    fn a_label_is_not_a_recipient() {
        for label in ["alice", "", "0x1234", "31284576"] {
            let err = recipient(label).expect_err("a label must be refused");
            assert!(err.contains("not an address"), "{label}: {err}");
        }
    }

    /// A pay against a node that has published no genesis is refused at
    /// the prompt that asked for it. The default network is exactly that
    /// node today, so this is the path every pay takes: the commander must
    /// not answer "receipt lands in sigma" for a transfer that will never
    /// be signed.
    #[test]
    fn an_unknown_chain_id_is_refused_before_the_thread() {
        let money = ChainMoney(Arc::new(Mutex::new(ChainMoneyState::default())));
        let signer = Signer::from_secret(&[7u8; 32]).expect("a key");
        let err = money
            .pay(
                "http://127.0.0.1:1".into(),
                signer,
                None,
                [0u8; 32],
                "31284576".repeat(8),
                1,
            )
            .expect_err("no chain id, no pay");
        assert!(err.contains("genesis"), "{err}");
        assert!(
            !money.snapshot().paying,
            "a refused pay must leave nothing in flight"
        );
    }

    /// The split at the submit: a node that said no judged these bytes and
    /// nothing left this body, so that is a failed pay. A transport
    /// failure, a timeout, or a 5xx from whatever stands in front of the
    /// node judged nothing — reported as a failure, the obvious next move
    /// is a second pay, and that one is a second real transfer.
    #[test]
    fn an_unanswered_submit_is_not_a_refusal() {
        assert!(unanswered(&wallet::Error::Http("connection reset".into())));
        assert!(unanswered(&wallet::Error::Timeout("90s".into())));
        assert!(unanswered(&wallet::Error::Rejected(
            "HTTP 502: bad gateway".into()
        )));
        assert!(!unanswered(&wallet::Error::Rejected(
            "bad nonce 7: expected 9".into()
        )));
        assert!(!unanswered(&wallet::Error::Rule("fee below floor".into())));
        assert!(!unanswered(&wallet::Error::Key("no key".into())));
        assert!(!unanswered(&wallet::Error::Encoding("short tx".into())));
    }

    /// What that receipt says: the txid to look up, the balance to read,
    /// and not one word that reads as a pay that did not happen.
    #[test]
    fn the_unanswered_receipt_names_the_txid() {
        let said = maybe_sent(7, "31284576", "deadbeef", "http: connection reset");
        assert!(said.contains("deadbeef.."), "the txid must be named: {said}");
        assert!(said.contains("may already be on the chain"), "{said}");
        assert!(said.contains("balance"), "{said}");
        assert!(!said.contains("failed"), "{said}");
    }

    #[test]
    fn a_64_hex_neuron_id_is_a_recipient() {
        let hex = "31284576".repeat(8);
        let id = recipient(&format!("  {hex}  ")).expect("64 hex digits are an address");
        assert_eq!(id[0], 0x31);
        assert_eq!(id[7], 0x76);
        assert_eq!(recipient(&format!("0x{hex}")).expect("0x prefixed too"), id);
    }
}
