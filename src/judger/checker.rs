mod special;
mod standard;

use crate::error::Result;
use crate::judger::checker::special::special_checker;
use crate::judger::checker::standard::standard_checker;
use crate::schema::CheckerType;
use std::path::Path;
use tracing::error;

pub(crate) async fn check(
    problem_id: impl AsRef<str>,
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    ans_path: impl AsRef<Path>,
    checker_type: CheckerType,
) -> Result<(bool, String)> {
    Ok(match checker_type {
        CheckerType::Standard => standard_checker(problem_id, output_path, ans_path).await?,
        CheckerType::Special => {
            special_checker(problem_id, input_path, output_path, ans_path).await?
        }
        CheckerType::Interactor => {
            let message = "Interactor problem should not be checked with checker";
            error!("{}", message);
            (false, message.to_string())
        }
    })
}
