// Judge one offline testcase, including run + checker + result assembly.
use crate::config::AijConfig;
use crate::error::{AijError, Result};
use crate::fs;
use crate::fs::{get_path_by_pid_name, get_text_by_path};
use crate::judger::checker::{self, CheckerResult};
use crate::judger::judge_result::{JudgeResultItem, JudgeResultType};
use crate::judger::judger_helpers::chmod_plus_x;
use crate::judger::runtime::{ResourceLimits, RunPaths, build_run_config, run_with_interactor};
use crate::schema::{CheckerType, CustomModules, ProblemType, TestCaseConfig};
use axum::http::StatusCode;
use tracing::{info, warn};

pub(crate) async fn judge_single_case(
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
    let (input_path, ans_path) =
        resolve_case_paths(case_config, pid, problem_type, tmp_dir).await?;
    let config = build_run_config(
        judge_config,
        RunPaths {
            exec_path: &exec_path,
            args: &args,
            input_path: &input_path,
            case_id: case_config.id,
            tmp_dir,
        },
        ResourceLimits {
            time_limit_ms: case_config.time_limit_ms,
            memory_limit_kb: case_config.memory_limit_kb,
        },
    );
    let interactor = resolve_interactor_path(pid, problem_type, custom_modules).await?;
    info!(
        "Judging submission {} on test case {} with interactor: {:?} AND config: {:?}",
        submission_id, case_config.id, interactor, config
    );
    let res = run_with_interactor(&config, interactor)?;
    info!(
        "Judger result for test case {} of submission {}: {:?}",
        case_config.id, submission_id, res
    );
    build_case_result(CaseResultContext {
        case_id: case_config.id,
        pid,
        submission_id,
        problem_type,
        checker_type,
        custom_modules,
        config: &config,
        ans_path: &ans_path,
        res,
    })
    .await
}

async fn resolve_case_paths(
    case_config: &TestCaseConfig,
    pid: &str,
    problem_type: ProblemType,
    tmp_dir: &std::path::Path,
) -> Result<(std::path::PathBuf, std::path::PathBuf)> {
    let input_path =
        get_path_by_pid_name(pid, format!("data/{}", case_config.in_path), true).await?;
    let ans_path = match problem_type {
        ProblemType::Standard => {
            get_path_by_pid_name(pid, format!("data/{}", case_config.ans_path), true).await?
        }
        ProblemType::Special => {
            if case_config.ans_path.is_empty() {
                let fallback_ans_path = tmp_dir.join(format!("{}.ans", case_config.id));
                fs::write_to_file(&fallback_ans_path, []).await?;
                fallback_ans_path
            } else {
                get_path_by_pid_name(pid, format!("data/{}", case_config.ans_path), true).await?
            }
        }
        ProblemType::Interactive => tmp_dir.join(format!("{}.ans", case_config.id)),
    };
    Ok((input_path, ans_path))
}

async fn resolve_interactor_path(
    pid: &str,
    problem_type: ProblemType,
    custom_modules: &Option<CustomModules>,
) -> Result<Option<std::path::PathBuf>> {
    match problem_type {
        ProblemType::Standard | ProblemType::Special => Ok(None),
        ProblemType::Interactive => {
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
                pid,
                format!("data/{}", interactor_exec_path.to_string_lossy()),
                true,
            )
            .await?;
            chmod_plus_x(&path).await?;
            Ok(Some(path))
        }
    }
}

struct CaseResultContext<'a> {
    case_id: i32,
    pid: &'a str,
    submission_id: &'a str,
    problem_type: ProblemType,
    checker_type: CheckerType,
    custom_modules: &'a Option<CustomModules>,
    config: &'a judger::Config,
    ans_path: &'a std::path::Path,
    res: judger::RunResult,
}

async fn build_case_result(ctx: CaseResultContext<'_>) -> Result<JudgeResultItem> {
    let truncated_len = AijConfig::get().output_truncate_length;
    let in_content = get_text_by_path(&ctx.config.input_path, Some(truncated_len)).await?;
    let ans_content = match ctx.problem_type {
        ProblemType::Standard | ProblemType::Special => {
            get_text_by_path(ctx.ans_path, Some(truncated_len)).await?
        }
        ProblemType::Interactive => Default::default(),
    };
    let out_content = get_text_by_path(&ctx.config.output_path, Some(truncated_len)).await?;
    let mut score = 0;
    let (sys, r#type) = match ctx.res.result {
        judger::ErrorCode::Success => {
            let mut result = ("Accepted".to_string(), JudgeResultType::Accepted);
            if ctx.problem_type != ProblemType::Interactive {
                match checker::check(
                    ctx.pid,
                    &ctx.config.input_path,
                    &ctx.config.output_path,
                    ctx.ans_path,
                    ctx.checker_type,
                    ctx.custom_modules
                        .as_ref()
                        .and_then(|m| m.checker_path.as_deref()),
                    ctx.res.cpu_time as u64,
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
                ctx.submission_id, ctx.case_id, ctx.res
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
                    ctx.res, ctx.submission_id, ctx.case_id
                ),
            ));
        }
    };
    Ok(JudgeResultItem {
        id: ctx.case_id,
        time: ctx.res.cpu_time,
        mem: ctx.res.memory,
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
