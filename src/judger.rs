//! Judger module for handling code submission judging.
use crate::config::AijConfig;
use crate::error::{AijError, Result};
use crate::fs::{get_dir_by_submission_id, get_path_by_id_name, get_text_by_path};
pub(crate) use crate::judger::utils::{
    CheckerType, ProblemCase, ProblemInfo, ProblemType, compile,
};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use utils::chmod_plus_x;

mod checker;
mod utils;

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
    let mut problem_info = ProblemInfo::from_pid(&pid).await?;
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
    let (exec_path, args, seccomp_rule) = match language {
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
            (exec_path, vec![], judger::SeccompRuleName::CCpp)
        }
        SupportedLanguages::Python3_12 => {
            problem_info.max_real_time_ms = match problem_info.max_real_time_ms {
                Some(-1) => Some(-1),
                Some(m) => Some(m * 2),
                None => None,
            };
            problem_info.max_cpu_time_ms = match problem_info.max_cpu_time_ms {
                Some(-1) => Some(-1),
                Some(m) => Some(m * 2),
                None => None,
            };
            (
                "/usr/bin/python3".to_string(),
                vec![
                    "/usr/bin/python3".to_string(),
                    source_file_path.to_string_lossy().to_string(),
                ],
                judger::SeccompRuleName::Python,
            )
        }
        SupportedLanguages::Nodejs22 => (
            "/usr/bin/node".to_string(),
            vec![
                "/usr/bin/node".to_string(),
                source_file_path.to_string_lossy().to_string(),
            ],
            judger::SeccompRuleName::Node,
        ),
        SupportedLanguages::Go1_22 => {
            let exec_path = tmp_dir.join("executable").to_string_lossy().to_string();
            let compile_output = tokio::process::Command::new("go")
                .arg("build")
                .arg("-o")
                .arg(&exec_path)
                .arg(&source_file_path)
                .output()
                .await
                .map_err(|e| AijError::Judge(format!("Failed to compile source code: {}", e)))?;
            if !compile_output.status.success() {
                let stderr = String::from_utf8_lossy(&compile_output.stderr);
                return Ok(JudgeResult::CompileError(stderr.to_string()));
            }
            (exec_path, vec![], judger::SeccompRuleName::Golang)
        }
        SupportedLanguages::Java17 => {
            return Err(AijError::Judge(format!(
                "Language {:?} not yet supported",
                language
            )));
        }
    };
    let mut config = problem_info.to_judger_config();
    config.seccomp_rule_name = Some(seccomp_rule);
    let mut out_vec = vec![];
    let case_info = ProblemCase::from_pid(&pid).await?;
    let problem_type = problem_info.problem_type.ok_or_else(|| {
        AijError::Judge(format!(
            "Problem type is not specified for problem `{}` (expected in info.json)",
            pid
        ))
    })?;
    let checker_type = problem_info.checker_type.ok_or_else(|| {
        AijError::Judge(format!(
            "Checker type is not specified for problem `{}` (expected in info.json)",
            pid
        ))
    })?;
    for case in case_info.0 {
        let input_path = get_path_by_id_name(&pid, case.in_name, true).await?;
        let ans_path = match problem_type {
            ProblemType::Standard => {
                get_path_by_id_name(
                    &pid,
                    case.ans_name.ok_or_else(|| {
                        AijError::Judge(format!(
                            "Answer file name is not specified for test case {} of problem {}",
                            case.id, pid
                        ))
                    })?,
                    true,
                )
                .await?
            }
            ProblemType::Interactive => tmp_dir.join(format!("{}.ans", case.id)),
        };
        let mut config = config.clone();
        config.exe_path = exec_path.clone();
        config.args = args.clone();
        config.input_path = input_path.to_string_lossy().to_string();
        config.output_path = tmp_dir
            .join(format!("{}.out", case.id))
            .to_string_lossy()
            .to_string();
        config.error_path = tmp_dir
            .join(format!("{}.err", case.id))
            .to_string_lossy()
            .to_string();
        config.log_path = tmp_dir
            .join(format!("{}.log", case.id))
            .to_string_lossy()
            .to_string();
        let interactor = match problem_type {
            ProblemType::Standard => None,
            ProblemType::Interactive => Some({
                let path = get_path_by_id_name(&pid, "interactor", true).await?;
                if !path.exists() {
                    return Err(AijError::FileSystem(format!(
                        "Interactor file does not exist: {}",
                        path.to_string_lossy()
                    )));
                }
                chmod_plus_x(&path).await.map_err(|e| {
                    AijError::FileSystem(format!(
                        "Failed to set execute permission for interactor {}: {}",
                        path.to_string_lossy(),
                        e
                    ))
                })?;
                path
            }),
        };
        info!(
            "Judging submission {} on test case {} with interactor: {:?} AND config: {:?}",
            submission_id, case.id, interactor, config
        );
        let res = judger::run(&config, interactor)
            .map_err(|e| AijError::Judge(format!("Judger run failed: {}", e)))?;
        info!(
            "Judger result for test case {} of submission {}: {:?}",
            case.id, submission_id, res
        );
        let truncated_len = AijConfig::get().output_truncate_length;
        let in_content = get_text_by_path(&config.input_path, Some(truncated_len)).await?;
        let ans_content = match problem_type {
            ProblemType::Standard => get_text_by_path(&ans_path, Some(truncated_len)).await?,
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
                    submission_id, case.id, res
                );
                warn!("{err_info}");
                (err_info, "SystemError")
            }
            _ => {
                return Err(AijError::Judge(format!(
                    "Unexpected judger result: {:?} for submission {} on test case {}",
                    res, submission_id, case.id
                )));
            }
        };
        out_vec.push(JudgeResultItem {
            cnt: case.id,
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
    use crate::judger::{ProblemCase, ProblemInfo, ProblemType};

    #[tokio::test]
    async fn test_problem_info_from_pid() {
        let pid = "1";
        let info = ProblemInfo::from_pid(pid).await;
        assert!(info.is_ok());
        let info = info.unwrap();
        assert_eq!(info.problem_type, Some(ProblemType::Standard));
    }

    #[tokio::test]
    async fn test_case_info_from_pid() {
        let pid = "1";
        let cases = ProblemCase::from_pid(pid).await;
        assert!(cases.is_ok());
        let cases = cases.unwrap();
        assert_eq!(cases.0.len(), 1);
    }
}
