use crate::error::{AijError, Result};
use crate::fs;
use crate::schema::{CheckerType, ProblemConfig};
use crate::server::utils::build_response_from_file_content;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use std::path::PathBuf;
use tracing::{info, warn};

pub(crate) async fn get_problem_file(
    Path((pid, filename)): Path<(String, String)>,
) -> Result<impl IntoResponse> {
    fs::assert_problem_exists(&pid).await?;
    let file_content = match filename.as_str() {
        "data.zip" => {
            let data_paths = fs::get_data_paths_by_id(&pid).await?;
            info!("Creating zip stream for data files of problem {}", pid,);
            fs::create_zip_stream(data_paths).await?
        }
        _ => {
            fs::validate_filename(&filename)?;
            let file_path =
                fs::get_path_by_pid_name(&pid, format!("data/{}", filename), true).await?;
            fs::get_stream_by_path(file_path).await?
        }
    };

    build_response_from_file_content(pid, filename, file_content)
}

pub(crate) async fn delete_problem_file(
    Path((pid, filename)): Path<(String, String)>,
) -> Result<impl IntoResponse> {
    info!(
        "Received request to delete file {} of problem {}",
        filename, pid
    );
    fs::validate_filename(&filename)?;
    let path = fs::get_path_by_pid_name(&pid, format!("data/{}", filename), false).await?;
    if !path.exists() {
        return Ok(StatusCode::NO_CONTENT);
    }
    let problem_config = ProblemConfig::from_pid(&pid).await?;
    if problem_config.problem_info.checker_type == CheckerType::Special
        && let Some(checker_path) = problem_config
            .custom_modules
            .clone()
            .and_then(|m| m.checker_path)
    {
        let checker_pathbuf = PathBuf::from(checker_path);
        let checker_without_ext = checker_pathbuf
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        if filename == checker_pathbuf || filename == checker_without_ext {
            let message = format!(
                "Cannot delete file '{}' because it is used as checker of problem {}",
                filename, pid
            );
            warn!("{}", message);
            return Err(AijError::Server(
                StatusCode::BAD_REQUEST,
                "FILE_IN_USE".to_string(),
                message,
            ));
        }
    }

    if problem_config.problem_info.checker_type == CheckerType::Interactor
        && let Some(interactor_path) = problem_config
            .custom_modules
            .clone()
            .and_then(|m| m.interactor_path)
    {
        let interactor_pathbuf = PathBuf::from(interactor_path);
        let interactor_without_ext = interactor_pathbuf
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        if filename == interactor_pathbuf || filename == interactor_without_ext {
            let message = format!(
                "Cannot delete file '{}' because it is used as interactor of problem {}",
                filename, pid
            );
            warn!("{}", message);
            return Err(AijError::Server(
                StatusCode::BAD_REQUEST,
                "FILE_IN_USE".to_string(),
                message,
            ));
        }
    }

    for case in problem_config.testcases {
        if case.in_path == filename || case.ans_path == filename {
            let message = format!(
                "Cannot delete file '{}' because it is used in testcases of problem {}",
                filename, pid
            );
            warn!("{}", message);
            return Err(AijError::Server(
                StatusCode::BAD_REQUEST,
                "FILE_IN_USE".to_string(),
                message,
            ));
        }
    }
    let file_path = fs::get_path_by_pid_name(&pid, format!("data/{}", filename), true).await?;
    fs::delete_file(&file_path).await?;
    Ok(StatusCode::NO_CONTENT)
}
