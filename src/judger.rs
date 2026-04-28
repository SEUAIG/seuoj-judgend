//! Judger module for handling code submission judging.

use crate::error::Result;
use crate::fs::get_dir_by_submission_id;
use crate::schema::{OnlineCase, ProblemConfig};
use serde::Deserialize;

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
    let (judge_config, exec_path, args) =
        match prepare_execution(&mut problem_config, &tmp_dir, code, language).await? {
            PrepareOutcome::Plan(plan) => (plan.judge_config, plan.exec_path, plan.args),
            PrepareOutcome::CompileError(detail) => {
                return Ok(JudgeResult::CompileError { detail });
            }
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
    let tmp_dir = get_dir_by_submission_id(&submission_id, true).await?;
    let (judge_config, exec_path, args) =
        match prepare_execution(&mut problem_config, &tmp_dir, code, language).await? {
            PrepareOutcome::Plan(plan) => (plan.judge_config, plan.exec_path, plan.args),
            PrepareOutcome::CompileError(detail) => {
                return Ok(JudgeResult::CompileError { detail });
            }
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
