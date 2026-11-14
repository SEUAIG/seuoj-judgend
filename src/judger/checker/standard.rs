// Default line comparison is performed on bytes by a bundled testlib checker.
use crate::config::AijConfig;
use crate::error::{AijError, Result};
use crate::judger::checker::CheckerResult;
use axum::http::StatusCode;
use std::path::Path;
use tracing::info;

pub(crate) async fn standard_checker(
    problem_id: impl AsRef<str>,
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    ans_path: impl AsRef<Path>,
) -> Result<CheckerResult> {
    info!(
        "Using testlib default checker for problem {}",
        problem_id.as_ref()
    );
    let checker = AijConfig::get().testlib_dir.join("default-checker");
    let result = tokio::process::Command::new(&checker)
        .arg(input_path.as_ref())
        .arg(output_path.as_ref())
        .arg(ans_path.as_ref())
        .kill_on_drop(true)
        .output()
        .await
        .map_err(|e| {
            AijError::Judge(
                StatusCode::INTERNAL_SERVER_ERROR,
                "CHECKER_EXECUTION_FAILED".to_string(),
                format!(
                    "Failed to execute default checker {}: {}",
                    checker.display(),
                    e
                ),
            )
        })?;
    // Only diagnostics are decoded lossily. The checker compares original bytes.
    let message = String::from_utf8_lossy(&result.stderr).trim().to_string();
    match result.status.code() {
        Some(0) => Ok(CheckerResult::Accepted),
        Some(1 | 2 | 8) => Ok(CheckerResult::WrongAnswer(message)),
        code => Err(AijError::Judge(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CHECKER_FAILED".to_string(),
            format!("Default checker exited with {:?}: {}", code, message),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn default_checker_compares_bytes_and_preserves_formatting() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("input");
        let output = dir.path().join("output");
        let answer = dir.path().join("answer");
        tokio::fs::write(&input, b"").await.unwrap();
        let cases: &[(&[u8], &[u8], bool)] = &[
            (b"a10\n", b"a10\n", true),
            (b"a10 \t\r\n\n", b"a10", true),
            (b"a10", b"a10\n\n", true),
            (b"\n", b"", true),
            (b"1 2\n", b"1  2\n", false),
            (b"1\n2\n", b"1 2\n", false),
            (b" 1\n", b"1\n", false),
            (b"01\n", b"1\n", false),
            (b"yes\n", b"Yes\n", false),
            (b"a10\nextra\n", b"a10\n", false),
            (b"a10\n", b"a10\nextra\n", false),
            (b"a10\n\nb20", b"a10\nb20", false),
            (b"a:\n", b"a10\n", false),
            (b"a\xff\n", b"a10\n", false),
            (b"a\xe4\xb8", b"a10\n", false),
            (b"a\0\n", b"a10\n", false),
            // Lossy decoding must never turn invalid bytes into a matching answer.
            (b"a\xff\n", "a\u{fffd}\n".as_bytes(), false),
            ("你好\n".as_bytes(), "你好\n".as_bytes(), true),
            ("a10\u{3000}\n".as_bytes(), b"a10\n", false),
            (b"a\rb\n", b"ab\n", false),
            (b"\xef\xbb\xbfa10\n", b"a10\n", false),
        ];
        for (out, ans, accepted) in cases {
            tokio::fs::write(&output, out).await.unwrap();
            tokio::fs::write(&answer, ans).await.unwrap();
            let result = standard_checker("test", &input, &output, &answer)
                .await
                .unwrap();
            assert_eq!(
                matches!(result, CheckerResult::Accepted),
                *accepted,
                "output={out:?}, answer={ans:?}"
            );
            if !accepted {
                assert!(matches!(result, CheckerResult::WrongAnswer(_)));
            }
        }
        // A genuine missing answer file must remain an infrastructure error.
        tokio::fs::remove_file(&answer).await.unwrap();
        assert!(
            standard_checker("test", &input, &output, &answer)
                .await
                .is_err()
        );
    }
}
