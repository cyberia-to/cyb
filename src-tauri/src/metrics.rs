use std::sync::atomic::Ordering;
use std::time::Instant;

use crate::mining::MiningState;

const DEFAULT_PUSHGATEWAY: &str = "https://cybernode.ai/pushgateway/";
/// Push aggregated metrics every 5 minutes (driven by mining batches, not wall clock)
const PUSH_INTERVAL_SECS: f64 = 300.0;

pub struct MetricsReporter {
    endpoint: String,
    instance: String,
    auth: Option<(String, String)>,
    last_push: Instant,
    session_start: Instant,
}

impl MetricsReporter {
    pub fn new(miner_address: &str) -> Self {
        let instance = if miner_address.len() >= 12 {
            &miner_address[..12]
        } else {
            miner_address
        };

        let endpoint = std::env::var("CYB_PUSHGATEWAY_URL")
            .unwrap_or_else(|_| DEFAULT_PUSHGATEWAY.to_string());

        let auth = match (
            std::env::var("CYB_PUSHGATEWAY_USER"),
            std::env::var("CYB_PUSHGATEWAY_PASS"),
        ) {
            (Ok(user), Ok(pass)) if !user.is_empty() => Some((user, pass)),
            _ => None,
        };

        let now = Instant::now();
        Self {
            endpoint,
            instance: instance.to_string(),
            auth,
            last_push: now,
            session_start: now,
        }
    }

    /// Called from the mining loop after each batch. Pushes if enough time has elapsed.
    /// Returns true if a push was triggered.
    pub fn maybe_push(&mut self, state: &MiningState, backend: &str) -> bool {
        let elapsed_since_push = self.last_push.elapsed().as_secs_f64();
        if elapsed_since_push < PUSH_INTERVAL_SECS {
            return false;
        }
        self.do_push(state, backend);
        true
    }

    /// Force-push current metrics (call on mining stop to flush final state)
    pub fn flush(&mut self, state: &MiningState, backend: &str) {
        self.do_push(state, backend);
    }

    fn do_push(&mut self, state: &MiningState, backend: &str) {
        let body = self.build_metrics(state, backend);
        self.push(body);
        self.last_push = Instant::now();
    }

    /// Build Prometheus text format metrics body
    fn build_metrics(&self, state: &MiningState, backend: &str) -> String {
        let session_secs = self.session_start.elapsed().as_secs_f64();
        let total_hashes = state.hash_count.load(Ordering::Relaxed);
        let hashrate = if session_secs > 0.0 {
            total_hashes as f64 / session_secs
        } else {
            0.0
        };
        let batches = state.batch_count.load(Ordering::Relaxed);
        let batch_time_us = state.total_batch_time_us.load(Ordering::Relaxed);
        let avg_batch_ms = if batches > 0 {
            batch_time_us as f64 / batches as f64 / 1000.0
        } else {
            0.0
        };
        let proofs_submitted = state.proofs_submitted.load(Ordering::Relaxed);
        let proofs_failed = state.proofs_failed.load(Ordering::Relaxed);

        format!(
            "# HELP cyb_mining_hashrate Current hashrate in H/s\n\
             # TYPE cyb_mining_hashrate gauge\n\
             cyb_mining_hashrate{{backend=\"{backend}\"}} {hashrate:.1}\n\
             # HELP cyb_mining_total_hashes Total hashes computed\n\
             # TYPE cyb_mining_total_hashes counter\n\
             cyb_mining_total_hashes {total_hashes}\n\
             # HELP cyb_mining_batch_avg_ms Average batch duration in milliseconds\n\
             # TYPE cyb_mining_batch_avg_ms gauge\n\
             cyb_mining_batch_avg_ms {avg_batch_ms:.2}\n\
             # HELP cyb_mining_batch_count Total batches processed\n\
             # TYPE cyb_mining_batch_count counter\n\
             cyb_mining_batch_count {batches}\n\
             # HELP cyb_mining_proofs_submitted Successfully submitted proofs\n\
             # TYPE cyb_mining_proofs_submitted counter\n\
             cyb_mining_proofs_submitted {proofs_submitted}\n\
             # HELP cyb_mining_proofs_failed Failed proof submissions\n\
             # TYPE cyb_mining_proofs_failed counter\n\
             cyb_mining_proofs_failed {proofs_failed}\n\
             # HELP cyb_mining_session_duration_seconds Mining session duration\n\
             # TYPE cyb_mining_session_duration_seconds gauge\n\
             cyb_mining_session_duration_seconds {session_secs:.1}\n",
        )
    }

    /// Push metrics to Pushgateway (fire-and-forget in background thread)
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    fn push(&self, body: String) {
        let url = format!(
            "{}metrics/job/cyb_mining/instance/{}",
            self.endpoint, self.instance
        );
        let auth = self.auth.clone();

        std::thread::spawn(move || {
            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build();
            let client = match client {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[Metrics] Failed to build HTTP client: {}", e);
                    return;
                }
            };
            let mut req = client
                .post(&url)
                .header("Content-Type", "text/plain; version=0.0.4")
                .body(body);
            if let Some((user, pass)) = auth {
                req = req.basic_auth(user, Some(pass));
            }
            match req.send() {
                Ok(resp) => {
                    if !resp.status().is_success() {
                        eprintln!("[Metrics] Push failed: HTTP {}", resp.status());
                    }
                }
                Err(e) => {
                    eprintln!("[Metrics] Push error: {}", e);
                }
            }
        });
    }

    #[cfg(any(target_os = "android", target_os = "ios"))]
    fn push(&self, _body: String) {
        // No metrics on mobile
    }
}
