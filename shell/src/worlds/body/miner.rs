//! The body earns: erga, managed as a real child process.
//!
//! cyb does not reimplement mining — the machinery is erga itself, spawned
//! as `erga mine --machine` and read over its line protocol (`DEVICE`,
//! `STAT rate_khs height accepted rejected hashed donated build% next%
//! status...`, every 500ms). It is never linked in-process: erga's own
//! contract forbids a second graphics context next to its Metal pipeline,
//! and cyb holds a window.
//!
//! Two controls are real levers, not UI theater:
//! - start/stop spawns and kills the child (the child dies with us too);
//! - intensity writes erga's 3-byte intensity file, which any running
//!   miner — ours or one the owner started elsewhere — re-reads every
//!   500ms. Duty cycle changes mid-flight, no restart.
//!
//! Earnings use erga's own formula against live network difficulty from
//! the pool API, then convert to PUSSY through `~/cyb/rates.toml` — a
//! declared rate, marked as such, because spacepussy has no market yet.

use std::io::BufRead as _;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// erga's chain constants (rs/app/src/pool.rs).
pub const BLOCK_REWARD_ERG: f64 = 3.0;
pub const BLOCK_TIME_S: f64 = 120.0;
const NETWORK_STATS: &str = "https://ergo.herominers.com/api/stats";

#[derive(Clone, Debug, Default)]
pub struct MinerStat {
    /// Our child is alive and speaking.
    pub running: bool,
    /// An erga we did not start is on this machine.
    pub external: bool,
    pub device: String,
    pub status: String,
    pub rate_khs: u64,
    pub height: u64,
    pub accepted: u64,
    pub rejected: u64,
    /// Network difficulty (0 = not fetched yet).
    pub difficulty: f64,
    /// ERG spot price in USD (0 = unknown).
    pub price_usd: f64,
    pub since: Option<Instant>,
}

impl MinerStat {
    pub fn rate_mhs(&self) -> f64 {
        self.rate_khs as f64 / 1000.0
    }

    /// erga's own per-day estimate (cli/src/face.rs), from the live rate.
    pub fn erg_per_day(&self) -> Option<f64> {
        if self.difficulty <= 0.0 || self.rate_khs == 0 {
            return None;
        }
        let net = self.difficulty / BLOCK_TIME_S;
        Some(self.rate_mhs() * 1e6 / net * (86_400.0 / BLOCK_TIME_S) * BLOCK_REWARD_ERG)
    }
}

/// The one miner the body manages. Child handle and stats live behind
/// mutexes so the reader thread, the poller and the UI never block each
/// other for long.
#[derive(Clone, Default)]
pub struct Miner {
    pub stat: Arc<Mutex<MinerStat>>,
    child: Arc<Mutex<Option<Child>>>,
}

/// The child must not outlive the body: a miner burning watts for an app
/// that is gone is exactly the orphan this machine has suffered before.
impl Drop for Miner {
    fn drop(&mut self) {
        if Arc::strong_count(&self.child) == 1 {
            self.stop();
        }
    }
}

fn erga_binary() -> Option<std::path::PathBuf> {
    let candidates = [
        std::path::PathBuf::from("/Applications/erga.app/Contents/MacOS/erga"),
        dirs_home().join("cyber/erga/target/release/erga"),
    ];
    candidates.into_iter().find(|p| p.exists())
}

fn dirs_home() -> std::path::PathBuf {
    std::path::PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()))
}

fn intensity_file() -> std::path::PathBuf {
    dirs_home().join("Library/Application Support/ai.cyber.erga/intensity")
}

