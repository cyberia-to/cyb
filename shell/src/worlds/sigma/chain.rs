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

/// What the chain last told us. Threads write, the page reads.
#[derive(Clone, Debug, Default)]
pub struct ChainMoneyState {
    pub balance: u64,
    pub supply: u64,
    pub height: u64,
    /// The tip block id as the chain spells it (empty on the chaosnet).
    pub tip: String,
    /// The state root of the tip.
    pub root: String,
    /// The last receipt: "paid N to X - txid .. in block h=.."
    pub receipt: String,
    pub error: String,
    pub busy: bool,
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
                    s.tip = st.tip.clone();
                    s.root = st.root.clone();
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

    /// Send `amount` of the chain's native `token` to `to` (label or hex)
    /// as a signed transfer, then re-read the balance. The receipt names
    /// the block the transfer was included in — that is the finality
    /// phase 1 offers (chain.md §5.2: an hour deep, it is final).
    pub fn pay(
        &self,
        url: String,
        signer: Signer,
        chain_id: Option<[u8; 32]>,
        token: [u8; 32],
        to: String,
        amount: u64,
    ) {
        let slot = self.0.clone();
        let me = self.clone();
        if let Ok(mut s) = slot.lock() {
            if s.busy {
                return;
            }
            s.busy = true;
        }
        std::thread::Builder::new()
            .name("sigma-pay".into())
            .spawn(move || {
                let neuron_hex = signer.neuron_hex();
                let outcome = send(&url, &signer, chain_id, token, &to, amount);
                {
                    let mut s = slot.lock().expect("chain money");
                    s.busy = false;
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
                me.refresh(url, neuron_hex);
            })
            .expect("spawn sigma-pay");
    }
}

/// Build, sign, submit, wait. Every step's failure is the error the page shows.
fn send(
    url: &str,
    signer: &Signer,
    chain_id: Option<[u8; 32]>,
    token: [u8; 32],
    to: &str,
    amount: u64,
) -> Result<String, String> {
    let chain_id = chain_id.ok_or("chain id unknown: the node's /status has no genesis yet")?;
    let client = Client::new(url).map_err(|e| e.to_string())?;
    let nonce = client.nonce(&signer.neuron).map_err(|e| format!("nonce: {e}"))?;
    let signed = signer
        .transfer(&chain_id, nonce, token, amount, wallet::card(to), None)
        .map_err(|e| e.to_string())?;
    client.submit(&signed).map_err(|e| e.to_string())?;
    let txid = signed.txid_hex();
    let short: String = txid.chars().take(8).collect();
    match client.wait(&signed.txid, INCLUSION_WAIT) {
        Ok(TxStatus::Included { height, .. }) => {
            Ok(format!("paid {amount} to {to} - tx {short}.. in block h={height}"))
        }
        Ok(_) | Err(wallet::Error::Timeout(_)) => {
            Ok(format!("paid {amount} to {to} - tx {short}.. submitted, awaiting a block"))
        }
        Err(e) => Err(e.to_string()),
    }
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
        } else if let Some(v) = line.strip_prefix("tip:") {
            s.tip = v.trim().to_string();
        }
    }
    s
}

fn save_cache(s: &ChainMoneyState) {
    let body = format!(
        "balance: {}\nsupply: {}\nheight: {}\ntip: {}\n",
        s.balance, s.supply, s.height, s.tip
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
