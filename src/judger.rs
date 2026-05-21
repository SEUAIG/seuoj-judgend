//! Judger module for handling code submission judging.

use crate::error::Result;
use crate::fs::get_dir_by_submission_id;
use crate::schema::{OnlineCase, ProblemConfig};
use serde::Deserialize;
use strum::{Display, EnumIter};

mod build_plan;
mod case_judge;
mod checker;
mod judge_result;
mod judger_helpers;
mod offline_cases;
mod online_cases;
mod runtime;

use crate::judger::build_plan::{PrepareOutcome, prepare_execution};
pub(crate) use crate::judger::judge_result::{JudgeResult, JudgeResultItem, JudgeResultType};
use crate::judger::offline_cases::run_offline_cases;
use crate::judger::online_cases::run_online_cases;
pub(crate) use judger_helpers::compile;
use tracing::warn;

fn check_code_length(code: &str, max_code_length: i64) -> Option<JudgeResult> {
    if max_code_length < 0 {
        return None;
    }
    let max_length = max_code_length as usize;
    if code.len() > max_length {
        let message = format!(
            "Code length {} exceeds maximum allowed length {}",
            code.len(),
            max_length
        );
        warn!("{}", message);
        return Some(JudgeResult::CodeTooLong { detail: message });
    }
    None
}

/// Supported programming languages for the judger system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, EnumIter, Display)]
pub(crate) enum SupportedLanguages {
    C,
    Cpp,
    Cpp20,
    Python,
    Nodejs,
    Go,
    Java,
}

pub(crate) async fn judge(
    pid: String,
    code: String,
    language: SupportedLanguages,
    submission_id: String,
) -> Result<JudgeResult> {
    let mut problem_config = ProblemConfig::from_pid(&pid).await?;
    if let Some(code_too_long) =
        check_code_length(&code, problem_config.problem_info.max_code_length)
    {
        return Ok(code_too_long);
    }
    let tmp_dir = get_dir_by_submission_id(&submission_id, true).await?;
    let (judge_config, exec_path, args) =
        match prepare_execution(&mut problem_config, &tmp_dir, code, language).await? {
            PrepareOutcome::Plan(plan) => (plan.judge_config, plan.exec_path, plan.args),
            PrepareOutcome::CompileError(detail) => {
                return Ok(JudgeResult::CompileError { detail });
            }
            PrepareOutcome::Success(_) => unreachable!(),
        };
    let (out_vec, subtasks) = run_offline_cases(
        &pid,
        &submission_id,
        &problem_config,
        &judge_config,
        &exec_path,
        &args,
        &tmp_dir,
    )
    .await?;
    Ok(JudgeResult::MaybeError {
        results: out_vec,
        subtask_configs: subtasks,
    })
}

pub(crate) async fn judge_online(
    pid: String,
    code: String,
    language: SupportedLanguages,
    submission_id: String,
    testcases: Vec<OnlineCase>,
) -> Result<JudgeResult> {
    let mut problem_config = ProblemConfig::from_pid(&pid).await?;
    if let Some(code_too_long) =
        check_code_length(&code, problem_config.problem_info.max_code_length)
    {
        return Ok(code_too_long);
    }
    let tmp_dir = get_dir_by_submission_id(&submission_id, true).await?;
    let (judge_config, exec_path, args) =
        match prepare_execution(&mut problem_config, &tmp_dir, code, language).await? {
            PrepareOutcome::Plan(plan) => (plan.judge_config, plan.exec_path, plan.args),
            PrepareOutcome::CompileError(detail) => {
                return Ok(JudgeResult::CompileError { detail });
            }
            PrepareOutcome::Success(_) => unreachable!(),
        };
    run_online_cases(testcases, &judge_config, &exec_path, &args, &tmp_dir).await
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
