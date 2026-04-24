//! Judger module for handling code submission judging.

use crate::config::AijConfig;
use crate::error::{AijError, Result};
use crate::fs;
use crate::fs::{get_dir_by_submission_id, get_path_by_pid_name, get_text_by_path};
use crate::schema::{
    CheckerType, CustomModules, ProblemConfig, ProblemType, SubtaskConfig, TestCaseConfig,
};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};
use utils::chmod_plus_x;

mod checker;
mod utils;

use crate::judger::checker::CheckerResult;
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
    let tmp_dir = get_dir_by_submission_id(&submission_id, true).await?;
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
    let mut source_file_path = tmp_dir.join("source").with_extension(source_file_extension);

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
                return Ok(JudgeResult::CompileError {
                    detail: stderr.to_string(),
                });
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
            problem_config.problem_info.time_limit_ms =
                match problem_config.problem_info.time_limit_ms {
                    -1 => -1,
                    m => m * 2,
                };
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
                return Ok(JudgeResult::CompileError {
                    detail: stderr.to_string(),
                });
            }
            problem_config.problem_info.memory_limit_kb =
                match problem_config.problem_info.memory_limit_kb {
                    -1 => -1,
                    m => m * 2,
                };
            (exec_path, vec![], judger::SeccompRuleName::Golang)
        }
        SupportedLanguages::Java17 => {
            let new_source_file_path = source_file_path
                .with_file_name("Main")
                .with_extension("java");
            fs::rename(&source_file_path, &new_source_file_path).await?;
            source_file_path = new_source_file_path;
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
                return Ok(JudgeResult::CompileError {
                    detail: stderr.to_string(),
                });
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
    let problem_type = problem_config.problem_info.problem_type;
    let checker_type = problem_config.problem_info.checker_type;
    let custom_modules = problem_config.custom_modules.clone();

    let topo_order = utils::get_topo_order(&problem_config.subtasks)?;
    info!(
        "Topological order of subtasks for problem {}: {:?}",
        pid, topo_order
    );
    let mut subtasks = problem_config.subtasks;
    if !topo_order.is_empty() {
        let mut error_map = HashMap::new();
        for sub_id in topo_order {
            info!("Judging subtask {} of submission {}", sub_id, submission_id);
            let subtask_config = subtasks.iter().find(|s| s.id == sub_id).ok_or_else(|| {
                AijError::Judge(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "SUBTASK_NOT_FOUND".to_string(),
                    format!("Subtask with id {} not found in problem config", sub_id),
                )
            })?;
            let have_error = subtask_config
                .pre_subtasks
                .iter()
                .any(|pre_id| error_map.get(pre_id).copied().unwrap_or(false));
            let mut scores = Vec::new();
            for case_id in &subtask_config.cases {
                let case_config = problem_config
                    .testcases
                    .iter()
                    .find(|c| c.id == *case_id)
                    .ok_or_else(|| {
                        AijError::Judge(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "TESTCASE_NOT_FOUND".to_string(),
                            format!("Test case with id {} not found in problem config", case_id),
                        )
                    })?;
                if have_error {
                    out_vec.push(JudgeResultItem {
                        id: case_config.id,
                        sys: "Skipped due to previous error in subtask".to_string(),
                        r#type: JudgeResultType::Skipped,
                        ..Default::default()
                    });
                    continue;
                }
                let res = judge_single_case(
                    case_config,
                    (&pid, &submission_id),
                    (problem_type, checker_type, &custom_modules),
                    &judge_config,
                    exec_path.clone(),
                    args.clone(),
                    &tmp_dir,
                )
                .await?;
                scores.push(res.score);
                out_vec.push(res.clone());
                if res.r#type != JudgeResultType::Accepted {
                    error_map.insert(sub_id, true);
                }
            }
            let subtask_type = subtasks
                .iter()
                .find(|s| s.id == sub_id)
                .map(|s| s.r#type.as_str())
                .unwrap_or("min");
            match subtask_type {
                "min" => {
                    let min_score = scores.into_iter().min().unwrap_or(0);
                    if let Some(subtask_config) = subtasks.iter_mut().find(|s| s.id == sub_id) {
                        subtask_config.score =
                            ((min_score * subtask_config.score) as f64 / 100.0) as i32;
                    } else {
                        return Err(AijError::Judge(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "SUBTASK_NOT_FOUND".to_string(),
                            format!("Subtask with id {} not found in problem config", sub_id),
                        ));
                    }
                }
                "sum" => {
                    let len = scores.len();
                    let avg_score: f64 = if len > 0 {
                        scores.into_iter().sum::<i32>() as f64 / len as f64
                    } else {
                        0.0
                    };
                    if let Some(subtask_config) = subtasks.iter_mut().find(|s| s.id == sub_id) {
                        subtask_config.score =
                            ((avg_score * subtask_config.score as f64) / 100.0) as i32;
                    }
                }
                _ => {
                    return Err(AijError::Judge(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "INVALID_SUBTASK_TYPE".to_string(),
                        format!("Invalid subtask type: {}", subtask_type),
                    ));
                }
            }
        }
    } else {
        let sum_weight: f64 = problem_config.testcases.iter().map(|s| s.weight).sum();
        if sum_weight == 0.0 {
            return Err(AijError::Judge(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INVALID_TESTCASE_WEIGHT".to_string(),
                "Sum of test case weights cannot be zero".to_string(),
            ));
        }
        for case_config in &problem_config.testcases {
            let mut res = judge_single_case(
                case_config,
                (&pid, &submission_id),
                (problem_type, checker_type, &custom_modules),
                &judge_config,
                exec_path.clone(),
                args.clone(),
                &tmp_dir,
            )
            .await?;
            res.score = (res.score as f64 * case_config.weight / sum_weight) as i32;
            out_vec.push(res);
        }
    }
    Ok(JudgeResult::MaybeError {
        results: out_vec,
        subtask_configs: subtasks,
    })
}

