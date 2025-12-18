use crate::error::Result;
use serde::{Deserialize, Serialize};

/// Supported programming languages for the judger system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum SupportedLanguages {
    C,
    Cpp,
    Cpp11,
    Cpp17,
    Cpp20,
    Python3_12,
    Nodejs22,
    Go1_22,
    Java17,
}

/// Information about a problem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ProblemInfo {
    /// Maximum CPU time in milliseconds (-1 for unlimited).
    pub(crate) max_cpu_time_ms: Option<i32>,
    /// Maximum real time in milliseconds (-1 for unlimited).
    pub(crate) max_real_time_ms: Option<i32>,
    /// Maximum memory in bytes (-1 for unlimited).
    pub(crate) max_memory_byte: Option<i64>,
    /// Maximum stack size in bytes.
    pub(crate) max_stack_byte: Option<i64>,
    /// Maximum number of processes (-1 for unlimited).
    pub(crate) max_process_number: Option<i32>,
    /// Maximum output size in bytes (-1 for unlimited).
    pub(crate) max_output_size_byte: Option<i64>,
    /// Number of test cases
    pub(crate) test_case_number: i32,
    /// type of the problem
    pub(crate) problem_type: ProbemType,
}

/// Type of the problem
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ProbemType {
    /// Standard IO problem
    Standard,
    /// Interactive problem
    Interactive,
}

impl ProblemInfo {
    pub(crate) async fn from_pid(pid: &str) -> Result<Self> {
        let content = crate::fs::read_file_by_id_name(pid, "info.json").await?;
        let info: ProblemInfo = serde_json::from_str(&content).map_err(|e| {
            crate::error::AijError::FileSystemError(format!(
                "Failed to parse info.json for problem {}: {}",
                pid, e
            ))
        })?;
        Ok(info)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::FileSystem;

    #[tokio::test]
    async fn test_problem_info_from_pid() {
        FileSystem::init("/tmp/aij/problems").unwrap();
        let pid = "1";
        let info = ProblemInfo::from_pid(pid).await;
        assert!(info.is_ok());
        let info = info.unwrap();
        assert_eq!(info.test_case_number, 1);
        assert_eq!(info.problem_type, ProbemType::Standard);
    }
}