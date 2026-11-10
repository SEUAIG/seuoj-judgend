//! Runtime-adjustable judge concurrency, plus a timing probe that shows how
//! much measured run time drifts as more submissions are judged at once.

use crate::config::AijConfig;
use crate::error::{AijError, Result};
use crate::server::AppJson;
use axum::Json;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::process::Command;
use tokio::sync::{AcquireError, Semaphore, SemaphorePermit};
use tracing::{info, warn};

/// Upper bound accepted by `PUT /judge/concurrency`.
pub(crate) const MAX_LIMIT: usize = 64;

/// Limits how many submissions are judged at the same time.
///
/// The limit can change while submissions are being judged. Raising it takes
/// effect at once. Lowering it removes idle permits immediately; permits that
/// are still held are retired as their submissions finish (`debt`), so a
/// running submission is never interrupted. Waiters are served in FIFO order.
pub(crate) struct JudgeLimiter {
    semaphore: Semaphore,
    state: Mutex<LimiterState>,
    running: AtomicUsize,
    waiting: AtomicUsize,
}

struct LimiterState {
    limit: usize,
    /// Held permits that must be forgotten instead of returned.
    debt: usize,
}

/// A slot for judging one submission; released on drop.
pub(crate) struct JudgePermit<'a> {
    permit: Option<SemaphorePermit<'a>>,
    limiter: &'a JudgeLimiter,
}

impl Drop for JudgePermit<'_> {
    fn drop(&mut self) {
        self.limiter.running.fetch_sub(1, Ordering::SeqCst);
        let mut state = self.limiter.lock_state();
        if let Some(permit) = self.permit.take() {
            if state.debt > 0 {
                state.debt -= 1;
                permit.forget();
            } else {
                drop(permit);
            }
        }
    }
}

struct WaitingGuard<'a>(&'a AtomicUsize);

impl Drop for WaitingGuard<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Current state of the limiter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LimiterSnapshot {
    pub(crate) limit: usize,
    pub(crate) running: usize,
    pub(crate) waiting: usize,
}

impl JudgeLimiter {
    pub(crate) fn new(limit: usize) -> Self {
        let limit = limit.max(1);
        Self {
            semaphore: Semaphore::new(limit),
            state: Mutex::new(LimiterState { limit, debt: 0 }),
            running: AtomicUsize::new(0),
            waiting: AtomicUsize::new(0),
        }
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, LimiterState> {
        // The critical sections never panic; recover the data if one did.
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub(crate) async fn acquire(&self) -> std::result::Result<JudgePermit<'_>, AcquireError> {
        self.waiting.fetch_add(1, Ordering::SeqCst);
        let guard = WaitingGuard(&self.waiting);
        let permit = self.semaphore.acquire().await?;
        drop(guard);
        self.running.fetch_add(1, Ordering::SeqCst);
        Ok(JudgePermit {
            permit: Some(permit),
            limiter: self,
        })
    }

    pub(crate) fn set_limit(&self, limit: usize) {
        let limit = limit.max(1);
        let mut state = self.lock_state();
        if limit > state.limit {
            let mut add = limit - state.limit;
            let cancelled = add.min(state.debt);
            state.debt -= cancelled;
            add -= cancelled;
            if add > 0 {
                self.semaphore.add_permits(add);
            }
        } else if limit < state.limit {
            let remove = state.limit - limit;
            let forgotten = self.semaphore.forget_permits(remove);
            state.debt += remove - forgotten;
        }
        state.limit = limit;
    }

    pub(crate) fn snapshot(&self) -> LimiterSnapshot {
        let limit = self.lock_state().limit;
        LimiterSnapshot {
            limit,
            running: self.running.load(Ordering::SeqCst),
            waiting: self.waiting.load(Ordering::SeqCst),
        }
    }
}

static LIMITER: OnceLock<JudgeLimiter> = OnceLock::new();

/// The process-wide limiter, initialised from `max_concurrent_requests`.
pub(crate) fn judge_limiter() -> &'static JudgeLimiter {
    LIMITER.get_or_init(|| JudgeLimiter::new(default_limit()))
}

fn default_limit() -> usize {
    AijConfig::get().max_concurrent_requests.max(1)
}

// ---------------------------------------------------------------- probe

/// Seconds between two probe runs.
const PROBE_INTERVAL_SECS: u64 = 15;
/// Samples kept in memory (two hours at the default interval).
const PROBE_KEEP: usize = 480;

