// Runtime helpers for translating limits and invoking the sandboxed runner.
use crate::error::{AijError, Result};
use crate::judger::judge_result::JudgeResultType;
use axum::http::StatusCode;
use std::path::Path;

pub(crate) struct RunPaths<'a> {
    pub(crate) exec_path: &'a str,
    pub(crate) args: &'a [String],
    pub(crate) input_path: &'a Path,
    pub(crate) case_id: i32,
    pub(crate) tmp_dir: &'a Path,
}

pub(crate) struct ResourceLimits {
    pub(crate) time_limit_ms: Option<i32>,
    pub(crate) memory_limit_kb: Option<i64>,
}

pub(crate) fn build_run_config(
    base: &judger::Config,
    paths: RunPaths<'_>,
    limits: ResourceLimits,
) -> judger::Config {
    let mut config = base.clone();
    if let Some(time_limit) = limits.time_limit_ms {
        config.max_cpu_time = time_limit;
        config.max_real_time = time_limit * 2;
    }
    if let Some(memory_limit) = limits.memory_limit_kb {
        config.max_memory = memory_limit * 1024;
        config.max_stack = memory_limit * 1024;
        config.max_output_size = memory_limit * 1024;
    }
    config.exe_path = paths.exec_path.to_string();
    config.args = paths.args.to_vec();
    config.input_path = paths.input_path.to_string_lossy().to_string();
    config.output_path = paths
        .tmp_dir
        .join(format!("{}.out", paths.case_id))
        .to_string_lossy()
        .to_string();
    config.error_path = paths
        .tmp_dir
        .join(format!("{}.err", paths.case_id))
        .to_string_lossy()
        .to_string();
    config.log_path = paths
        .tmp_dir
        .join(format!("{}.log", paths.case_id))
        .to_string_lossy()
        .to_string();
    config
}

tokio::task_local! {
    /// CPU core of the judge host judging the current submission. Sandboxed
    /// runs and compilers started inside the scope are pinned to it.
    pub(crate) static JUDGE_CORE: usize;
}

/// The judge host core for the current task, if any.
pub(crate) fn current_judge_core() -> Option<usize> {
    JUDGE_CORE.try_with(|core| *core).ok()
}

fn cpu_set_of(cores: &[usize]) -> libc::cpu_set_t {
    // SAFETY: cpu_set_t is plain data; CPU_ZERO/CPU_SET only write inside it.
    unsafe {
        let mut set: libc::cpu_set_t = std::mem::zeroed();
        libc::CPU_ZERO(&mut set);
        for &core in cores {
            libc::CPU_SET(core, &mut set);
        }
        set
    }
}

/// Cores this process may run on, in ascending order.
pub(crate) fn available_cores() -> Vec<usize> {
    // SAFETY: sched_getaffinity writes at most size_of::<cpu_set_t>() bytes.
    unsafe {
        let mut set: libc::cpu_set_t = std::mem::zeroed();
        if libc::sched_getaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &mut set) != 0 {
            return Vec::new();
        }
        (0..libc::CPU_SETSIZE as usize)
            .filter(|&c| libc::CPU_ISSET(c, &set))
            .collect()
    }
}

/// Pin the calling thread (and processes it forks afterwards) to `cores`.
/// Safe to call between fork and exec.
pub(crate) fn pin_current_thread(cores: &[usize]) -> bool {
    let set = cpu_set_of(cores);
    // SAFETY: pid 0 means the calling thread; `set` is a valid cpu_set_t.
    unsafe { libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &set) == 0 }
}

/// Run `f` with the calling thread pinned to the current judge host's core,
/// restoring the previous affinity afterwards.
fn with_judge_core<T>(f: impl FnOnce() -> T) -> T {
    let Some(core) = current_judge_core() else {
        return f();
    };
    let previous = available_cores();
    let pinned = !previous.is_empty() && pin_current_thread(&[core]);
    let out = f();
    if pinned {
        pin_current_thread(&previous);
    }
    out
}

pub(crate) fn run_with_interactor(
    config: &judger::Config,
    interactor: Option<std::path::PathBuf>,
) -> Result<judger::RunResult> {
    with_judge_core(|| crate::sandbox::run_current(config, interactor)).map_err(|e| {
        AijError::Judge(
            StatusCode::INTERNAL_SERVER_ERROR,
            "JUDGER_RUN_FAILED".to_string(),
            format!("Judger run failed: {}", e),
        )
    })
}

pub(crate) fn map_runtime_status(code: judger::ErrorCode) -> (String, JudgeResultType) {
    match code {
        judger::ErrorCode::Success => ("Success".to_string(), JudgeResultType::Accepted),
        judger::ErrorCode::CpuTimeLimitExceeded | judger::ErrorCode::RealTimeLimitExceeded => (
            "Time Limit Exceeded".to_string(),
            JudgeResultType::TimeLimitExceeded,
        ),
        judger::ErrorCode::MemoryLimitExceeded => (
            "Memory Limit Exceeded".to_string(),
            JudgeResultType::MemoryLimitExceeded,
        ),
        judger::ErrorCode::RuntimeError => {
            ("Runtime Error".to_string(), JudgeResultType::RuntimeError)
        }
        judger::ErrorCode::SystemError => {
            ("System Error".to_string(), JudgeResultType::SystemError)
        }
        judger::ErrorCode::WrongAnswer(s) => (s, JudgeResultType::WrongAnswer),
        other => (
            format!("Unexpected judger result: {:?}", other),
            JudgeResultType::SystemError,
        ),
    }
}
