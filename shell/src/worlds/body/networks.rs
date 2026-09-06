//! networks — the chains this body talks to, and what the talk costs.
//!
//! Each configured network is polled on its own cadence: one `/status`
//! probe per step, the soft3 plain-text form (`height:`, `bbg-root:`).
//! The body page shows, per network, the last known chain state (height
//! and state root), how long ago the last successful step was, and the
//! session's exchange with that chain — bytes out, bytes in, measured on
//! our own requests, not invented.
//!
//! The latest state root of the FIRST network is the beacon: the prover
//! binds every ticket's public vector to it, which is the offline shape
//! of "prove the last block" — tickets from two bodies watching the same
//! chain sample the same beacon, and multi-environment runs can check
//! exactly that.
//!
//! Config lives in `~/cyb/networks.toml`, edited by the commander:
//! `net add <name> <url>` / `net set <name> <url>` / `net rm <name>` /
//! `net` to list. spacepussy-test is the default first network.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// How often each network is stepped.
const STEP_EVERY: Duration = Duration::from_secs(15);
/// A failed network retries sooner; a chain outage should not hide for
/// a whole quiet period.
const RETRY_EVERY: Duration = Duration::from_secs(5);

#[derive(Clone, Debug, Default)]
pub struct NetState {
    pub name: String,
    pub url: String,
    /// Last seen chain height (0 = never reached).
    pub height: u64,
    /// Last seen state root (bbg-root), hex.
    pub root: String,
    /// What the last step did — "ok h=3" or the error, verbatim short.
    pub last_step: String,
    pub ok: bool,
    pub last_sync: Option<Instant>,
    /// Session exchange with this chain, bytes.
    pub rx: u64,
    pub tx: u64,
}

/// The hub every reader shares: sync thread writes, UI and prover read.
#[derive(Clone, Default)]
pub struct NetHub(pub Arc<Mutex<Vec<NetState>>>);

impl NetHub {
    pub fn snapshot(&self) -> Vec<NetState> {
        self.0.lock().map(|v| v.clone()).unwrap_or_default()
    }

    /// The beacon: (name, height, root) of the first reachable network.
    pub fn beacon(&self) -> Option<(String, u64, String)> {
        self.0.lock().ok()?.iter().find(|n| n.height > 0).map(|n| {
            (n.name.clone(), n.height, n.root.clone())
        })
    }

    /// Start the sync engine: load config, then step every network on its
    /// cadence, forever. Config edits land via [`reload`].
    pub fn start() -> Self {
        let hub = NetHub::default();
        hub.reload();
        let states = hub.0.clone();
        std::thread::Builder::new()
            .name("body-networks".into())
            .spawn(move || {
                let mut due: std::collections::HashMap<String, Instant> = Default::default();
                loop {
                    let targets: Vec<(String, String)> = states
                        .lock()
                        .map(|v| v.iter().map(|n| (n.name.clone(), n.url.clone())).collect())
                        .unwrap_or_default();
                    for (name, url) in targets {
                        let now = Instant::now();
                        if due.get(&name).map(|t| now < *t).unwrap_or(false) {
                            continue;
                        }
                        let step = step_status(&url);
                        due.insert(
                            name.clone(),
                            now + if step.is_ok() { STEP_EVERY } else { RETRY_EVERY },
                        );
                        if let Ok(mut v) = states.lock() {
                            if let Some(n) = v.iter_mut().find(|n| n.name == name) {
                                n.tx += (url.len() + 16) as u64; // request line, honest floor
                                match &step {
                                    Ok((height, root, bytes)) => {
                                        n.rx += *bytes as u64;
                                        n.height = *height;
                                        n.root = root.clone();
                                        n.ok = true;
                                        n.last_step = format!("ok h={height}");
                                        n.last_sync = Some(Instant::now());
                                        persist_state(&v);
                                    }
                                    Err(e) => {
                                        n.ok = false;
                                        n.last_step = e.clone();
                                    }
                                }
                            }
                        }
                    }
                    std::thread::sleep(Duration::from_millis(500));
                }
            })
            .expect("spawn body-networks");
        hub
    }

