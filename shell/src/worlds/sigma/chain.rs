//! The real money: this body's balance ON THE CHAIN, and the flow that
//! moves it. Everything here talks to the first configured network
//! (pussy) over its native endpoints: `GET /balance/<neuron>` for the
//! state, `POST /v1/pay` for transfers. One signal, one block — the pay
//! response carries the height and root it finalized in, and that pair
//! is shown as the receipt.
//!
//! The last good snapshot is painted immediately; a fetch never blanks
//! the page. See [[cyb/parts/live]].
//!
//! The subsidy side of the loop (proofs -> testpussy) lives on the node
//! per tru/specs/rewards.md §8; here is where the earned balance becomes
//! visible and spendable.

use std::sync::{Arc, Mutex};

use bevy::prelude::*;

/// What the chain last told us. Threads write, the page reads.
#[derive(Clone, Debug, Default)]
pub struct ChainMoneyState {
    pub balance: u64,
    pub supply: u64,
    pub height: u64,
    /// The last receipt: "paid N to X - block h=.. root .."
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
                    .and_then(|mut r| r.body_mut().read_to_string().ok());
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
                    let h = field(st, "height:");
                    if h > 0 {
                        s.height = h;
                    }
                    s.error.clear();
                }
                match bal {
                    Some(body) => {
                        s.balance = field(&body, "balance:");
                        s.supply = field(&body, "supply:");
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

    /// Send `amount` to `to` (label or hex), then re-read the balance.
    /// The receipt shows the block the pay finalized in — that IS the
    /// finality: one signal, one block, no waiting period.
    pub fn pay(&self, url: String, neuron_hex: String, to: String, amount: u64) {
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
                let body = serde_json::json!({
                    "neuron": neuron_hex,
                    "to": to,
                    "amount": amount,
                });
                let agent = crate::worlds::body::networks::agent_raw();
                let sent = agent.post(&format!("{url}/v1/pay")).send_json(&body);
                {
                    let mut s = slot.lock().expect("chain money");
                    s.busy = false;
                    s.version += 1;
                    match sent {
                        Ok(mut r) => {
                            let status = r.status();
                            let v: Option<serde_json::Value> =
                                r.body_mut().read_json::<serde_json::Value>().ok();
                            if status.is_success()
                                && v.as_ref()
                                    .and_then(|x| x.get("ok"))
                                    .and_then(|x| x.as_bool())
                                    == Some(true)
                            {
                                let v = v.unwrap();
                                let h = v.get("height").and_then(|x| x.as_u64()).unwrap_or(0);
                                let root = v
                                    .get("root")
                                    .and_then(|x| x.as_str())
                                    .unwrap_or("")
                                    .chars()
                                    .take(8)
                                    .collect::<String>();
                                s.receipt = format!(
                                    "paid {amount} to {to} - final in block h={h} {root}.."
                                );
                                s.error.clear();
                            } else {
                                s.error = v
                                    .as_ref()
                                    .and_then(|x| x.get("error"))
                                    .and_then(|x| x.as_str())
                                    .map(str::to_string)
                                    .unwrap_or_else(|| format!("pay refused ({status})"));
                            }
                        }
                        Err(e) => {
                            s.error = crate::worlds::body::networks::short_http_err(&e.to_string());
                        }
                    }
                }
                me.refresh(url, neuron_hex);
            })
            .expect("spawn sigma-pay");
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
