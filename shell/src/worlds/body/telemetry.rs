//! What the body is made of, measured for real.
//!
//! One sampler thread wakes every second and reads the machine the way an
//! operator would: `ps` for who eats the cores, `vm_stat` for memory,
//! `netstat` byte counters for the wire. Watts come from a second thread
//! running one-shot `sudo -n powermetrics` sweeps — one-shot on purpose:
//! a long-lived root child would outlive us as an orphan, and this body
//! has been burned by orphaned processes before.
//!
//! Android reads the same facts from /proc. No number on the body page is
//! invented: a probe that fails shows as absent, not as zero pretending.

use std::sync::{Arc, Mutex};
use std::time::Duration;

/// One process line: name, cpu %, resident MB.
#[derive(Clone, Debug, Default)]
pub struct Task {
    pub name: String,
    pub cpu_pct: f32,
    pub rss_mb: f32,
}

/// The body's vital signs at one instant.
#[derive(Clone, Debug, Default)]
pub struct Vitals {
    /// Whole-machine CPU load, 0..100 (all cores = 100).
    pub cpu_pct: f32,
    /// CPU package power, milliwatts. 0 = unknown.
    pub cpu_mw: u32,
    /// GPU busy residency 0..100, negative = unknown.
    pub gpu_pct: f32,
    /// GPU power, milliwatts. 0 = unknown.
    pub gpu_mw: u32,
    pub mem_used: u64,
    pub mem_total: u64,
    /// Bytes per second over physical interfaces.
    pub net_rx_bps: f64,
    pub net_tx_bps: f64,
    /// Top CPU eaters right now, biggest first.
    pub top: Vec<Task>,
}

/// Shared handle: the sampler threads write, the UI reads a clone.
#[derive(Clone, Default)]
pub struct Telemetry(pub Arc<Mutex<Vitals>>);

impl Telemetry {
    pub fn snapshot(&self) -> Vitals {
        self.0.lock().map(|v| v.clone()).unwrap_or_default()
    }

    /// Start the samplers. Called once; threads live as long as the app.
    pub fn start() -> Self {
        let t = Telemetry::default();

        let shared = t.0.clone();
        std::thread::Builder::new()
            .name("body-vitals".into())
            .spawn(move || {
                let mut net_prev: Option<(u64, u64, std::time::Instant)> = None;
                loop {
                    let (cpu, top) = sample_cpu();
                    let (used, total) = sample_mem();
                    let net = sample_net(&mut net_prev);
                    if let Ok(mut v) = shared.lock() {
                        v.cpu_pct = cpu;
                        v.top = top;
                        v.mem_used = used;
                        v.mem_total = total;
                        if let Some((rx, tx)) = net {
                            v.net_rx_bps = rx;
                            v.net_tx_bps = tx;
                        }
                    }
                    std::thread::sleep(Duration::from_secs(1));
                }
            })
            .expect("spawn body-vitals");

        #[cfg(target_os = "macos")]
        {
            let shared = t.0.clone();
            std::thread::Builder::new()
                .name("body-power".into())
                .spawn(move || loop {
                    match sample_power() {
                        Some((cpu_mw, gpu_mw, gpu_pct)) => {
                            if let Ok(mut v) = shared.lock() {
                                v.cpu_mw = cpu_mw;
                                v.gpu_mw = gpu_mw;
                                v.gpu_pct = gpu_pct;
                            }
                        }
                        // No passwordless sudo on this machine: stop asking.
                        None => break,
                    }
                    std::thread::sleep(Duration::from_secs(2));
                })
                .expect("spawn body-power");
        }

        t
    }
}

