use crate::error::{AijError, Result};
use crate::fs::{get_dir_by_submission_id, get_path_by_id_name};
use judger::Config;
use serde::{Deserialize, Serialize};

mod comparer;

/// Supported programming languages for the judger system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum SupportedLanguages {
    C,
    Cpp,
    Cpp11,
    Cpp17,
    Cpp20,
    Python3_12,
    Nodejs22,
    Go1_22,
    Java17,
}

/// Information about a problem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ProblemInfo {
    /// Maximum CPU time in milliseconds (-1 for unlimited).
    pub(crate) max_cpu_time_ms: Option<i32>,
    /// Maximum real time in milliseconds (-1 for unlimited).
    pub(crate) max_real_time_ms: Option<i32>,
    /// Maximum memory in bytes (-1 for unlimited).
    pub(crate) max_memory_byte: Option<i64>,
    /// Maximum stack size in bytes.
    pub(crate) max_stack_byte: Option<i64>,
    /// Maximum number of processes (-1 for unlimited).
    pub(crate) max_process_number: Option<i32>,
    /// Maximum output size in bytes (-1 for unlimited).
    pub(crate) max_output_size_byte: Option<i64>,
    /// Number of test cases
    pub(crate) test_case_number: i32,
    /// type of the problem
    pub(crate) problem_type: ProblemType,
}

/// Type of the problem
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ProblemType {
    /// Standard IO problem
    Standard,
    /// Interactive problem
    Interactive,
}

impl ProblemInfo {
    pub(crate) async fn from_pid(pid: &str) -> Result<Self> {
        let content = crate::fs::read_file_by_id_name(pid, "info.json").await?;
        let info: ProblemInfo = serde_json::from_str(&content).map_err(|e| {
            AijError::FileSystem(format!(
                "Failed to parse info.json for problem {}: {}",
                pid, e
            ))
        })?;
        Ok(info)
    }

    pub(crate) fn to_judger_config(&self) -> Config {
        let mut config = Config::default();
        if let Some(cpu_time) = self.max_cpu_time_ms {
            config.max_cpu_time = cpu_time;
        }
        if let Some(real_time) = self.max_real_time_ms {
            config.max_real_time = real_time;
        }
        if let Some(memory) = self.max_memory_byte {
            config.max_memory = memory;
        }
        if let Some(stack) = self.max_stack_byte {
            config.max_stack = stack;
        }
        if let Some(process_number) = self.max_process_number {
            config.max_process_number = process_number;
        }
        if let Some(output_size) = self.max_output_size_byte {
            config.max_output_size = output_size;
        }
        config
    }
}

