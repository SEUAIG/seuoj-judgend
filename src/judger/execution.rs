use crate::error::{AijError, Result};
use crate::judger::types::JudgeResultType;
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

pub(crate) fn run_with_interactor(
    config: &judger::Config,
    interactor: Option<std::path::PathBuf>,
) -> Result<judger::RunResult> {
    judger::run(config, interactor).map_err(|e| {
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