/// A fixed CPU- and cache-bound workload. Its measured time only changes
/// when the machine is busier, which is what the probe wants to show.
const PROBE_SOURCE: &str = r#"#include <stdio.h>
#define N (1u << 20)
static unsigned a[N];
int main(void) {
    unsigned x = 1, s = 0;
    for (int r = 0; r < 24; r++)
        for (unsigned i = 0; i < N; i++) {
            x = x * 1103515245u + 12345u;
            a[(x >> 8) & (N - 1)] += x;
            s += a[i];
        }
    printf("%u\n", s);
    return 0;
}
"#;

/// One probe run.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProbeSample {
    /// Unix time in milliseconds when the run started.
    at: i64,
    /// Submissions being judged during the run (the larger of start and end).
    running: usize,
    cpu_ms: i32,
    real_ms: i32,
}

#[derive(Default)]
struct ProbeState {
    samples: VecDeque<ProbeSample>,
    error: Option<String>,
}

static PROBE: Mutex<ProbeState> = Mutex::new(ProbeState {
    samples: VecDeque::new(),
    error: None,
});

fn probe_state() -> std::sync::MutexGuard<'static, ProbeState> {
    PROBE.lock().unwrap_or_else(|e| e.into_inner())
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

async fn build_probe(dir: &Path) -> std::result::Result<PathBuf, String> {
    tokio::fs::create_dir_all(dir)
        .await
        .map_err(|e| format!("create {}: {}", dir.display(), e))?;
    let source = dir.join("probe.c");
    let exe = dir.join("probe");
    tokio::fs::write(&source, PROBE_SOURCE)
        .await
        .map_err(|e| format!("write probe source: {}", e))?;
    tokio::fs::write(dir.join("probe.in"), b"")
        .await
        .map_err(|e| format!("write probe input: {}", e))?;
    let gcc = AijConfig::get_binary_path("gcc")
        .await
        .map_err(|e| format!("gcc not found: {}", e))?;
    let output = Command::new(gcc)
        .arg("-O2")
        .arg(&source)
        .arg("-o")
        .arg(&exe)
        .output()
        .await
        .map_err(|e| format!("run gcc: {}", e))?;
    if !output.status.success() {
        return Err(format!(
            "compile probe: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(exe)
}

fn probe_config(dir: &Path, exe: &Path) -> judger::Config {
    let path = |name: &str| dir.join(name).to_string_lossy().to_string();
    judger::Config {
        max_cpu_time: 5000,
        max_real_time: 10000,
        max_memory: 256 * 1024 * 1024,
        max_stack: 64 * 1024 * 1024,
        max_process_number: 1,
        max_output_size: 1024 * 1024,
        exe_path: exe.to_string_lossy().to_string(),
        input_path: path("probe.in"),
        output_path: path("probe.out"),
        error_path: path("probe.err"),
        log_path: path("probe.log"),
        seccomp_rule_name: Some(judger::SeccompRuleName::CCpp),
        ..Default::default()
    }
}

async fn run_probe_once(config: &judger::Config) -> std::result::Result<ProbeSample, String> {
    let limiter = judge_limiter();
    let at = now_millis();
    let before = limiter.snapshot().running;
    let config = config.clone();
    let result = tokio::task::spawn_blocking(move || judger::run(&config, None))
        .await
        .map_err(|e| format!("probe task: {}", e))??;
    let after = limiter.snapshot().running;
    if !matches!(result.result, judger::ErrorCode::Success) {
        return Err(format!("probe run failed: {:?}", result.result));
    }
    Ok(ProbeSample {
        at,
        running: before.max(after),
        cpu_ms: result.cpu_time,
        real_ms: result.real_time,
    })
}

/// Start the background timing probe. Call once from `main`.
pub(crate) fn start_timing_probe() {
    tokio::spawn(async {
        let dir = std::env::temp_dir().join("seuoj-timing-probe");
        let exe = loop {
            match build_probe(&dir).await {
                Ok(exe) => break exe,
                Err(e) => {
                    warn!("Timing probe unavailable: {}", e);
                    probe_state().error = Some(e);
                    tokio::time::sleep(Duration::from_secs(600)).await;
                }
            }
        };
        info!("Timing probe ready: {}", exe.display());
        let config = probe_config(&dir, &exe);
        let mut ticker = tokio::time::interval(Duration::from_secs(PROBE_INTERVAL_SECS));
        loop {
            ticker.tick().await;
            let outcome = run_probe_once(&config).await;
            let mut state = probe_state();
            match outcome {
                Ok(sample) => {
                    state.error = None;
                    if state.samples.len() >= PROBE_KEEP {
                        state.samples.pop_front();
                    }
                    state.samples.push_back(sample);
                }
                Err(e) => {
                    if state.error.as_deref() != Some(e.as_str()) {
                        warn!("Timing probe: {}", e);
                    }
                    state.error = Some(e);
                }
            }
        }
    });
}

// ---------------------------------------------------------------- HTTP

fn status_json(include_samples: bool) -> serde_json::Value {
    let snapshot = judge_limiter().snapshot();
    let probe = probe_state();
    let samples: Vec<&ProbeSample> = if include_samples {
        probe.samples.iter().collect()
    } else {
        Vec::new()
    };
    json!({
        "code": 0,
        "message": "Success",
        "data": {
            "limit": snapshot.limit,
            "defaultLimit": default_limit(),
            "maxLimit": MAX_LIMIT,
            "running": snapshot.running,
            "waiting": snapshot.waiting,
            "probe": {
                "intervalSeconds": PROBE_INTERVAL_SECS,
                "error": probe.error,
                "samples": samples,
            },
        },
    })
}

/// `GET /judge/concurrency`: limit, load and timing probe samples.
pub(crate) async fn get_concurrency() -> Result<Json<serde_json::Value>> {
    Ok(Json(status_json(true)))
}

#[derive(Deserialize)]
pub(crate) struct SetConcurrencyRequest {
    limit: usize,
}

/// `PUT /judge/concurrency`: change the limit without restarting.
pub(crate) async fn put_concurrency(
    AppJson(payload): AppJson<SetConcurrencyRequest>,
) -> Result<Json<serde_json::Value>> {
    if payload.limit == 0 || payload.limit > MAX_LIMIT {
        return Err(AijError::Request(
            StatusCode::BAD_REQUEST,
            "INVALID_LIMIT".to_string(),
            format!("limit must be between 1 and {}", MAX_LIMIT),
        ));
    }
    let before = judge_limiter().snapshot().limit;
    judge_limiter().set_limit(payload.limit);
    if before != payload.limit {
        info!("Judge concurrency changed: {} -> {}", before, payload.limit);
    }
    Ok(Json(status_json(false)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn snap(l: &JudgeLimiter) -> (usize, usize, usize) {
        let s = l.snapshot();
        (s.limit, s.running, s.waiting)
    }

    #[tokio::test]
    async fn raising_the_limit_admits_waiters_at_once() {
        let l = Arc::new(JudgeLimiter::new(1));
        let first = l.acquire().await.unwrap_or_else(|_| unreachable!());
        let l2 = l.clone();
        let waiter = tokio::spawn(async move {
            let _p = l2.acquire().await;
            tokio::time::sleep(Duration::from_millis(50)).await;
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert_eq!(snap(&l), (1, 1, 1));
        l.set_limit(2);
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert_eq!(snap(&l), (2, 2, 0));
        drop(first);
        let _ = waiter.await;
        assert_eq!(snap(&l), (2, 0, 0));
        assert_eq!(l.semaphore.available_permits(), 2);
    }

    #[tokio::test]
    async fn lowering_the_limit_waits_for_running_submissions() {
        let l = JudgeLimiter::new(3);
        let a = l.acquire().await.unwrap_or_else(|_| unreachable!());
        let b = l.acquire().await.unwrap_or_else(|_| unreachable!());
        // one idle permit is removed now, one held permit becomes debt
        l.set_limit(1);
        assert_eq!(l.semaphore.available_permits(), 0);
        assert_eq!(l.lock_state().debt, 1);
        drop(a);
        // the permit was retired, not returned
        assert_eq!(l.semaphore.available_permits(), 0);
        drop(b);
        assert_eq!(l.semaphore.available_permits(), 1);
        assert_eq!(snap(&l), (1, 0, 0));
    }

    #[tokio::test]
    async fn raising_again_cancels_pending_debt() {
        let l = JudgeLimiter::new(2);
        let a = l.acquire().await.unwrap_or_else(|_| unreachable!());
        let b = l.acquire().await.unwrap_or_else(|_| unreachable!());
        l.set_limit(1);
        assert_eq!(l.lock_state().debt, 1);
        l.set_limit(3);
        assert_eq!(l.lock_state().debt, 0);
        assert_eq!(l.semaphore.available_permits(), 1);
        drop(a);
        drop(b);
        assert_eq!(l.semaphore.available_permits(), 3);
    }

    #[tokio::test]
    async fn limit_never_drops_below_one() {
        let l = JudgeLimiter::new(0);
        assert_eq!(l.snapshot().limit, 1);
        l.set_limit(0);
        assert_eq!(l.snapshot().limit, 1);
        assert_eq!(l.semaphore.available_permits(), 1);
    }
}