impl Miner {
    pub fn start() -> Self {
        let m = Miner::default();

        // Background poller: network difficulty + price every 10 min,
        // external-miner detection every 5s. Never touches the UI thread.
        let stat = m.stat.clone();
        let child = m.child.clone();
        std::thread::Builder::new()
            .name("body-miner-poll".into())
            .spawn(move || {
                let mut last_net: Option<Instant> = None;
                let mut net_every = Duration::from_secs(30);
                loop {
                    if last_net.map(|t| t.elapsed() > net_every).unwrap_or(true) {
                        match fetch_network() {
                            Some((diff, price)) => {
                                if let Ok(mut s) = stat.lock() {
                                    s.difficulty = diff;
                                    s.price_usd = price;
                                }
                                net_every = Duration::from_secs(600);
                            }
                            // A failed probe retries soon; difficulty is
                            // what turns the rate into ERG/day.
                            None => net_every = Duration::from_secs(30),
                        }
                        last_net = Some(Instant::now());
                    }
                    let own = child
                        .lock()
                        .ok()
                        .and_then(|c| c.as_ref().map(|c| c.id()))
                        .unwrap_or(0);
                    let ext = external_erga(own);
                    if let Ok(mut s) = stat.lock() {
                        s.external = ext;
                    }
                    std::thread::sleep(Duration::from_secs(5));
                }
            })
            .expect("spawn body-miner-poll");

        m
    }

    pub fn is_ours(&self) -> bool {
        self.child.lock().map(|c| c.is_some()).unwrap_or(false)
    }

    /// Spawn `erga mine --machine` and start reading its stats.
    pub fn mine(&self) -> Result<(), String> {
        if self.is_ours() {
            return Ok(());
        }
        let bin = erga_binary().ok_or("erga not found (install erga.app)")?;
        let mut cmd = Command::new(&bin);
        cmd.args(["mine", "--machine"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .stdin(Stdio::null());
        let mut ch = cmd.spawn().map_err(|e| format!("erga: {e}"))?;
        let stdout = ch.stdout.take().ok_or("erga: no stdout")?;

        {
            let mut slot = self.child.lock().expect("miner child");
            *slot = Some(ch);
        }
        if let Ok(mut s) = self.stat.lock() {
            s.running = true;
            s.status = "starting...".into();
            s.since = Some(Instant::now());
            s.accepted = 0;
            s.rejected = 0;
            s.rate_khs = 0;
        }

        // Reader thread: erga speaks every 500ms; EOF means the child died.
        let stat = self.stat.clone();
        let child = self.child.clone();
        std::thread::Builder::new()
            .name("body-miner-read".into())
            .spawn(move || {
                let reader = std::io::BufReader::new(stdout);
                for line in reader.lines() {
                    let Ok(line) = line else { break };
                    if let Ok(mut s) = stat.lock() {
                        parse_line(&line, &mut s);
                    }
                }
                if let Ok(mut s) = stat.lock() {
                    s.running = false;
                    s.rate_khs = 0;
                    s.status = "stopped".into();
                }
                if let Ok(mut slot) = child.lock() {
                    if let Some(mut c) = slot.take() {
                        let _ = c.wait();
                    }
                }
            })
            .expect("spawn body-miner-read");
        Ok(())
    }

    pub fn stop(&self) {
        if let Ok(mut slot) = self.child.lock() {
            if let Some(c) = slot.as_mut() {
                let _ = c.kill();
            }
            // The reader thread reaps the exit status on EOF.
        }
    }

    /// Current duty-cycle mode from erga's own config file.
    pub fn intensity(&self) -> String {
        std::fs::read_to_string(intensity_file())
            .map(|s| s.trim().to_string())
            .ok()
            .filter(|s| ["max", "eco", "min"].contains(&s.as_str()))
            .unwrap_or_else(|| "max".into())
    }

    /// Write the mode; every running erga re-reads this file within 500ms.
    pub fn set_intensity(&self, mode: &str) {
        let path = intensity_file();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, mode);
    }
}

/// erga's `--machine` line protocol (rs/app/src/miner.rs parse_line).
fn parse_line(line: &str, s: &mut MinerStat) {
    if let Some(rest) = line.strip_prefix("DEVICE ") {
        s.device = rest.trim().to_string();
        return;
    }
    let Some(rest) = line.strip_prefix("STAT ") else { return };
    let mut it = rest.split_whitespace();
    let mut next_u64 = |dst: &mut u64| {
        if let Some(v) = it.next().and_then(|x| x.parse().ok()) {
            *dst = v;
        }
    };
    next_u64(&mut s.rate_khs);
    next_u64(&mut s.height);
    next_u64(&mut s.accepted);
    next_u64(&mut s.rejected);
    let (mut hashed, mut donated, mut build, mut next_pct) = (0, 0, 0, 0);
    next_u64(&mut hashed);
    next_u64(&mut donated);
    next_u64(&mut build);
    next_u64(&mut next_pct);
    let status: Vec<&str> = it.collect();
    if !status.is_empty() {
        // erga's status uses a middle dot the UI font lacks; ASCII, not tofu.
        s.status = status
            .join(" ")
            .chars()
            .map(|c| if c.is_ascii() { c } else { '-' })
            .collect();
    }
    if build < 100 {
        s.status = format!("building epoch table {build}%");
    }
    s.running = true;
}

/// Is an erga we did not spawn running? `own` is our child's pid (0 = none).
fn external_erga(own: u32) -> bool {
    let Ok(out) = Command::new("pgrep").args(["-f", "MacOS/erga|erga mine"]).output() else {
        return false;
    };
    let me = std::process::id();
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.trim().parse::<u32>().ok())
        .any(|pid| pid != own && pid != me)
}