    /// Re-read the config, keeping live counters for networks that stay.
    pub fn reload(&self) {
        let configured = load_config();
        if let Ok(mut v) = self.0.lock() {
            let old = std::mem::take(&mut *v);
            for (name, url) in configured {
                let mut st = old
                    .iter()
                    .find(|n| n.name == name)
                    .cloned()
                    .unwrap_or_default();
                st.name = name;
                if st.url != url {
                    // A different endpoint is a different conversation.
                    st = NetState { name: st.name.clone(), ..Default::default() };
                }
                st.url = url;
                v.push(st);
            }
        }
    }
}

fn config_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::Path::new(&home).join("cyb").join("networks.toml")
}

fn state_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::Path::new(&home).join("cyb").join("netstate")
}

/// `[[network]]` blocks with `name` and `url` keys. Hand-parsed, same as
/// every other cyb config: one less dependency, one obvious format.
fn parse_config(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let (mut name, mut url): (Option<String>, Option<String>) = (None, None);
    let mut flush = |name: &mut Option<String>, url: &mut Option<String>, out: &mut Vec<_>| {
        if let (Some(n), Some(u)) = (name.take(), url.take()) {
            out.push((n, u));
        }
    };
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("[[") {
            flush(&mut name, &mut url, &mut out);
        } else if let Some((k, v)) = line.split_once('=') {
            let v = v.trim().trim_matches('"').to_string();
            match k.trim() {
                "name" => name = Some(v),
                "url" => url = Some(v.trim_end_matches('/').to_string()),
                _ => {}
            }
        }
    }
    flush(&mut name, &mut url, &mut out);
    out
}

fn load_config() -> Vec<(String, String)> {
    let path = config_path();
    match std::fs::read_to_string(&path) {
        Ok(text) => parse_config(&text),
        Err(_) => {
            // First boot: pussy is the first network — the one the stack
            // tests itself against.
            let default = "# Networks this body talks to. Edited by hand or by the\n\
                           # commander: net add <name> <url> / net set / net rm\n\
                           \n\
                           [[network]]\n\
                           name = \"pussy\"\n\
                           url = \"https://cyb.ai/spacepussy-test\"\n";
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(&path, default);
            parse_config(default)
        }
    }
}

fn save_config(nets: &[(String, String)]) -> Result<(), String> {
    let mut text = String::from(
        "# Networks this body talks to. Edited by hand or by the\n\
         # commander: net add <name> <url> / net set / net rm\n",
    );
    for (name, url) in nets {
        text.push_str(&format!("\n[[network]]\nname = \"{name}\"\nurl = \"{url}\"\n"));
    }
    std::fs::write(config_path(), text).map_err(|e| e.to_string())
}

/// The last good state per network, on disk — a restarted body remembers
/// where each chain stood, and the fleet harness can assert on it.
fn persist_state(states: &[NetState]) {
    let mut text = String::new();
    for n in states {
        if n.height > 0 {
            text.push_str(&format!("{} {} {}\n", n.name, n.height, n.root));
        }
    }
    let _ = std::fs::write(state_path(), text);
}

/// One sync step: GET `<url>/status`, parse the soft3 plain-text form.
/// Returns (height, root, response_bytes).
fn step_status(url: &str) -> Result<(u64, String, usize), String> {
    let full = format!("{url}/status");
    let mut res = ureq::get(&full)
        .call()
        .map_err(|e| short_err(&e.to_string()))?;
    let body = res
        .body_mut()
        .read_to_string()
        .map_err(|e| short_err(&e.to_string()))?;
    let field = |key: &str| -> Option<String> {
        body.lines()
            .find(|l| l.trim_start().starts_with(key))
            .and_then(|l| l.split_once(':'))
            .map(|(_, v)| v.trim().to_string())
    };
    let height: u64 = field("height")
        .and_then(|h| h.parse().ok())
        .ok_or("no height in status")?;
    let root = field("bbg-root").unwrap_or_default();
    Ok((height, root, body.len()))
}

fn short_err(e: &str) -> String {
    let mut s: String = e.chars().take(48).collect();
    if s.len() < e.len() {
        s.push_str("...");
    }
    s
}

// ── the commander verbs ─────────────────────────────────────────────────

