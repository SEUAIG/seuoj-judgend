// Run all offline testcases with subtask dependency and scoring logic.
use crate::error::{AijError, Result};
use crate::judger::case_judge::judge_single_case;
use crate::judger::{JudgeResultItem, JudgeResultType};
use crate::schema::{CheckerType, CustomModules, ProblemConfig, ProblemType, SubtaskConfig};
use axum::http::StatusCode;
use std::collections::HashMap;
use tracing::info;

pub(crate) async fn run_offline_cases(
    pid: &str,
    submission_id: &str,
    problem_config: &ProblemConfig,
    judge_config: &judger::Config,
    exec_path: &str,
    args: &[String],
    tmp_dir: &std::path::Path,
) -> Result<(Vec<JudgeResultItem>, Vec<SubtaskConfig>)> {
    let mut out_vec = vec![];
    let problem_type = problem_config.problem_info.problem_type;
    let checker_type = problem_config.problem_info.checker_type;
    let custom_modules = problem_config.custom_modules.clone();

    let topo_order = crate::judger::judger_helpers::get_topo_order(&problem_config.subtasks)?;
    info!(
        "Topological order of subtasks for problem {}: {:?}",
        pid, topo_order
    );
    let mut subtasks = problem_config.subtasks.clone();
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
                    (pid, submission_id),
                    (problem_type, checker_type, &custom_modules),
                    judge_config,
                    exec_path.to_string(),
                    args.to_vec(),
                    tmp_dir,
                )
                .await?;
                scores.push(res.score);
                out_vec.push(res.clone());
                if res.r#type != JudgeResultType::Accepted {
                    error_map.insert(sub_id, true);
                }
            }
            apply_subtask_score(sub_id, &scores, &mut subtasks)?;
        }
    } else {
        run_weighted_cases(
            pid,
            submission_id,
            problem_config,
            problem_type,
            checker_type,
            &custom_modules,
            judge_config,
            exec_path,
            args,
            tmp_dir,
            &mut out_vec,
        )
        .await?;
    }

    Ok((out_vec, subtasks))
}

fn apply_subtask_score(sub_id: i32, scores: &[i32], subtasks: &mut [SubtaskConfig]) -> Result<()> {
    let subtask_type = subtasks
        .iter()
        .find(|s| s.id == sub_id)
        .map(|s| s.r#type.as_str())
        .unwrap_or("min");
    match subtask_type {
        "min" => {
            let min_score = scores.iter().copied().min().unwrap_or(0);
            if let Some(subtask_config) = subtasks.iter_mut().find(|s| s.id == sub_id) {
                subtask_config.score = ((min_score * subtask_config.score) as f64 / 100.0) as i32;
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
                scores.iter().sum::<i32>() as f64 / len as f64
            } else {
                0.0
            };
            if let Some(subtask_config) = subtasks.iter_mut().find(|s| s.id == sub_id) {
                subtask_config.score = ((avg_score * subtask_config.score as f64) / 100.0) as i32;
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
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn run_weighted_cases(
    pid: &str,
    submission_id: &str,
    problem_config: &ProblemConfig,
    problem_type: ProblemType,
    checker_type: CheckerType,
    custom_modules: &Option<CustomModules>,
    judge_config: &judger::Config,
    exec_path: &str,
    args: &[String],
    tmp_dir: &std::path::Path,
    out_vec: &mut Vec<JudgeResultItem>,
) -> Result<()> {
    let sum_weight: i32 = problem_config.testcases.iter().map(|s| s.weight).sum();
    if sum_weight != 100 {
        return Err(AijError::Judge(
            StatusCode::INTERNAL_SERVER_ERROR,
            "INVALID_TESTCASE_WEIGHT".to_string(),
            format!(
                "Sum of test case weights must be 100, but got {}",
                sum_weight
            ),
        ));
    }
    for case_config in &problem_config.testcases {
        let mut res = judge_single_case(
            case_config,
            (pid, submission_id),
            (problem_type, checker_type, custom_modules),
            judge_config,
            exec_path.to_string(),
            args.to_vec(),
            tmp_dir,
        )
        .await?;
        res.score = (res.score * case_config.weight) / 100;
        out_vec.push(res);
    }
    Ok(())
}
