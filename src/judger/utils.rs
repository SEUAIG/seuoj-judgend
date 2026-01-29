use crate::error::AijError;
use judger::Config;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::path::Path;

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
    pub(crate) max_output_size: Option<i64>,
    /// Number of test cases
    pub(crate) test_case_number: Option<i32>,
    /// type of the problem
    pub(crate) problem_type: ProblemType,
    /// type of the checker
    pub(crate) checker_type: CheckerType,
}

/// Type of the problem
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ProblemType {
    /// Standard IO problem
    Standard,
    /// Interactive problem
    Interactive,
}

/// Type of the checker
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum CheckerType {
    /// Standard
    Standard,
    /// Special judge
    Special,
}

impl Display for ProblemType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProblemType::Standard => write!(f, "Standard"),
            ProblemType::Interactive => write!(f, "Interactive"),
        }
    }
}

impl ProblemInfo {
    pub(crate) async fn from_pid(pid: &str) -> crate::error::Result<Self> {
        let content = crate::fs::read_file_by_id_name(pid, "info.json").await?;
        let info: ProblemInfo = serde_json::from_str(&content).map_err(|e| {
            AijError::FileSystem(format!(
                "Failed to parse info.json for problem {}: {}",
                pid, e
            ))
        })?;
        Ok(info)
    }

    pub(crate) fn apply_defaults(mut self) -> Self {
        if self.max_cpu_time_ms.is_none() {
            self.max_cpu_time_ms = Some(1000);
        }
        if self.max_real_time_ms.is_none() {
            self.max_real_time_ms = Some(2000);
        }
        if self.max_memory_byte.is_none() {
            self.max_memory_byte = Some(128 * 1024 * 1024);
        }
        if self.max_stack_byte.is_none() {
            self.max_stack_byte = Some(32 * 1024 * 1024);
        }
        if self.max_process_number.is_none() {
            self.max_process_number = Some(1);
        }
        if self.max_output_size.is_none() {
            self.max_output_size = Some(1000000);
        }
        self
    }

    pub(crate) fn to_judger_config(&self) -> Config {
        let mut config = Config::default();
        if let Some(cpu_time) = self.max_cpu_time_ms {
            config.max_cpu_time = cpu_time;
        }
        if let Some(real_time) = self.max_real_time_ms {
            config.max_real_time = real_time;
        }
        if let Some(memory) = self.max_memory_byte {
            config.max_memory = memory;
        }
        if let Some(stack) = self.max_stack_byte {
            config.max_stack = stack;
        }
        if let Some(process_number) = self.max_process_number {
            config.max_process_number = process_number;
        }
        if let Some(output_size) = self.max_output_size {
            config.max_output_size = output_size;
        }
        config
    }
}

/// Make the file at `path` executable by adding execute permissions for user, group, and others.
pub(crate) async fn chmod_plus_x(path: impl AsRef<Path>) -> tokio::io::Result<()> {
    #[cfg(unix)]
    {
        let metadata = tokio::fs::metadata(&path).await?;
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = metadata.permissions();
        if permissions.mode() & 0o111 == 0 {
            permissions.set_mode(permissions.mode() | 0o111);
            return tokio::fs::set_permissions(&path, permissions).await;
        }
    }
    Ok(())
}