async fn judge_single_case(
    case_config: &TestCaseConfig,
    (pid, submission_id): (&str, &str),
    (problem_type, checker_type, custom_modules): (
        ProblemType,
        CheckerType,
        &Option<CustomModules>,
    ),
    judge_config: &judger::Config,
    exec_path: String,
    args: Vec<String>,
    tmp_dir: &std::path::Path,
) -> Result<JudgeResultItem> {
    let input_path =
        get_path_by_pid_name(&pid, format!("data/{}", case_config.in_path), true).await?;
    let ans_path = match problem_type {
        ProblemType::Standard => {
            get_path_by_pid_name(&pid, format!("data/{}", case_config.ans_path), true).await?
        }
        ProblemType::Special => {
            if case_config.ans_path.is_empty() {
                let fallback_ans_path = tmp_dir.join(format!("{}.ans", case_config.id));
                fs::write_to_file(&fallback_ans_path, []).await?;
                fallback_ans_path
            } else {
                get_path_by_pid_name(&pid, format!("data/{}", case_config.ans_path), true).await?
            }
        }
        ProblemType::Interactive => tmp_dir.join(format!("{}.ans", case_config.id)),
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
        .join(format!("{}.out", case_config.id))
        .to_string_lossy()
        .to_string();
    config.error_path = tmp_dir
        .join(format!("{}.err", case_config.id))
        .to_string_lossy()
        .to_string();
    config.log_path = tmp_dir
        .join(format!("{}.log", case_config.id))
        .to_string_lossy()
        .to_string();
    let interactor = match problem_type {
        ProblemType::Standard | ProblemType::Special => None,
        ProblemType::Interactive => Some({
            let interactor_path = custom_modules
                .as_ref()
                .and_then(|m| m.interactor_path.as_deref())
                .ok_or_else(|| {
                    AijError::Judge(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "INTERACTOR_PATH_MISSING".to_string(),
                        format!(
                            "Interactor path is missing in problem config for interactive problem {}",
                            pid
                        ),
                    )
                })?;
            let interactor_exec_path = if interactor_path.ends_with(".cpp") {
                std::path::Path::new(interactor_path).with_extension("")
            } else {
                std::path::PathBuf::from(interactor_path)
            };
            let path = get_path_by_pid_name(
                &pid,
                format!("data/{}", interactor_exec_path.to_string_lossy()),
                true,
            )
            .await?;
            chmod_plus_x(&path).await?;
            path
        }),
    };
    info!(
        "Judging submission {} on test case {} with interactor: {:?} AND config: {:?}",
        submission_id, case_config.id, interactor, config
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
        case_config.id, submission_id, res
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
    let mut score = 0;
    let (sys, r#type) = match res.result {
        judger::ErrorCode::Success => {
            let mut result = ("Accepted".to_string(), JudgeResultType::Accepted);
            if problem_type != ProblemType::Interactive {
                match checker::check(
                    &pid,
                    &config.input_path,
                    &config.output_path,
                    &ans_path,
                    checker_type,
                    custom_modules
                        .as_ref()
                        .and_then(|m| m.checker_path.as_deref()),
                )
                .await?
                {
                    CheckerResult::Accepted => {}
                    CheckerResult::PartiallyAccepted(score_f, detail) => {
                        let score_i = (score_f * 100.0) as i32;
                        result = (detail, JudgeResultType::PartiallyAccepted);
                        score = score_i;
                    }
                    CheckerResult::WrongAnswer(detail) => {
                        result = (detail, JudgeResultType::WrongAnswer)
                    }
                }
            }
            result
        }
        judger::ErrorCode::WrongAnswer(s) => (s, JudgeResultType::WrongAnswer),
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
            let err_info = format!(
                "Judger System Error on submission {} test case {}: {:?}",
                submission_id, case_config.id, res
            );
            warn!("{err_info}");
            (err_info, JudgeResultType::SystemError)
        }
        _ => {
            return Err(AijError::Judge(
                StatusCode::INTERNAL_SERVER_ERROR,
                "UNEXPECTED_JUDGER_RESULT".to_string(),
                format!(
                    "Unexpected judger result: {:?} for submission {} on test case {}",
                    res, submission_id, case_config.id
                ),
            ));
        }
    };
    Ok(JudgeResultItem {
        id: case_config.id,
        time: res.cpu_time,
        mem: res.memory,
        sys,
        r#in: in_content,
        ans: ans_content,
        out: out_content,
        score: if r#type == JudgeResultType::Accepted {
            100
        } else {
            score
        },
        r#type,
    })
}

