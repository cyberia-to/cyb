//! Seer — the structure miner. Second earner on the body.
//!
//! Finds an unlinked pair of existing files and casts the cyberlink.
//! Never creates a file. Idles when no pair in reach is worth it.
//! Tru's φ* is not computed here (invariant 1); this v1 ranks by
//! inverse-degree (prefer sparse ends) — a prior, not the mint.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::super::SharedCell;

#[derive(Clone, Debug, Default)]
pub struct SeerStat {
    pub running: bool,
    pub proposals: u64,
    pub casts: u64,
    pub idle: bool,
    pub last_score: f32,
    pub last_pair: Option<(String, String)>,
    pub since: Option<Instant>,
}

impl SeerStat {
    pub fn casts_per_min(&self) -> f64 {
        match self.since {
            Some(t) if self.casts > 0 => {
                self.casts as f64 / (t.elapsed().as_secs_f64() / 60.0).max(1e-9)
            }
            _ => 0.0,
        }
    }
}

#[derive(Clone, Default)]
pub struct Seer {
    pub stat: Arc<Mutex<SeerStat>>,
    run: Arc<AtomicBool>,
}

fn wanted_file() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::Path::new(&home).join("cyb").join("seer")
}

pub fn wanted() -> bool {
    std::fs::read_to_string(wanted_file())
        .map(|s| s.trim() == "on")
        .unwrap_or(false)
}

pub fn set_wanted(on: bool) {
    let path = wanted_file();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, if on { "on" } else { "off" });
}

fn intensity_file() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::Path::new(&home)
        .join("cyb")
        .join("seer-intensity")
}

pub fn intensity() -> String {
    std::fs::read_to_string(intensity_file())
        .map(|s| s.trim().to_string())
        .ok()
        .filter(|s| ["max", "eco", "min"].contains(&s.as_str()))
        .unwrap_or_else(|| "eco".into())
}

pub fn set_intensity(mode: &str) {
    let path = intensity_file();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, mode);
}

fn pause(mode: &str) -> Duration {
    match mode {
        "max" => Duration::from_millis(200),
        "min" => Duration::from_secs(4),
        _ => Duration::from_secs(1),
    }
}

impl Seer {
    pub fn start() -> Self {
        Seer::default()
    }

    pub fn is_running(&self) -> bool {
        self.run.load(Ordering::Relaxed)
    }

    pub fn mine(&self, shared: SharedCell, neuron: [u8; 32]) {
        if self.run.swap(true, Ordering::SeqCst) {
            return;
        }
        set_wanted(true);
        if let Ok(mut st) = self.stat.lock() {
            st.running = true;
            st.idle = false;
            st.since = Some(Instant::now());
        }
        let run = self.run.clone();
        let stat = self.stat.clone();
        if std::thread::Builder::new()
            .name("cyb-seer".into())
            .spawn(move || {
                loop {
                    if !run.load(Ordering::Relaxed) {
                        break;
                    }
                    let mode = intensity();
                    let found = propose_and_cast(&shared, neuron);
                    if let Ok(mut st) = stat.lock() {
                        st.proposals += 1;
                        match found {
                            Some((score, a, b)) => {
                                st.casts += 1;
                                st.idle = false;
                                st.last_score = score;
                                st.last_pair = Some((
                                    file::Particle::from_bytes(a).short_hex(),
                                    file::Particle::from_bytes(b).short_hex(),
                                ));
                                shared.bump();
                            }
                            None => st.idle = true,
                        }
                    }
                    std::thread::sleep(pause(&mode));
                }
            })
            .is_err()
        {
            self.run.store(false, Ordering::SeqCst);
            if let Ok(mut st) = self.stat.lock() {
                st.running = false;
            }
        }
    }

    pub fn stop(&self) {
        self.run.store(false, Ordering::SeqCst);
        set_wanted(false);
        if let Ok(mut st) = self.stat.lock() {
            st.running = false;
        }
    }
}

/// Inverse-degree prior among unlinked node pairs. No φ*. No new files.
fn propose_and_cast(shared: &SharedCell, neuron: [u8; 32]) -> Option<(f32, [u8; 32], [u8; 32])> {
    let mut cell = shared.cell.lock().ok()?;
    let axons = cell.axons();
    let nodes = cell.nodes();
    if nodes.len() < 2 {
        return None;
    }
    let mut linked: HashSet<([u8; 32], [u8; 32])> = HashSet::new();
    let mut deg: HashMap<[u8; 32], u32> = HashMap::new();
    for (a, b, _) in &axons {
        linked.insert(if a <= b { (*a, *b) } else { (*b, *a) });
        *deg.entry(*a).or_default() += 1;
        *deg.entry(*b).or_default() += 1;
    }
    let ps: Vec<[u8; 32]> = nodes.iter().map(|(p, _)| *p).collect();
    let n = ps.len().min(64);
    let mut best: Option<(f32, [u8; 32], [u8; 32])> = None;
    for i in 0..n {
        for j in (i + 1)..n {
            let a = ps[i];
            let b = ps[j];
            let key = if a <= b { (a, b) } else { (b, a) };
            if linked.contains(&key) {
                continue;
            }
            let da = *deg.get(&a).unwrap_or(&0) as f32;
            let db = *deg.get(&b).unwrap_or(&0) as f32;
            let score = 1.0 / ((1.0 + da) * (1.0 + db));
            if best.map(|(s, _, _)| score > s).unwrap_or(true) {
                best = Some((score, a, b));
            }
        }
    }
    let (score, a, b) = best?;
    cell.cast(neuron, [(a, b)]).ok()?;
    Some((score, a, b))
}
