//! Judger module for handling code submission judging.

use crate::config::AijConfig;
use crate::error::{AijError, Result};
use crate::fs;
use crate::fs::{get_dir_by_submission_id, get_path_by_id_name, get_text_by_path};
use crate::schema::{ProblemConfig, ProblemType};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use utils::chmod_plus_x;

mod checker;
mod utils;

pub(crate) use utils::compile;

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

pub(crate) async fn judge(
    pid: String,
    code: String,
    language: SupportedLanguages,
    submission_id: String,
) -> Result<JudgeResult> {
    let mut problem_config = ProblemConfig::from_pid(&pid).await?;
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
    let source_file_path = tmp_dir.join("Main").with_extension(source_file_extension);

    fs::write_to_file(&source_file_path, code).await?;
    let (exec_path, args, seccomp_rule) = match language {
        SupportedLanguages::C
        | SupportedLanguages::Cpp
        | SupportedLanguages::Cpp11
        | SupportedLanguages::Cpp17
        | SupportedLanguages::Cpp20 => {
            // Compile the code
            let exec_path = tmp_dir.join("executable").to_string_lossy().to_string();
            let compile_output = if language == SupportedLanguages::C {
                let gcc = AijConfig::get_binary_path("gcc").await?;
                tokio::process::Command::new(gcc)
                    .arg(&source_file_path)
                    .arg("-o")
                    .arg(&exec_path)
                    .output()
                    .await
            } else {
                let gpp = AijConfig::get_binary_path("g++").await?;
                let mut cmd = tokio::process::Command::new(gpp);
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
            .map_err(|e| {
                AijError::Judge(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "COMPILE_ERROR".to_string(),
                    format!("Failed to compile source code: {}", e),
                )
            })?;
            if !compile_output.status.success() {
                let stderr = String::from_utf8_lossy(&compile_output.stderr);
                return Ok(JudgeResult::CompileError(stderr.to_string()));
            }
            (exec_path, vec![], judger::SeccompRuleName::CCpp)
        }
        SupportedLanguages::Python3_12 => {
            problem_config.problem_info.time_limit_ms =
                match problem_config.problem_info.time_limit_ms {
                    -1 => -1,
                    m => m * 2,
                };
            let python3 = AijConfig::get_binary_path("python3").await?;
            (
                python3.to_string_lossy().to_string(),
                vec![
                    python3.to_string_lossy().to_string(),
                    source_file_path.to_string_lossy().to_string(),
                ],
                judger::SeccompRuleName::Python,
            )
        }
        SupportedLanguages::Nodejs22 => {
            let nodejs = AijConfig::get_binary_path("node").await?;
            (
                nodejs.to_string_lossy().to_string(),
                vec![
                    nodejs.to_string_lossy().to_string(),
                    source_file_path.to_string_lossy().to_string(),
                ],
                judger::SeccompRuleName::Node,
            )
        }
        SupportedLanguages::Go1_22 => {
            let exec_path = tmp_dir.join("executable").to_string_lossy().to_string();
            let go_bin = AijConfig::get_binary_path("go").await?;
            let compile_output = tokio::process::Command::new(go_bin)
                .arg("build")
                .arg("-o")
                .arg(&exec_path)
                .arg(&source_file_path)
                .output()
                .await
                .map_err(|e| {
                    AijError::Judge(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "COMPILE_ERROR".to_string(),
                        format!("Failed to compile source code: {}", e),
                    )
                })?;
            if !compile_output.status.success() {
                let stderr = String::from_utf8_lossy(&compile_output.stderr);
                return Ok(JudgeResult::CompileError(stderr.to_string()));
            }
            (exec_path, vec![], judger::SeccompRuleName::Golang)
        }
        SupportedLanguages::Java17 => {
            problem_config.problem_info.time_limit_ms =
                match problem_config.problem_info.time_limit_ms {
                    -1 => -1,
                    m => m * 2,
                };
            let javac = AijConfig::get_binary_path("javac").await?;
            let compile_output = tokio::process::Command::new(javac)
                .arg(&source_file_path)
                .output()
                .await
                .map_err(|e| {
                    AijError::Judge(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "COMPILE_ERROR".to_string(),
                        format!("Failed to compile source code: {}", e),
                    )
                })?;
            if !compile_output.status.success() {
                let stderr = String::from_utf8_lossy(&compile_output.stderr);
                return Ok(JudgeResult::CompileError(stderr.to_string()));
            }
            let mut args = vec![
                "java".to_string(),
                "-cp".to_string(),
                tmp_dir.to_string_lossy().to_string(),
                "Main".to_string(),
            ];
            if problem_config.problem_info.memory_limit_kb != -1 {
                args.insert(
                    1,
                    format!("-Xmx{}m", problem_config.problem_info.memory_limit_kb / 512),
                );
            };
            let java = AijConfig::get_binary_path("java").await?;
            (
                java.to_string_lossy().to_string(),
                args,
                judger::SeccompRuleName::Java,
            )
        }
    };
    let mut judge_config = problem_config.problem_info.to_judger_config();
    judge_config.seccomp_rule_name = Some(seccomp_rule);
    let mut out_vec = vec![];
    let case_map = problem_config.testcases;
    let problem_type = problem_config.problem_info.problem_type;
    let checker_type = problem_config.problem_info.checker_type;
    // todo! subtask judge
    // for now we just judge all test cases and return the result list
    for (id, case_config) in case_map {
        let input_path =
            get_path_by_id_name(&pid, format!("data/{}", case_config.in_path), true).await?;
        let ans_path = match problem_type {
            ProblemType::Standard | ProblemType::Special => {
                get_path_by_id_name(&pid, format!("data/{}", case_config.ans_path), true).await?
            }
            ProblemType::Interactive => tmp_dir.join(format!("{}.ans", id)),
        };
        let mut config = judge_config.clone();
        if let Some(time_limit) = case_config.time_limit_ms {
            config.max_cpu_time = time_limit;
            config.max_real_time = time_limit * 2;
        }
        if let Some(memory_limit) = case_config.memory_limit_kb {
            config.max_memory = memory_limit * 1024;
            config.max_stack = memory_limit * 1024;
            config.max_output_size = memory_limit * 1024;
        }
        config.exe_path = exec_path.clone();
        config.args = args.clone();
        config.input_path = input_path.to_string_lossy().to_string();
        config.output_path = tmp_dir
            .join(format!("{}.out", id))
            .to_string_lossy()
            .to_string();
        config.error_path = tmp_dir
            .join(format!("{}.err", id))
            .to_string_lossy()
            .to_string();
        config.log_path = tmp_dir
            .join(format!("{}.log", id))
            .to_string_lossy()
            .to_string();
        let interactor = match problem_type {
            ProblemType::Standard | ProblemType::Special => None,
            ProblemType::Interactive => Some({
                let path = get_path_by_id_name(&pid, "data/interactor", true).await?;
                chmod_plus_x(&path).await?;
                path
            }),
        };
        info!(
            "Judging submission {} on test case {} with interactor: {:?} AND config: {:?}",
            submission_id, id, interactor, config
        );
        let res = judger::run(&config, interactor).map_err(|e| {
            AijError::Judge(
                StatusCode::INTERNAL_SERVER_ERROR,
                "JUDGER_RUN_FAILED".to_string(),
                format!("Judger run failed: {}", e),
            )
        })?;
        info!(
            "Judger result for test case {} of submission {}: {:?}",
            id, submission_id, res
        );
        let truncated_len = AijConfig::get().output_truncate_length;
        let in_content = get_text_by_path(&config.input_path, Some(truncated_len)).await?;
        let ans_content = match problem_type {
            ProblemType::Standard | ProblemType::Special => {
                get_text_by_path(&ans_path, Some(truncated_len)).await?
            }
            ProblemType::Interactive => Default::default(),
        };
        let out_content = get_text_by_path(&config.output_path, Some(truncated_len)).await?;
        let (sys, r#type) = match res.result {
            judger::ErrorCode::Success => {
                let mut result = ("Accepted".to_string(), "Accepted");
                if problem_type != ProblemType::Interactive {
                    let (res, detail) = checker::check(
                        &pid,
                        &config.input_path,
                        &config.output_path,
                        &ans_path,
                        checker_type,
                    )
                    .await?;
                    if !res {
                        result = (detail, "WrongAnswer")
                    }
                }
                result
            }
            judger::ErrorCode::WrongAnswer(s) => (s, "WrongAnswer"),
            judger::ErrorCode::CpuTimeLimitExceeded | judger::ErrorCode::RealTimeLimitExceeded => {
                ("Time Limit Exceeded".to_string(), "TimeLimitExceeded")
            }
            judger::ErrorCode::MemoryLimitExceeded => {
                ("Memory Limit Exceeded".to_string(), "MemoryLimitExceeded")
            }
            judger::ErrorCode::RuntimeError => ("Runtime Error".to_string(), "RuntimeError"),
            judger::ErrorCode::SystemError => {
                let err_info = format!(
                    "Judger System Error on submission {} test case {}: {:?}",
                    submission_id, id, res
                );
                warn!("{err_info}");
                (err_info, "SystemError")
            }
            _ => {
                return Err(AijError::Judge(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "UNEXPECTED_JUDGER_RESULT".to_string(),
                    format!(
                        "Unexpected judger result: {:?} for submission {} on test case {}",
                        res, submission_id, id
                    ),
                ));
            }
        };
        out_vec.push(JudgeResultItem {
            cnt: id.parse::<usize>().unwrap_or(0),
            time: res.cpu_time,
            mem: res.memory,
            sys,
            r#in: in_content,
            ans: ans_content,
            out: out_content,
            r#type: r#type.to_string(),
        });
    }
    Ok(JudgeResult::MaybeError(out_vec))
}

/// Result of once judging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct JudgeResultItem {
    /// count of the test case
    pub(crate) cnt: usize,
    /// real time used in milliseconds
    pub(crate) time: i32,
    /// memory used in bytes
    pub(crate) mem: i64,
    /// output of system
    pub(crate) sys: String,
    /// input of test case
    pub(crate) r#in: String,
    /// expected answer of test case
    pub(crate) ans: String,
    /// output of user code
    pub(crate) out: String,
    /// type of the result
    pub(crate) r#type: String,
}

/// Result of the judging process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum JudgeResult {
    CompileError(String),
    MaybeError(Vec<JudgeResultItem>),
}

#[cfg(test)]
mod tests {
    use crate::schema::{ProblemConfig, ProblemMetadata};

    #[tokio::test]
    async fn test_problem_info_from_pid() {
        let pid = "1";
        let metadata = ProblemMetadata::from_pid(pid).await;
        println!("Metadata for problem {}: {:?}", pid, metadata);
        assert!(metadata.is_ok());
        let metadata = metadata.unwrap();
        assert_eq!(metadata.example.len(), 1);
        assert_eq!(metadata.example[0].ans, "3");
    }

    #[tokio::test]
    async fn test_case_info_from_pid() {
        let pid = "1";
        let cases = ProblemConfig::from_pid(pid).await;
        println!("Cases for problem {}: {:?}", pid, cases);
        assert!(cases.is_ok());
        let cases = cases.unwrap();
        assert_eq!(cases.testcases.len(), 1);
    }
}