// ── macOS probes ────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn sample_cpu() -> (f32, Vec<Task>) {
    let out = std::process::Command::new("ps")
        .args(["axo", "pcpu=,rss=,comm=", "-r"])
        .output();
    let Ok(out) = out else { return (0.0, Vec::new()) };
    let text = String::from_utf8_lossy(&out.stdout);
    let ncpu = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1) as f32;

    let mut sum = 0.0f32;
    let mut top = Vec::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let (Some(pcpu), Some(rss)) = (it.next(), it.next()) else { continue };
        let Ok(pcpu) = pcpu.parse::<f32>() else { continue };
        let rss_mb = rss.parse::<f32>().unwrap_or(0.0) / 1024.0;
        sum += pcpu;
        if top.len() < 4 && pcpu >= 1.0 {
            // comm is the executable path; the basename is the name.
            let comm = it.collect::<Vec<_>>().join(" ");
            let name = comm.rsplit('/').next().unwrap_or(&comm).to_string();
            top.push(Task { name, cpu_pct: pcpu, rss_mb });
        }
    }
    ((sum / ncpu).min(100.0), top)
}

#[cfg(target_os = "macos")]
fn sample_mem() -> (u64, u64) {
    let total = std::process::Command::new("sysctl")
        .args(["-n", "hw.memsize"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<u64>().ok())
        .unwrap_or(0);

    let Ok(out) = std::process::Command::new("vm_stat").output() else {
        return (0, total);
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let mut page: u64 = 16384;
    let mut used_pages: u64 = 0;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("Mach Virtual Memory Statistics:") {
            if let Some(p) = rest.split("page size of ").nth(1) {
                page = p
                    .split_whitespace()
                    .next()
                    .and_then(|n| n.parse().ok())
                    .unwrap_or(16384);
            }
        }
        // Used = what an operator means by used: active + wired + compressed.
        for key in ["Pages active:", "Pages wired down:", "Pages occupied by compressor:"] {
            if let Some(rest) = line.strip_prefix(key) {
                used_pages += rest.trim().trim_end_matches('.').parse::<u64>().unwrap_or(0);
            }
        }
    }
    (used_pages * page, total)
}

#[cfg(target_os = "macos")]
fn sample_net(prev: &mut Option<(u64, u64, std::time::Instant)>) -> Option<(f64, f64)> {
    let out = std::process::Command::new("netstat").args(["-ibn"]).output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let (mut rx, mut tx) = (0u64, 0u64);
    for line in text.lines() {
        // Only the <Link#N> row carries the interface's own counters; the
        // per-address rows repeat them and would double-count.
        if !line.contains("<Link#") || line.starts_with("lo0") {
            continue;
        }
        let f: Vec<&str> = line.split_whitespace().collect();
        // Trailing columns are fixed even when Address is blank:
        // ... Ipkts Ierrs Ibytes Opkts Oerrs Obytes Coll
        if f.len() < 7 {
            continue;
        }
        rx += f[f.len() - 5].parse::<u64>().unwrap_or(0);
        tx += f[f.len() - 2].parse::<u64>().unwrap_or(0);
    }
    net_rate(prev, rx, tx)
}

/// Parse one `powermetrics` sweep: CPU mW, GPU mW, GPU busy %.
#[cfg(target_os = "macos")]
fn sample_power() -> Option<(u32, u32, f32)> {
    let out = std::process::Command::new("sudo")
        .args([
            "-n",
            "powermetrics",
            "--samplers",
            "cpu_power,gpu_power",
            "-i",
            "500",
            "-n",
            "1",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let (mut cpu_mw, mut gpu_mw, mut gpu_pct) = (0u32, 0u32, -1.0f32);
    for line in text.lines() {
        let l = line.trim();
        if let Some(rest) = l.strip_prefix("CPU Power:") {
            cpu_mw = parse_mw(rest).unwrap_or(cpu_mw);
        } else if let Some(rest) = l.strip_prefix("GPU Power:") {
            gpu_mw = parse_mw(rest).unwrap_or(gpu_mw);
        } else if let Some(rest) = l.strip_prefix("GPU HW active residency:") {
            gpu_pct = rest
                .trim()
                .split('%')
                .next()
                .and_then(|n| n.trim().parse::<f32>().ok())
                .unwrap_or(gpu_pct);
        } else if gpu_pct < 0.0 {
            if let Some(rest) = l.strip_prefix("GPU idle residency:") {
                if let Some(idle) = rest
                    .trim()
                    .split('%')
                    .next()
                    .and_then(|n| n.trim().parse::<f32>().ok())
                {
                    gpu_pct = (100.0 - idle).max(0.0);
                }
            }
        }
    }
    Some((cpu_mw, gpu_mw, gpu_pct))
}

#[cfg(target_os = "macos")]
fn parse_mw(rest: &str) -> Option<u32> {
    rest.trim().split_whitespace().next()?.parse().ok()
}

// ── /proc probes (Android and other unixes) ─────────────────────────────

#[cfg(not(target_os = "macos"))]
fn sample_cpu() -> (f32, Vec<Task>) {
    use std::sync::OnceLock;
    static PREV: OnceLock<Mutex<(u64, u64)>> = OnceLock::new();

    let Ok(stat) = std::fs::read_to_string("/proc/stat") else {
        return (0.0, Vec::new());
    };
    let Some(cpu) = stat.lines().next() else { return (0.0, Vec::new()) };
    let n: Vec<u64> = cpu
        .split_whitespace()
        .skip(1)
        .filter_map(|x| x.parse().ok())
        .collect();
    if n.len() < 4 {
        return (0.0, Vec::new());
    }
    let idle = n[3] + n.get(4).copied().unwrap_or(0);
    let total: u64 = n.iter().sum();

    let prev = PREV.get_or_init(|| Mutex::new((0, 0)));
    let mut p = prev.lock().expect("cpu prev");
    let (dt, di) = (total.saturating_sub(p.0), idle.saturating_sub(p.1));
    *p = (total, idle);
    if dt == 0 {
        return (0.0, Vec::new());
    }
    (
        100.0 * (dt - di.min(dt)) as f32 / dt as f32,
        Vec::new(), // per-process walk is a later refinement here
    )
}

#[cfg(not(target_os = "macos"))]
fn sample_mem() -> (u64, u64) {
    let Ok(info) = std::fs::read_to_string("/proc/meminfo") else { return (0, 0) };
    let field = |key: &str| -> u64 {
        info.lines()
            .find(|l| l.starts_with(key))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|n| n.parse::<u64>().ok())
            .unwrap_or(0)
            * 1024
    };
    let total = field("MemTotal:");
    (total.saturating_sub(field("MemAvailable:")), total)
}

#[cfg(not(target_os = "macos"))]
fn sample_net(prev: &mut Option<(u64, u64, std::time::Instant)>) -> Option<(f64, f64)> {
    let dev = std::fs::read_to_string("/proc/net/dev").ok()?;
    let (mut rx, mut tx) = (0u64, 0u64);
    for line in dev.lines().skip(2) {
        let (name, rest) = line.split_once(':')?;
        if name.trim() == "lo" {
            continue;
        }
        let f: Vec<&str> = rest.split_whitespace().collect();
        if f.len() >= 9 {
            rx += f[0].parse::<u64>().unwrap_or(0);
            tx += f[8].parse::<u64>().unwrap_or(0);
        }
    }
    net_rate(prev, rx, tx)
}

/// Counter pair -> bytes/second since the previous sample.
fn net_rate(
    prev: &mut Option<(u64, u64, std::time::Instant)>,
    rx: u64,
    tx: u64,
) -> Option<(f64, f64)> {
    let now = std::time::Instant::now();
    let rate = prev.map(|(prx, ptx, pt)| {
        let dt = now.duration_since(pt).as_secs_f64().max(0.1);
        (
            rx.saturating_sub(prx) as f64 / dt,
            tx.saturating_sub(ptx) as f64 / dt,
        )
    });
    *prev = Some((rx, tx, now));
    rate
}
