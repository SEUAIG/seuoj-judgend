use crate::error::Result;
use crate::fs;
use crate::judger::{CheckerType, ProblemInfo, ProblemType};
use crate::server::AppJson;
use axum::response::IntoResponse;
use axum::Json;
use base64::engine::general_purpose;
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{error, info, warn};

/// Program type.
#[derive(Debug, Clone, Serialize, Deserialize)]
enum ProgramType {
    #[serde(rename = "Binary")]
    Base64Binary,
    Source,
}

/// Interactor information for a problem.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Program {
    r#type: ProgramType,
    data: String,
}

/// Option Problem metadata and description.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct OptionProblem {
    pid: String,
    description: Option<String>,
    input: Option<String>,
    output: Option<String>,
    example: Option<Vec<OptionSample>>,
    info: Option<ProblemInfo>,
    interactor: Option<Program>,
    checker: Option<Program>,
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
    pub(crate) fn is_complete(&self) -> std::result::Result<(), String> {
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
        if let Some(info) = &self.info {
            if info.test_case_number.is_none() {
                warn!("Test case number is missing for problem id: {}", self.pid);
                return Err("Test case number is missing in problem info".to_string());
            }
            if info.problem_type == Some(ProblemType::Interactive) && self.interactor.is_none() {
                warn!(
                    "Interactor is missing for interactive problem id: {}",
                    self.pid
                );
                return Err("Interactor program is missing for interactive problem".to_string());
            }
            if info.checker_type == Some(CheckerType::Special) && self.checker.is_none() {
                warn!(
                    "Checker is missing for special judge problem id: {}",
                    self.pid
                );
                return Err("Checker program is missing for special judge problem".to_string());
            }
            Ok(())
        } else {
            warn!("Problem info is missing for problem id: {}", self.pid);
            Err("Problem info is missing".to_string())
        }
    }
}

pub(crate) async fn edit_problem_by_id(
    AppJson(payload): AppJson<OptionProblem>,
) -> Result<impl IntoResponse> {
    let problem_id = &payload.pid;
    info!("Received edit problem request: pid={}", problem_id,);
    let info_path = fs::get_path_by_id_name(problem_id, "info.json", false).await?;
    let is_new = !info_path.exists();
    if is_new {
        info!("Creating new problem with id: {}", &problem_id);
        payload.is_complete().map_err(|e| {
            error!("Invalid payload for new problem id {}: {}", problem_id, e);
            crate::error::AijError::Request(format!("Invalid payload for new problem: {}", e))
        })?;
    }
    if let Some(description) = &payload.description {
        let path = fs::get_path_by_id_name(problem_id, "description.md", false).await?;
        fs::write_to_file(&path, description).await?;
    }
    if let Some(input) = &payload.input {
        let path = fs::get_path_by_id_name(problem_id, "input.md", false).await?;
        fs::write_to_file(&path, input).await?;
    }
    if let Some(output) = &payload.output {
        let path = fs::get_path_by_id_name(problem_id, "output.md", false).await?;
        fs::write_to_file(&path, output).await?;
    }
    if let Some(example) = &payload.example {
        for (index, sample) in example.iter().enumerate() {
            if let Some(r#in) = &sample.r#in {
                let path = fs::get_path_by_id_name(
                    problem_id,
                    &format!("example_{}.in", index + 1),
                    false,
                )
                    .await?;
                fs::write_to_file(&path, r#in).await?;
            }
            if let Some(ans) = &sample.ans {
                let path = fs::get_path_by_id_name(
                    problem_id,
                    &format!("example_{}.ans", index + 1),
                    false,
                )
                    .await?;
                fs::write_to_file(&path, ans).await?;
            }
            if let Some(description) = &sample.description {
                let path = fs::get_path_by_id_name(
                    problem_id,
                    &format!("example_{}.md", index + 1),
                    false,
                )
                    .await?;
                fs::write_to_file(&path, description).await?;
            }
        }
    }
    if let Some(info) = payload.info {
        let info = if is_new {
            info
        } else {
            let mut problem_info = ProblemInfo::from_pid(problem_id).await?;
            problem_info.update_from_option(&info);
            problem_info
        };
        let info_json = serde_json::to_string_pretty(&info).map_err(|e| {
            error!(
                "Failed to serialize problem info for problem id {}: {}",
                problem_id, e
            );
            crate::error::AijError::Server(format!("Failed to serialize problem info: {}", e))
        })?;
        fs::write_to_file(&info_path, &info_json).await?;
    }
    if let Some(interactor) = &payload.interactor {
        match interactor.r#type {
            ProgramType::Base64Binary => {
                let path = fs::get_path_by_id_name(problem_id, "interactor", false).await?;
                let data = general_purpose::STANDARD
                    .decode(&interactor.data)
                    .map_err(|e| {
                        error!(
                            "Failed to decode interactor base64 data for problem id {}: {}",
                            problem_id, e
                        );
                        crate::error::AijError::Request(format!(
                            "Failed to decode interactor base64 data: {}",
                            e
                        ))
                    })?;
                fs::write_to_file(&path, &data).await?;
            }
            ProgramType::Source => {
                let path = fs::get_path_by_id_name(problem_id, "interactor.cpp", false).await?;
                fs::write_to_file(&path, &interactor.data).await?;
                crate::judger::compile(&path).await?;
            }
        }
    }
    if let Some(checker) = &payload.checker {
        match checker.r#type {
            ProgramType::Base64Binary => {
                let path = fs::get_path_by_id_name(problem_id, "checker", false).await?;
                let data = general_purpose::STANDARD
                    .decode(&checker.data)
                    .map_err(|e| {
                        error!(
                            "Failed to decode checker base64 data for problem id {}: {}",
                            problem_id, e
                        );
                        crate::error::AijError::Request(format!(
                            "Failed to decode checker base64 data: {}",
                            e
                        ))
                    })?;
                fs::write_to_file(&path, &data).await?;
            }
            ProgramType::Source => {
                let path = fs::get_path_by_id_name(problem_id, "checker.cpp", false).await?;
                fs::write_to_file(&path, &checker.data).await?;
                crate::judger::compile(&path).await?;
            }
        }
    }
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
