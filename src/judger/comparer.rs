use std::path::Path;

pub(crate) async fn standard_comparer(
    output_path: impl AsRef<Path>,
    ans_path: impl AsRef<Path>,
) -> Result<bool, crate::error::AijError> {
    {
        let out = tokio::fs::read_to_string(output_path.as_ref())
            .await
            .map_err(|e| {
                crate::error::AijError::FileSystem(format!(
                    "Failed to read output file {}: {}",
                    output_path.as_ref().to_string_lossy(),
                    e
                ))
            })?;
        let ans = tokio::fs::read_to_string(ans_path.as_ref())
            .await
            .map_err(|e| {
                crate::error::AijError::FileSystem(format!(
                    "Failed to read answer file {}: {}",
                    ans_path.as_ref().to_string_lossy(),
                    e
                ))
            })?;
        Ok(standard_comparer_str(&out, &ans).await)
    }
}

pub(crate) async fn standard_comparer_str(output: &str, answer: &str) -> bool {
    let normalize = |mut text: &str| {
        while text.ends_with('\n') || text.ends_with('\r') {
            text = &text[..text.len() - 1];
        }
        text.lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
    };
    normalize(output) == normalize(answer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_standard_comparer_str() {
        let output = "Hello, World!  \nThis is a test.\n\n";
        let answer = "Hello, World!\nThis is a test.";
        assert!(standard_comparer_str(output, answer).await);

        let output = "Hello, World!\nThis is a test.\nExtra line.";
        let answer = "Hello, World!\nThis is a test.";
        assert!(!standard_comparer_str(output, answer).await);
    }
}
