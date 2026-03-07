use crate::error::Result;
use crate::fs;
use crate::schema::{CheckerType, ProblemConfig, ProblemType};
use crate::server::AppJson;
use axum::Json;
use axum::extract::Path;
use axum::response::IntoResponse;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use tracing::{error, info};

pub(crate) async fn put_problem_config(
    Path(pid): Path<String>,
    AppJson(problem_config): AppJson<ProblemConfig>,
) -> Result<impl IntoResponse> {
    info!("Received request to update config for problem ID '{}'", pid);

    fs::assert_problem_exists(&pid).await?;

    match problem_config.problem_info.problem_type {
        ProblemType::Interactive => {
            if problem_config.problem_info.checker_type == CheckerType::Interactor {
                if let Some(custom_modules) = &problem_config.custom_modules
                    && let Some(interactor_path) = &custom_modules.interactor_path
                {
                    fs::validate_filename(interactor_path)?;
                    let interactor_full_path =
                        fs::get_path_by_id_name(&pid, format!("data/{}", interactor_path), true)
                            .await?;
                    if interactor_path.contains(".cpp") {
                        crate::judger::compile(interactor_full_path).await?;
                    }
                } else {
                    return Err(crate::error::AijError::Request(
                        axum::http::StatusCode::BAD_REQUEST,
                        "INTERACTOR_WITHOUT_PATH".to_string(),
                        "Checker type is 'Interactor' but no interactor path provided".to_string(),
                    ));
                }
            } else {
                return Err(crate::error::AijError::Request(
                    axum::http::StatusCode::BAD_REQUEST,
                    "INVALID_CHECKER_TYPE".to_string(),
                    "Checker type must be 'Interactor' for interactive problems".to_string(),
                ));
            }
        }
        ProblemType::Special => {
            if problem_config.problem_info.checker_type == CheckerType::Special {
                if let Some(custom_modules) = &problem_config.custom_modules
                    && let Some(checker_path) = &custom_modules.checker_path
                {
                    fs::validate_filename(checker_path)?;
                    let checker_full_path =
                        fs::get_path_by_id_name(&pid, format!("data/{}", checker_path), true)
                            .await?;
                    if checker_path.contains(".cpp") {
                        crate::judger::compile(checker_full_path).await?;
                    }
                } else {
                    return Err(crate::error::AijError::Request(
                        axum::http::StatusCode::BAD_REQUEST,
                        "CHECKER_WITHOUT_PATH".to_string(),
                        "Checker type is 'Special' but no checker path provided".to_string(),
                    ));
                }
            } else {
                return Err(crate::error::AijError::Request(
                    axum::http::StatusCode::BAD_REQUEST,
                    "INVALID_CHECKER_TYPE".to_string(),
                    "Checker type must be 'Special' for special problems".to_string(),
                ));
            }
        }
        _ => {}
    }

    let mut existing_id = HashSet::new();
    for case in &problem_config.testcases {
        if !existing_id.insert(case.id) {
            return Err(crate::error::AijError::Request(
                axum::http::StatusCode::BAD_REQUEST,
                "DUPLICATE_TEST_CASE_ID".to_string(),
                format!("Duplicate test case ID: {}", case.id),
            ));
        }
        if case.weight <= 0.0 {
            return Err(crate::error::AijError::Request(
                axum::http::StatusCode::BAD_REQUEST,
                "INVALID_TEST_CASE_WEIGHT".to_string(),
                format!(
                    "Test case ID {} has non-positive weight: {}",
                    case.id, case.weight
                ),
            ));
        }
        let in_path = &case.in_path;
        fs::validate_filename(in_path)?;
        let _ = fs::get_path_by_id_name(&pid, format!("data/{in_path}"), true).await?;
        let ans_path = &case.ans_path;
        if !ans_path.is_empty() {
            fs::validate_filename(ans_path)?;
            let _ = fs::get_path_by_id_name(&pid, format!("data/{ans_path}"), true).await?;
        }
    }
    let all_case_ids = existing_id;
    if !problem_config.subtasks.is_empty() {
        let all_subtask_ids: HashSet<i32> = problem_config
            .subtasks
            .iter()
            .map(|subtask| subtask.id)
            .collect();
        let mut existing_subtask_id = HashSet::new();
        let mut sum_score = 0;
        let mut graph = HashMap::new();
        for subtask in &problem_config.subtasks {
            sum_score += subtask.score;
            if !existing_subtask_id.insert(subtask.id) {
                return Err(crate::error::AijError::Request(
                    axum::http::StatusCode::BAD_REQUEST,
                    "DUPLICATE_SUBTASK_ID".to_string(),
                    format!("Duplicate subtask ID: {}", subtask.id),
                ));
            }
            for case_id in &subtask.cases {
                if !all_case_ids.contains(case_id) {
                    return Err(crate::error::AijError::Request(
                        axum::http::StatusCode::BAD_REQUEST,
                        "SUBTASK_CASE_ID_NOT_FOUND".to_string(),
                        format!(
                            "Subtask {} references non-existent test case ID: {}",
                            subtask.id, case_id
                        ),
                    ));
                }
            }
            for &pre_id in &subtask.pre_subtasks {
                if !all_subtask_ids.contains(&pre_id) {
                    return Err(crate::error::AijError::Request(
                        axum::http::StatusCode::BAD_REQUEST,
                        "SUBTASK_PRE_ID_NOT_FOUND".to_string(),
                        format!(
                            "Subtask {} references non-existent prerequisite subtask ID: {}",
                            subtask.id, pre_id
                        ),
                    ));
                }
                graph
                    .entry(pre_id)
                    .or_insert_with(Vec::new)
                    .push(subtask.id);
            }
        }
        if sum_score != 100 {
            return Err(crate::error::AijError::Request(
                axum::http::StatusCode::BAD_REQUEST,
                "INVALID_SUBTASK_SCORE".to_string(),
                format!("Sum of subtask scores must be 100, but got {}", sum_score),
            ));
        }
        // Check for cycles using DFS
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        for &subtask_id in existing_subtask_id.iter() {
            if !visited.contains(&subtask_id)
                && has_cycle(subtask_id, &graph, &mut visited, &mut rec_stack)
            {
                error!(
                    "Cycle detected in subtask dependencies for problem ID '{}'",
                    pid
                );
                return Err(crate::error::AijError::Request(
                    axum::http::StatusCode::BAD_REQUEST,
                    "CYCLE_IN_SUBTASK_DEPENDENCY".to_string(),
                    "Cycle detected in subtask dependencies".to_string(),
                ));
            }
        }
    }

    problem_config.save(&pid).await?;

    info!("Successfully updated config for problem ID '{}'", pid);
    Ok(Json(json!({
        "code": 0,
        "message": "Success",
    })))
}

fn has_cycle(
    u: i32,
    adj: &HashMap<i32, Vec<i32>>,
    visited: &mut HashSet<i32>,
    rec_stack: &mut HashSet<i32>,
) -> bool {
    visited.insert(u);
    rec_stack.insert(u);

    if let Some(neighbors) = adj.get(&u) {
        for &v in neighbors {
            if rec_stack.contains(&v) {
                return true;
            }
            if !visited.contains(&v) && has_cycle(v, adj, visited, rec_stack) {
                return true;
            }
        }
    }

    rec_stack.remove(&u);
    false
}