pub(crate) async fn judge(
    pid: String,
    code: String,
    language: SupportedLanguages,
    submission_id: String,
) -> Result<JudgeResult> {
    let problem_info = ProblemInfo::from_pid(&pid).await?;
    let tmp_dir = get_dir_by_submission_id(&submission_id).await?;
    let source_file_extension = match language {
        SupportedLanguages::C => "c",
        SupportedLanguages::Cpp
        | SupportedLanguages::Cpp11
        | SupportedLanguages::Cpp17
        | SupportedLanguages::Cpp20 => "cpp",
        SupportedLanguages::Python3_12 => "py",
        SupportedLanguages::Nodejs22 => "js",
        SupportedLanguages::Go1_22 => "go",
        SupportedLanguages::Java17 => "java",
    };
    let source_file_path = tmp_dir
        .join("source_code")
        .with_extension(source_file_extension);

    tokio::fs::write(&source_file_path, code)
        .await
        .map_err(|e| {
            AijError::FileSystem(format!(
                "Failed to write source code to {}: {}",
                source_file_path.to_string_lossy(),
                e
            ))
        })?;
    let (exec_path, args) = match language {
        SupportedLanguages::C
        | SupportedLanguages::Cpp
        | SupportedLanguages::Cpp11
        | SupportedLanguages::Cpp17
        | SupportedLanguages::Cpp20 => {
            // Compile the code
            let exec_path = tmp_dir.join("executable").to_string_lossy().to_string();
            let compile_output = if language == SupportedLanguages::C {
                tokio::process::Command::new("gcc")
                    .arg(&source_file_path)
                    .arg("-o")
                    .arg(&exec_path)
                    .output()
                    .await
            } else {
                let mut cmd = tokio::process::Command::new("g++");
                match language {
                    SupportedLanguages::Cpp11 => {
                        cmd.arg("-std=c++11");
                    }
                    SupportedLanguages::Cpp17 => {
                        cmd.arg("-std=c++17");
                    }
                    SupportedLanguages::Cpp20 => {
                        cmd.arg("-std=c++20");
                    }
                    _ => {}
                }
                cmd.arg(&source_file_path)
                    .arg("-o")
                    .arg(&exec_path)
                    .output()
                    .await
            }
                .map_err(|e| AijError::Judge(format!("Failed to compile source code: {}", e)))?;
            if !compile_output.status.success() {
                let stderr = String::from_utf8_lossy(&compile_output.stderr);
                return Ok(JudgeResult::CompileError(stderr.to_string()));
            }
            (exec_path, vec![])
        }
        SupportedLanguages::Python3_12 => (
            "/usr/bin/python3".to_string(),
            vec![
                "/usr/bin/python3".to_string(),
                source_file_path.to_string_lossy().to_string(),
            ],
        ),
        SupportedLanguages::Nodejs22 | SupportedLanguages::Go1_22 | SupportedLanguages::Java17 => {
            return Err(AijError::Judge(format!(
                "Language {:?} not yet supported",
                language
            )));
        }
    };
    let config = problem_info.to_judger_config();
    for i in 1..=problem_info.test_case_number {
        let input_path = get_path_by_id_name(&pid, &format!("{}.in", i)).await?;
        let ans_path = get_path_by_id_name(&pid, &format!("{}.ans", i)).await?;
        let mut config = config.clone();
        config.exe_path = exec_path.clone();
        config.args = args.clone();
        config.input_path = input_path.to_string_lossy().to_string();
        config.output_path = tmp_dir
            .join(format!("{}.out", i))
            .to_string_lossy()
            .to_string();
        config.error_path = tmp_dir
            .join(format!("{}.err", i))
            .to_string_lossy()
            .to_string();
        config.log_path = tmp_dir
            .join(format!("{}.log", i))
            .to_string_lossy()
            .to_string();
        let interactor = match &problem_info.problem_type {
            ProblemType::Standard => None,
            ProblemType::Interactive => Some({
                let path = get_path_by_id_name(&pid, "interactor").await?;
                if !path.exists() {
                    return Err(AijError::FileSystem(format!(
                        "Interactor file does not exist: {}",
                        path.to_string_lossy()
                    )));
                }
                path
            }),
        };
        let res = judger::run(&config, interactor)
            .map_err(|e| AijError::Judge(format!("Judger run failed: {}", e)))?;
        match res.result {
            judger::ErrorCode::Success => {}
            judger::ErrorCode::CpuTimeLimitExceeded | judger::ErrorCode::RealTimeLimitExceeded => {
                return Ok(JudgeResult::TimeLimitExceeded);
            }
            judger::ErrorCode::MemoryLimitExceeded => {
                return Ok(JudgeResult::MemoryLimitExceeded);
            }
            judger::ErrorCode::RuntimeError => {
                return Ok(JudgeResult::RuntimeError);
            }
            judger::ErrorCode::SystemError => {
                return Ok(JudgeResult::SystemError);
            }
            _ => {
                return Err(AijError::Judge(format!(
                    "Unexpected judger result: {:?}",
                    res
                )));
            }
        }
        let compare_result = comparer::standard_comparer(&config.output_path, &ans_path).await?;
        if !compare_result {
            return Ok(JudgeResult::WrongAnswer(format!(
                "Wrong answer on test case {}",
                i
            )));
        }
    }
    // Placeholder for the judging logic
    Ok(JudgeResult::Accepted)
}

/// Result of the judging process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum JudgeResult {
    Accepted,
    WrongAnswer(String),
    TimeLimitExceeded,
    MemoryLimitExceeded,
    RuntimeError,
    CompileError(String),
    SystemError,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::FileSystem;

    #[tokio::test]
    async fn test_problem_info_from_pid() {
        FileSystem::init("/tmp/aij/problems").unwrap();
        let pid = "1";
        let info = ProblemInfo::from_pid(pid).await;
        assert!(info.is_ok());
        let info = info.unwrap();
        assert_eq!(info.test_case_number, 1);
        assert_eq!(info.problem_type, ProblemType::Standard);
    }
}
