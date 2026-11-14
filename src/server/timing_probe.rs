//! Background timing probe: runs one fixed program in the sandbox every few
//! seconds, so the admin page can show how measured time drifts with load.

use crate::config::AijConfig;
use crate::server::judgehosts::judge_hosts;
use serde::Serialize;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use tokio::process::Command;
use tracing::{info, warn};

/// Seconds between two probe runs.
pub(crate) const PROBE_INTERVAL_SECS: u64 = 15;
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

pub(crate) struct ProbeState {
    pub(crate) samples: VecDeque<ProbeSample>,
    pub(crate) error: Option<String>,
}

static PROBE: Mutex<ProbeState> = Mutex::new(ProbeState {
    samples: VecDeque::new(),
    error: None,
});

pub(crate) fn probe_state() -> std::sync::MutexGuard<'static, ProbeState> {
    PROBE.lock().unwrap_or_else(|e| e.into_inner())
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
    let at = super::judgehosts::now_millis();
    let before = judge_hosts().running();
    let config = config.clone();
    let result = tokio::task::spawn_blocking(move || crate::sandbox::run_current(&config, None))
        .await
        .map_err(|e| format!("probe task: {}", e))??;
    let after = judge_hosts().running();
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