/// Type of the judging result for a single test case
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub enum JudgeResultType {
    /// Accepted: the output matches the expected answer
    #[default]
    Accepted,
    /// Skipped: the test case was skipped due to a previous error in the same subtask
    Skipped,
    /// Partially Accepted: the output is partially correct, with a score between 0 and 100
    PartiallyAccepted,
    /// Wrong Answer: the output does not match the expected answer
    WrongAnswer,
    /// Time Limit Exceeded: the program exceeded the time limit for execution
    TimeLimitExceeded,
    /// Memory Limit Exceeded: the program exceeded the memory limit for execution
    MemoryLimitExceeded,
    /// Runtime Error: the program crashed or encountered a runtime error during execution
    RuntimeError,
    /// System Error: an error occurred in the judger system itself, not related to the user's code
    SystemError,
}

/// Result of once judging
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct JudgeResultItem {
    /// count of the test case
    pub(crate) id: i32,
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
    pub(crate) r#type: JudgeResultType,
    /// score of the test case
    pub(crate) score: i32,
}

/// Result of the judging process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum JudgeResult {
    CompileError {
        detail: String,
    },
    MaybeError {
        results: Vec<JudgeResultItem>,
        subtask_configs: Vec<SubtaskConfig>,
    },
}

#[cfg(test)]
mod tests {
    use crate::schema::{ProblemConfig, ProblemMetadata};

    #[tokio::test]
    async fn test_problem_info_from_pid() {
        let pid = "test01";
        let metadata = ProblemMetadata::from_pid(pid).await;
        println!("Metadata for problem {}: {:?}", pid, metadata);
        assert!(metadata.is_ok());
        let metadata = metadata.unwrap();
        assert_eq!(metadata.example.len(), 1);
        assert_eq!(metadata.example[0].ans, "3");
    }

    #[tokio::test]
    async fn test_case_info_from_pid() {
        let pid = "test01";
        let cases = ProblemConfig::from_pid(pid).await;
        println!("Cases for problem {}: {:?}", pid, cases);
        assert!(cases.is_ok());
        let cases = cases.unwrap();
        assert_eq!(cases.testcases.len(), 1);
    }
}
