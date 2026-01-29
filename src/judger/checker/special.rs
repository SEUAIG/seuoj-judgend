use crate::fs::get_path_by_id_name;
use std::path::Path;
use tracing::error;

pub(crate) async fn special_checker(
    problem_id: impl AsRef<str>,
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    ans_path: impl AsRef<Path>,
) -> crate::error::Result<(bool, String)> {
    {
        let checker_path = get_path_by_id_name(problem_id.as_ref(), "checker").await?;
        let output = tokio::process::Command::new(checker_path)
            .arg(input_path.as_ref())
            .arg(output_path.as_ref())
            .arg(ans_path.as_ref())
            .output()
            .await
            .map_err(|e| {
                error!(
                    "Failed to execute checker of problem {}: {}",
                    problem_id.as_ref(),
                    e
                );
                crate::error::AijError::Judge(format!("Failed to execute checker: {}", e))
            })?;
        let checker_message = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if output.status.success() {
            Ok((true, String::new()))
        } else {
            Ok((false, checker_message))
        }
    }
}