/// `net ...` from com. Returns what to say; the hub reloads after edits.
pub fn handle_command(rest: &str, hub: &NetHub) -> String {
    let rest = rest.trim();
    let mut nets = load_config();
    if rest.is_empty() || rest == "list" {
        if nets.is_empty() {
            return "net: none configured - net add <name> <url>".into();
        }
        let states = hub.snapshot();
        return nets
            .iter()
            .map(|(n, u)| {
                let st = states.iter().find(|s| &s.name == n);
                match st {
                    Some(s) if s.height > 0 => {
                        format!("{n} {u} - h={} {}", s.height, short_root(&s.root))
                    }
                    _ => format!("{n} {u} - not reached yet"),
                }
            })
            .collect::<Vec<_>>()
            .join("  |  ");
    }
    if let Some(spec) = rest.strip_prefix("add ") {
        let Some((name, url)) = split_name_url(spec) else {
            return "net add <name> <url>".into();
        };
        if nets.iter().any(|(n, _)| *n == name) {
            return format!("net: {name} exists - net set {name} <url> to change it");
        }
        nets.push((name.clone(), url));
        return apply(&nets, hub, format!("net: {name} added"));
    }
    if let Some(spec) = rest.strip_prefix("set ") {
        let Some((name, url)) = split_name_url(spec) else {
            return "net set <name> <url>".into();
        };
        match nets.iter_mut().find(|(n, _)| *n == name) {
            Some(entry) => {
                entry.1 = url;
                apply(&nets, hub, format!("net: {name} repointed"))
            }
            None => format!("net: no network named {name}"),
        }
    } else if let Some(name) = rest.strip_prefix("rm ") {
        let name = name.trim();
        let before = nets.len();
        nets.retain(|(n, _)| n != name);
        if nets.len() == before {
            return format!("net: no network named {name}");
        }
        apply(&nets, hub, format!("net: {name} gone"))
    } else {
        "net [list]  |  net add <name> <url>  |  net set <name> <url>  |  net rm <name>".into()
    }
}

fn split_name_url(spec: &str) -> Option<(String, String)> {
    let mut it = spec.split_whitespace();
    let name = it.next()?.to_string();
    let url = it.next()?.trim_end_matches('/').to_string();
    (!url.is_empty()).then_some((name, url))
}

fn apply(nets: &[(String, String)], hub: &NetHub, ok: String) -> String {
    match save_config(nets) {
        Ok(()) => {
            hub.reload();
            ok
        }
        Err(e) => format!("net: {e}"),
    }
}

pub fn short_root(root: &str) -> String {
    if root.len() <= 8 {
        root.to_string()
    } else {
        format!("{}..", &root[..8])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_round_trips() {
        let text = "\n[[network]]\nname = \"pussy\"\nurl = \"https://cyb.ai/spacepussy-test\"\n\
                    \n[[network]]\nname = \"local\"\nurl = \"http://127.0.0.1:9911/\"\n";
        let nets = parse_config(text);
        assert_eq!(
            nets,
            vec![
                ("pussy".into(), "https://cyb.ai/spacepussy-test".into()),
                ("local".into(), "http://127.0.0.1:9911".into()),
            ]
        );
    }

    #[test]
    fn status_fields_parse_from_soft3_form() {
        let body = "particle: status\nchain: spacepussy-test\nheight: 3\n\
                    bbg-root: 3acf50598b5f855224a4d253e5a1395c01d2ee17\n";
        let field = |key: &str| -> Option<String> {
            body.lines()
                .find(|l| l.trim_start().starts_with(key))
                .and_then(|l| l.split_once(':'))
                .map(|(_, v)| v.trim().to_string())
        };
        assert_eq!(field("height").unwrap(), "3");
        assert!(field("bbg-root").unwrap().starts_with("3acf5059"));
    }

    /// Live probe of the first network — run by hand:
    /// `cargo test -p cyb net_step_live -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn net_step_live() {
        let (h, root, bytes) = step_status("https://cyb.ai/spacepussy-test").expect("reachable");
        eprintln!("pussy: h={h} root={root} ({bytes} bytes)");
        assert!(h >= 1);
        assert!(!root.is_empty());
    }
}
