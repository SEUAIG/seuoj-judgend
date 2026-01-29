mod special;
mod standard;

use crate::error::Result;
use crate::judger::CheckerType;
use crate::judger::checker::special::special_checker;
use crate::judger::checker::standard::standard_checker;
use std::path::Path;

pub(crate) async fn check(
    problem_id: impl AsRef<str>,
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    ans_path: impl AsRef<Path>,
    checker_type: CheckerType,
) -> Result<(bool, String)> {
    Ok(match checker_type {
        CheckerType::Standard => standard_checker(output_path, ans_path).await?,
        CheckerType::Special => {
            special_checker(problem_id, input_path, output_path, ans_path).await?
        }
    })
}
