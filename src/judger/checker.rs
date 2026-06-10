// Checker dispatch layer for standard and special checker implementations.
mod special;
mod standard;

use crate::error::Result;
use crate::judger::checker::special::special_checker;
use crate::judger::checker::standard::standard_checker;
use crate::schema::CheckerType;
use std::path::Path;
use tracing::error;

pub(crate) enum CheckerResult {
    Accepted,
    PartiallyAccepted(f64, String),
    WrongAnswer(String),
}

pub(crate) async fn check(
    problem_id: impl AsRef<str>,
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    ans_path: impl AsRef<Path>,
    checker_type: CheckerType,
    checker_path: Option<&str>,
    time_ms: u64,
) -> Result<CheckerResult> {
    Ok(match checker_type {
        CheckerType::Standard => standard_checker(problem_id, output_path, ans_path).await?,
        CheckerType::Special => {
            special_checker(
                problem_id,
                input_path,
                output_path,
                ans_path,
                checker_path,
                time_ms,
            )
            .await?
        }
        CheckerType::Interactor => {
            let message = "Interactor problem should not be checked with checker";
            error!("{}", message);
            return Err(crate::error::AijError::Judge(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "CHECKER_TYPE_INVALID".to_string(),
                message.to_string(),
            ));
        }
    })
}
