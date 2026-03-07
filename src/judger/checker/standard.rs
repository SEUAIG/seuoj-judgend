use crate::error::Result;
use crate::fs::get_text_by_path;
use crate::judger::checker::CheckerResult;
use std::path::Path;
use tracing::info;

pub(crate) async fn standard_checker(
    problem_id: impl AsRef<str>,
    output_path: impl AsRef<Path>,
    ans_path: impl AsRef<Path>,
) -> Result<CheckerResult> {
    {
        info!("Using standard checker for problem {}", problem_id.as_ref());
        let out = get_text_by_path(output_path, None).await?;
        let ans = get_text_by_path(ans_path, None).await?;
        Ok(standard_checker_str(&out, &ans).await)
    }
}

pub(crate) async fn standard_checker_str(output: &str, answer: &str) -> CheckerResult {
    let normalize = |mut text: &str| {
        while text.ends_with('\n') || text.ends_with('\r') {
            text = &text[..text.len() - 1];
        }
        text.lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
    };
    let output_normalized = normalize(output);
    let answer_normalized = normalize(answer);
    let output_lines: Vec<&str> = output_normalized.lines().collect();
    let answer_lines: Vec<&str> = answer_normalized.lines().collect();

    if output_lines == answer_lines {
        return CheckerResult::Accepted;
    }

    // Find the first line that differs
    let max_lines = output_lines.len().max(answer_lines.len());
    for i in 0..max_lines {
        let out_line = output_lines.get(i).unwrap_or(&"");
        let ans_line = answer_lines.get(i).unwrap_or(&"");
        if out_line != ans_line {
            return CheckerResult::WrongAnswer(format!(
                "In line {}:\nExpected: '{}'\nFound:    '{}'",
                i + 1,
                ans_line,
                out_line
            ));
        }
    }

    CheckerResult::WrongAnswer(format!(
        "Line number mismatch: Expected {} row, actual {} row",
        answer_lines.len(),
        output_lines.len()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_standard_checker_str() {
        let output = "Hello, World!  \nThis is a test.\n\n";
        let answer = "Hello, World!\nThis is a test.";
        assert!(matches!(
            standard_checker_str(output, answer).await,
            CheckerResult::Accepted
        ));

        let output = "Hello, World!\nThis is a test.\nExtra line.";
        let answer = "Hello, World!\nThis is a test.";
        assert!(matches!(
            standard_checker_str(output, answer).await,
            CheckerResult::WrongAnswer(_)
        ));
    }
}
