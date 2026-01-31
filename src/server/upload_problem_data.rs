use crate::error::{AijError, Result};
use crate::fs;
use crate::fs::Case;
use crate::judger::{ProblemCase, ProblemInfo, ProblemType};
use crate::server::AppJson;
use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{error, info};

/// Test case for a problem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ProblemTestCases {
    pid: String,
    testcase: Vec<Case>,
}

impl ProblemTestCases {
    /// Check if testcases are valid
    fn check_complete(&mut self, problem_type: &ProblemType) -> std::result::Result<(), String> {
        if self.testcase.is_empty() {
            return Err("Testcase list is empty".to_string());
        }
        self.testcase.sort_unstable_by_key(|x| x.id);

        for (index, tc) in self.testcase.iter().enumerate() {
            if tc.id != index + 1 {
                return Err("Testcase id is not continuous".to_string());
            }
            if tc.r#in.is_none() {
                return Err(format!("Testcase {} is missing input", tc.id));
            }
            // Interactive problems may not have answers
            if problem_type != &ProblemType::Interactive
                && (tc.ans.is_none() || tc.ans_name.is_none())
            {
                return Err(format!("Testcase {} is missing answer", tc.id));
            }
        }

        Ok(())
    }
}

pub(crate) async fn upload_problem_data(
    AppJson(mut payload): AppJson<ProblemTestCases>,
) -> Result<impl IntoResponse> {
    info!("Uploading test cases for problem id: {}", &payload.pid);

    if !fs::check_problem_exists(&payload.pid).await? {
        error!("Problem id: {} does not exist", &payload.pid);
        return Err(AijError::Request(
            StatusCode::UNPROCESSABLE_ENTITY,
            "PROBLEM_NOT_FOUND".to_string(),
            format!("Problem id: {} does not exist", &payload.pid),
        ));
    }
    let problem_type = ProblemInfo::from_pid(&payload.pid)
        .await?
        .problem_type
        .ok_or_else(|| {
            AijError::Request(
                StatusCode::UNPROCESSABLE_ENTITY,
                "PROBLEM_TYPE_NOT_FOUND".to_string(),
                format!("Problem id: {} has no problem type", &payload.pid),
            )
        })?;

    payload.check_complete(&problem_type).map_err(|e| {
        error!("Invalid test cases: {}", e);
        AijError::Request(
            StatusCode::UNPROCESSABLE_ENTITY,
            "INVALID_TEST_CASES".to_string(),
            format!("Invalid test cases: {}", e),
        )
    })?;

    let mut case_info = ProblemCase::from_pid(&payload.pid).await?;
    case_info.clear_cases(&payload.pid).await?;

    for tc in &mut payload.testcase {
        if let Some(r#in) = &tc.r#in {
            let input_path = fs::get_path_by_id_name(&payload.pid, &tc.in_name, false).await?;
            fs::write_to_file(&input_path, r#in).await?;
        }
        tc.r#in = None;
        if problem_type == ProblemType::Standard
            && let Some(ans) = &tc.ans
            && let Some(ans_name) = &tc.ans_name
        {
            let answer_path = fs::get_path_by_id_name(&payload.pid, ans_name, false).await?;
            fs::write_to_file(&answer_path, ans).await?;
        }
        tc.ans = None;
    }

    case_info.0 = payload.testcase;
    case_info.save(&payload.pid).await?;

    info!(
        "Successfully uploaded {} test cases for problem id: {}",
        case_info.len(),
        &payload.pid
    );

    Ok(Json(json!(
        {
            "code": 0,
            "message": "Success",
        }
    )))
}
