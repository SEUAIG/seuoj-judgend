use crate::config::AijConfig;
use crate::error::AijError;
use crate::error::Result;
use crate::fs;
use crate::fs::Case;
use judger::Config;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::path::Path;
use tracing::error;

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
    /// type of the problem
    pub(crate) problem_type: Option<ProblemType>,
    /// type of the checker
    pub(crate) checker_type: Option<CheckerType>,
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
    pub(crate) async fn from_pid(pid: impl AsRef<str>) -> Result<Self> {
        let content = fs::read_file_by_id_name(&pid, "info.toml").await?;
        let info: ProblemInfo = toml::from_str(&content).map_err(|e| {
            AijError::FileSystem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "FAILED_PARSE_PROBLEM_INFO".to_string(),
                format!(
                    "Failed to parse info.toml for problem {}: {}",
                    pid.as_ref(),
                    e
                ),
            )
        })?;
        Ok(info.apply_defaults())
    }

    fn apply_defaults(mut self) -> Self {
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
        if self.problem_type.is_none() {
            self.problem_type = Some(ProblemType::Standard);
        }
        if self.checker_type.is_none() {
            self.checker_type = Some(CheckerType::Standard);
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

    pub(crate) fn update_from_option(&mut self, other: &ProblemInfo) {
        if other.max_cpu_time_ms.is_some() {
            self.max_cpu_time_ms = other.max_cpu_time_ms;
        }
        if other.max_real_time_ms.is_some() {
            self.max_real_time_ms = other.max_real_time_ms;
        }
        if other.max_memory_byte.is_some() {
            self.max_memory_byte = other.max_memory_byte;
        }
        if other.max_stack_byte.is_some() {
            self.max_stack_byte = other.max_stack_byte;
        }
        if other.max_process_number.is_some() {
            self.max_process_number = other.max_process_number;
        }
        if other.max_output_size.is_some() {
            self.max_output_size = other.max_output_size;
        }
        if other.problem_type.is_some() {
            self.problem_type = other.problem_type;
        }
        if other.checker_type.is_some() {
            self.checker_type = other.checker_type;
        }
    }

    pub(crate) async fn save(&self, pid: impl AsRef<str>) -> Result<()> {
        let info_json = toml::to_string_pretty(&self).map_err(|e| {
            error!(
                "Failed to serialize problem info for problem id {}: {}",
                pid.as_ref(),
                e
            );
            AijError::Server(
                StatusCode::INTERNAL_SERVER_ERROR,
                "FAILED_SERIALIZE_PROBLEM_INFO".to_string(),
                format!("Failed to serialize problem info: {}", e),
            )
        })?;
        let info_path = fs::get_path_by_id_name(pid.as_ref(), "info.toml", false).await?;
        fs::write_to_file(&info_path, &info_json).await
    }
}

/// A collection of problem cases
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct ProblemCase {
    pub(crate) test_cases: Vec<Case>,
}

impl ProblemCase {
    pub(crate) fn is_empty(&self) -> bool {
        self.test_cases.is_empty()
    }

    pub(crate) async fn from_pid(pid: impl AsRef<str>) -> Result<Self> {
        let content = fs::read_file_by_id_name(&pid, "data/case.toml").await?;
        let mut cases: Self = toml::from_str(&content).map_err(|e| {
            AijError::FileSystem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "FAILED_PARSE_PROBLEM_CASES".to_string(),
                format!(
                    "Failed to parse case.toml for problem {}: {}",
                    pid.as_ref(),
                    e
                ),
            )
        })?;
        cases.test_cases.sort_unstable_by_key(|x| x.id);
        Ok(cases)
    }

    pub(crate) async fn save(&self, pid: impl AsRef<str>) -> Result<()> {
        let case_json = toml::to_string_pretty(&self).map_err(|e| {
            error!(
                "Failed to serialize problem cases for problem id {}: {}",
                pid.as_ref(),
                e
            );
            AijError::Server(
                StatusCode::INTERNAL_SERVER_ERROR,
                "FAILED_SERIALIZE_PROBLEM_CASES".to_string(),
                format!("Failed to serialize problem cases: {}", e),
            )
        })?;
        let case_path = fs::get_path_by_id_name(pid.as_ref(), "data/case.toml", false).await?;
        fs::write_to_file(&case_path, &case_json).await
    }
}

/// Make the file at `path` executable by adding execute permissions for user, group, and others.
pub(crate) async fn chmod_plus_x(path: impl AsRef<Path>) -> Result<()> {
    #[cfg(unix)]
    {
        let metadata = tokio::fs::metadata(&path).await.map_err(|e| {
            AijError::FileSystem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "FAILED_GET_FILE_METADATA".to_string(),
                format!(
                    "Failed to get metadata for file {}: {}",
                    path.as_ref().display(),
                    e
                ),
            )
        })?;
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = metadata.permissions();
        if permissions.mode() & 0o111 == 0 {
            permissions.set_mode(permissions.mode() | 0o111);
            return tokio::fs::set_permissions(&path, permissions)
                .await
                .map_err(|e| {
                    AijError::FileSystem(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "FAILED_SET_FILE_PERMISSIONS".to_string(),
                        format!(
                            "Failed to set execute permissions for file {}: {}",
                            path.as_ref().display(),
                            e
                        ),
                    )
                });
        }
    }
    Ok(())
}

pub(crate) async fn compile(path: impl AsRef<Path>) -> Result<()> {
    let testlib_path = &AijConfig::get().testlib_dir;
    let output = tokio::process::Command::new("g++")
        .arg(path.as_ref())
        .arg("-o")
        .arg(path.as_ref().with_extension(""))
        .arg("-O2")
        .arg("-static")
        .arg("-std=c++23")
        .arg("-I")
        .arg(testlib_path)
        .output()
        .await
        .map_err(|e| {
            let message = if e.kind() == std::io::ErrorKind::NotFound {
                "g++ not found. Please ensure g++ is installed and in the system PATH.".to_string()
            } else {
                e.to_string()
            };
            error!("Compilation error: {}", message);
            AijError::Request(
                StatusCode::INTERNAL_SERVER_ERROR,
                "COMPILATION_EXECUTION_FAILED".to_string(),
                format!(
                    "Failed to execute g++ for {}: {}",
                    path.as_ref().display(),
                    message
                ),
            )
        })?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr);
        error!(
            "Compilation failed for {}: {}",
            path.as_ref().display(),
            message
        );
        return Err(AijError::Request(
            StatusCode::BAD_REQUEST,
            "COMPILATION_FAILED".to_string(),
            format!(
                "Compilation failed for {}: {}",
                path.as_ref().display(),
                message
            ),
        ));
    }
    Ok(())
}
