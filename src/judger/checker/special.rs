// Special checker wrapper that executes problem-provided checker binaries.
use crate::fs::get_path_by_pid_name;
use crate::judger::checker::CheckerResult;
use crate::judger::judger_helpers::chmod_plus_x;
use axum::http::StatusCode;
use std::path::Path;
use tracing::{error, info};

pub(crate) async fn special_checker(
    problem_id: impl AsRef<str>,
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    ans_path: impl AsRef<Path>,
    checker_path: Option<&str>,
) -> crate::error::Result<CheckerResult> {
    {
        info!("Using special checker for problem {}", problem_id.as_ref());
        let checker_path = checker_path.ok_or_else(|| {
            let message = format!(
                "Checker path is missing in problem config for special problem {}",
                problem_id.as_ref()
            );
            error!("{}", message);
            crate::error::AijError::Judge(
                StatusCode::INTERNAL_SERVER_ERROR,
                "CHECKER_PATH_MISSING".to_string(),
                message,
            )
        })?;
        let checker_exec_path = if checker_path.ends_with(".cpp") {
            Path::new(checker_path).with_extension("")
        } else {
            checker_path.into()
        };
        let checker_path = get_path_by_pid_name(
            problem_id.as_ref(),
            format!("data/{}", checker_exec_path.to_string_lossy()),
            true,
        )
        .await?;
        chmod_plus_x(&checker_path).await.map_err(|e| {
            let message = format!(
                "Failed to set execute permission for checker of problem {}: {}",
                problem_id.as_ref(),
                e
            );
            error!("{}", message);
            crate::error::AijError::Judge(
                StatusCode::INTERNAL_SERVER_ERROR,
                "CHECKER_PERMISSION_FAILED".to_string(),
                message,
            )
        })?;

        let output = tokio::process::Command::new(checker_path)
            .arg(input_path.as_ref())
            .arg(output_path.as_ref())
            .arg(ans_path.as_ref())
            .output()
            .await
            .map_err(|e| {
                let message = format!(
                    "Failed to execute checker of problem {}: {}",
                    problem_id.as_ref(),
                    e
                );
                error!("{}", message);
                crate::error::AijError::Judge(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "CHECKER_EXECUTION_FAILED".to_string(),
                    message,
                )
            })?;
        let checker_message = String::from_utf8_lossy(&output.stderr).trim().to_string();
        match output.status.code() {
            Some(0) => Ok(CheckerResult::Accepted),
            Some(7) => {
                if let Some(stripped) = checker_message.strip_prefix("points") {
                    let stripped = stripped.trim();
                    let points_str = stripped.split_whitespace().next().unwrap_or("");
                    let left_over_message = stripped[points_str.len()..].trim().to_string();
                    match points_str.parse::<f64>() {
                        Ok(points) => {
                            Ok(CheckerResult::PartiallyAccepted(points, left_over_message))
                        }
                        Err(e) => {
                            error!(
                                "Checker of problem {} returned code 7 but failed to parse points: {}, error: {}",
                                problem_id.as_ref(),
                                checker_message,
                                e
                            );
                            Ok(CheckerResult::WrongAnswer(format!(
                                "Invalid points format: {}",
                                checker_message
                            )))
                        }
                    }
                } else {
                    error!(
                        "Checker of problem {} returned code 7 but no points message: {}",
                        problem_id.as_ref(),
                        checker_message
                    );
                    Ok(CheckerResult::WrongAnswer(format!(
                        "Missing points message: {}",
                        checker_message
                    )))
                }
            }

            _ => Ok(CheckerResult::WrongAnswer(checker_message)),
        }
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_special_checker() {
        let input_path = "assets/problems/test01/data/1.in";
        let output_path = "assets/problems/test01/data/1.ans";
        let ans_path = "assets/problems/test01/data/1.ans";
        let res =
            super::special_checker("test01", input_path, output_path, ans_path, Some("checker"))
                .await
                .unwrap();
        assert!(matches!(res, super::CheckerResult::Accepted));
    }
}
