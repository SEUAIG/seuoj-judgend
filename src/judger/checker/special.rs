use crate::fs::get_path_by_id_name;
use crate::judger::utils::chmod_plus_x;
use axum::http::StatusCode;
use std::path::Path;
use tracing::{error, info};

pub(crate) async fn special_checker(
    problem_id: impl AsRef<str>,
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    ans_path: impl AsRef<Path>,
) -> crate::error::Result<(bool, String)> {
    {
        info!("Using special checker for problem {}", problem_id.as_ref());
        let checker_path = get_path_by_id_name(problem_id.as_ref(), "checker", true).await?;
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
        if output.status.success() {
            Ok((true, String::new()))
        } else {
            Ok((false, checker_message))
        }
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_special_checker() {
        let input_path = "assets/problems/1/1.in";
        let output_path = "assets/problems/1/1.ans";
        let ans_path = "assets/problems/1/1.ans";
        let res = super::special_checker("1", input_path, output_path, ans_path)
            .await
            .unwrap();
        assert!(res.0);
    }
}
