use crate::error::Result;
use crate::fs;
use crate::fs::check_problem_exists;
use crate::schema::{ProblemExample, ProblemMetadata};
use crate::server::AppJson;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{error, info, warn};

/// Option Problem metadata and description.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct OptionProblem {
    pid: String,
    description: Option<String>,
    input: Option<String>,
    output: Option<String>,
    example: Option<Vec<OptionSample>>,
    hint: Option<String>,
}

/// Option Sample input/output pair for a problem.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OptionSample {
    r#in: Option<String>,
    ans: Option<String>,
    description: Option<String>,
}

impl OptionProblem {
    /// Check if all fields are not None
    pub(crate) fn check_complete(&self) -> std::result::Result<(), String> {
        if self.description.is_none() {
            warn!("Description is missing for problem id: {}", self.pid);
            return Err("Description is missing".to_string());
        }
        if self.input.is_none() {
            warn!("Input is missing for problem id: {}", self.pid);
            return Err("Input is missing".to_string());
        }
        if self.output.is_none() {
            warn!("Output is missing for problem id: {}", self.pid);
            return Err("Output is missing".to_string());
        }
        if self.hint.is_none() {
            warn!("Hint is missing for problem id: {}", self.pid);
            return Err("Hint is missing".to_string());
        }
        if let Some(example) = &self.example {
            for (index, sample) in example.iter().enumerate() {
                if sample.r#in.is_none() {
                    warn!(
                        "Example {} input is missing for problem id: {}",
                        index + 1,
                        self.pid
                    );
                    return Err(format!("Example {} input is missing", index + 1));
                }
            }
        } else {
            warn!("Examples are missing for problem id: {}", self.pid);
            return Err("Examples are missing".to_string());
        }
        Ok(())
    }
}

pub(crate) async fn edit_problem_by_id(
    AppJson(mut payload): AppJson<OptionProblem>,
) -> Result<impl IntoResponse> {
    let problem_id = &payload.pid;
    info!("Received edit problem request: pid={}", problem_id,);
    let is_new = !check_problem_exists(problem_id).await?;

    if is_new {
        payload.check_complete().map_err(|e| {
            error!("Validation failed for new problem: {}", e);
            crate::error::AijError::Request(
                StatusCode::BAD_REQUEST,
                "INCOMPLETE_PROBLEM_DATA".to_string(),
                format!("Validation failed for new problem: {}", e),
            )
        })?;
    }

    // Handle problem metadata (problem.json)
    // Read existing metadata if it exists
    let mut metadata = if !is_new
        && let Ok(content) = fs::read_file_by_id_name(problem_id, "problem.json").await
    {
        // Try to read existing problem.json
        serde_json::from_str::<ProblemMetadata>(&content).map_err(|e| {
            error!("Failed to parse existing problem.json: {}", e);
            crate::error::AijError::Request(
                StatusCode::INTERNAL_SERVER_ERROR,
                "FAILED_PARSE_PROBLEM_METADATA".to_string(),
                format!("Failed to parse existing problem.json: {}", e),
            )
        })?
    } else {
        ProblemMetadata {
            pid: problem_id.clone(),
            ..Default::default()
        }
    };
    if let Some(description) = payload.description.take() {
        metadata.description = description;
    }
    if let Some(input) = payload.input.take() {
        metadata.input = input;
    }
    if let Some(output) = payload.output.take() {
        metadata.output = output;
    }
    if let Some(hint) = payload.hint.take() {
        metadata.hint = hint;
    }
    if let Some(example) = payload.example.take() {
        metadata.example = example
            .into_iter()
            .map(|sample| ProblemExample {
                r#in: sample.r#in.unwrap_or_default(),
                ans: sample.ans.unwrap_or_default(),
                description: sample.description.unwrap_or_default(),
            })
            .collect();
    }

    metadata.save(problem_id).await?;

    info!(
        "Successfully {} problem with id: {}",
        if is_new { "created" } else { "edited" },
        problem_id
    );

    Ok(Json(json!(
        {
            "code": 0,
            "message": "Success",
        }
    )))
}
