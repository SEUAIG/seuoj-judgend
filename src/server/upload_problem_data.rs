use crate::error::{AijError, Result};
use crate::fs;
use crate::server::AppJson;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{error, info};

/// Test case for a problem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TestCase {
    id: i32,
    r#in: String,
    ans: String,
}


/// Test case for a problem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ProblemTestCases {
    pid: String,
    testcase: Vec<TestCase>,
}

impl ProblemTestCases {
    /// Check if testcases are valid
    fn check(&mut self) -> std::result::Result<(), String> {
        if self.testcase.is_empty() {
            return Err("Testcase list is empty".to_string());
        }
        self.testcase.sort_unstable_by_key(|x| { x.id });

        for (index, tc) in self.testcase.iter().enumerate() {
            if tc.id != (index + 1) as i32 {
                return Err("Testcase id is not continuous".to_string());
            }
        }

        Ok(())
    }
}

pub(crate) async fn upload_problem_data(
    AppJson(mut payload): AppJson<ProblemTestCases>,
) -> Result<impl IntoResponse> {
    info!("Uploading test cases for problem id: {}", &payload.pid);

    payload.check().map_err(|e| {
        error!("Invalid test cases: {}", e);
        AijError::Request(format!("Invalid test cases: {}", e))
    })?;

    for tc in &payload.testcase {
        let input_path = fs::get_path_by_id_name(&payload.pid, &format!("{}.in", tc.id), false).await?;
        fs::write_to_file(&input_path, &tc.r#in).await?;
        let answer_path = fs::get_path_by_id_name(&payload.pid, &format!("{}.ans", tc.id), false).await?;
        fs::write_to_file(&answer_path, &tc.ans).await?;
    }

    info!("Successfully uploaded {} test cases for problem id: {}", payload.testcase.len(), &payload.pid);

    Ok(Json(json!(
        {
            "code": 0,
            "message": "Success",
        }
    )))
}