/// (network difficulty, ERG price USD) from the pool's stats endpoint —
/// erga's own source (rs/app/src/pool.rs::network).
fn fetch_network() -> Option<(f64, f64)> {
    let mut res = ureq::get(NETWORK_STATS)
        .call()
        .map_err(|e| bevy::log::warn!("body: pool stats: {e}"))
        .ok()?;
    let json: serde_json::Value = res
        .body_mut()
        .read_json()
        .map_err(|e| bevy::log::warn!("body: pool stats decode: {e}"))
        .ok()?;
    // The API sends big numbers as strings and small ones as numbers
    // (erga's num() learned this the hard way) — accept both.
    let num = |v: Option<&serde_json::Value>| -> f64 {
        v.and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
            .unwrap_or(0.0)
    };
    let diff = num(json.get("network").and_then(|n| n.get("difficulty")));
    let price = num(
        json.get("pool")
            .and_then(|p| p.get("price"))
            .and_then(|p| p.get("usd")),
    );
    (diff > 0.0).then_some((diff, price))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Live probe of the pool API — network-dependent, run by hand:
    /// `cargo test -p cyb fetch_network -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn fetch_network_returns_difficulty() {
        let (diff, price) = fetch_network().expect("pool stats reachable");
        assert!(diff > 1e12, "difficulty: {diff}");
        assert!(price > 0.0, "price: {price}");
    }

    #[test]
    fn stat_line_parses() {
        let mut s = MinerStat::default();
        parse_line("DEVICE Apple M4 Max", &mut s);
        parse_line(
            "STAT 8420 1862800 37 0 412876032 2 100 200 mining - next table ready",
            &mut s,
        );
        assert_eq!(s.device, "Apple M4 Max");
        assert_eq!(s.rate_khs, 8420);
        assert_eq!(s.height, 1_862_800);
        assert_eq!(s.accepted, 37);
        assert_eq!(s.rejected, 0);
        assert!(s.running);
        assert_eq!(s.status, "mining - next table ready");
    }

    #[test]
    fn status_is_forced_to_ascii() {
        let mut s = MinerStat::default();
        parse_line("STAT 1 2 3 0 0 0 100 200 mining \u{b7} development share", &mut s);
        assert_eq!(s.status, "mining - development share");
    }

    #[test]
    fn erg_per_day_matches_ergas_formula() {
        let s = MinerStat {
            rate_khs: 8420, // 8.42 MH/s
            difficulty: 2.0e15,
            ..Default::default()
        };
        let net = 2.0e15 / BLOCK_TIME_S;
        let expect = 8.42e6 / net * (86_400.0 / BLOCK_TIME_S) * BLOCK_REWARD_ERG;
        assert!((s.erg_per_day().unwrap() - expect).abs() < 1e-12);
    }

    #[test]
    fn building_epoch_reports_progress() {
        let mut s = MinerStat::default();
        parse_line("STAT 0 1862800 0 0 0 0 40 200 building", &mut s);
        assert_eq!(s.status, "building epoch table 40%");
    }
}
