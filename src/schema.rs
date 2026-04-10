//! Data schemas for problem configuration and metadata.
//!
//! This module defines the data structures for:
//! 1. Problem metadata and description (stored as JSON)
//! 2. Problem configuration (stored as TOML)

use crate::error::{AijError, Result};
use crate::fs;
use axum::http::StatusCode;
use judger::Config;
use serde::{Deserialize, Serialize};
use tracing::error;

/// Problem metadata and description stored as JSON.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct ProblemMetadata {
    /// Problem ID
    pub(crate) pid: String,
    /// Problem description (Markdown format)
    pub(crate) description: String,
    /// Input format description (Markdown format)
    pub(crate) input: String,
    /// Output format description (Markdown format)
    pub(crate) output: String,
    /// Problem hints (Markdown format)
    pub(crate) hint: String,
    /// Examples
    pub(crate) example: Vec<ProblemExample>,
}

impl ProblemMetadata {
    pub(crate) async fn from_pid(pid: impl AsRef<str>) -> Result<Self> {
        let meta_json = fs::read_file_by_id_name(&pid, "problem.json").await?;
        Self::from_json_str(meta_json)
    }

    pub(crate) fn from_json_str(toml_str: impl AsRef<str>) -> Result<Self> {
        let metadata: Self = serde_json::from_str(toml_str.as_ref()).map_err(|e| {
            AijError::Request(
                StatusCode::BAD_REQUEST,
                "INVALID_JSON".to_string(),
                e.to_string(),
            )
        })?;
        Ok(metadata)
    }

    pub(crate) async fn save(&self, pid: impl AsRef<str>) -> Result<()> {
        let problem_json = serde_json::to_string_pretty(&self).map_err(|e| {
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
        let meta_path = fs::get_path_by_pid_name(pid.as_ref(), "problem.json", false).await?;
        fs::write_to_file(&meta_path, &problem_json).await
    }
}

/// Problem example (input/output pair)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ProblemExample {
    /// Example input
    pub(crate) r#in: String,
    /// Example answer/output
    pub(crate) ans: String,
    /// Example description
    pub(crate) description: String,
}

/// Problem configuration stored as TOML.
/// This corresponds to the "配置数据 Schema".
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct ProblemConfig {
    /// Problem Info
    pub(crate) problem_info: ProblemInfo,
    /// Test cases
    pub(crate) testcases: Vec<TestCaseConfig>,
    /// Subtasks (optional)
    #[serde(default)]
    pub(crate) subtasks: Vec<SubtaskConfig>,
    /// Custom modules (checker/interactor)
    #[serde(default)]
    pub(crate) custom_modules: Option<CustomModules>,
}

impl ProblemConfig {
    pub(crate) async fn from_pid(pid: impl AsRef<str>) -> Result<Self> {
        let config_toml = fs::read_file_by_id_name(&pid, "info.toml").await?;
        Self::from_toml_str(config_toml)
    }

    pub(crate) fn from_toml_str(toml_str: impl AsRef<str>) -> Result<Self> {
        let problem_config: Self = toml::from_str(toml_str.as_ref()).map_err(|e| {
            AijError::Request(
                StatusCode::BAD_REQUEST,
                "INVALID_TOML".to_string(),
                e.to_string(),
            )
        })?;
        Ok(problem_config)
    }

    pub(crate) async fn save(&self, pid: impl AsRef<str>) -> Result<()> {
        let config_toml = toml::to_string_pretty(&self).map_err(|e| {
            error!(
                "Failed to serialize problem config for problem id {}: {}",
                pid.as_ref(),
                e
            );
            AijError::Server(
                StatusCode::INTERNAL_SERVER_ERROR,
                "FAILED_SERIALIZE_PROBLEM_CONFIG".to_string(),
                format!("Failed to serialize problem config: {}", e),
            )
        })?;
        let config_path = fs::get_path_by_pid_name(pid.as_ref(), "info.toml", false).await?;
        fs::write_to_file(&config_path, &config_toml).await
    }
}

/// Information about a problem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ProblemInfo {
    /// type of the problem
    pub(crate) problem_type: ProblemType,
    /// type of the checker
    pub(crate) checker_type: CheckerType,
    /// Maximum CPU time in milliseconds (-1 for unlimited).
    #[serde(default = "default_time_limit")]
    pub(crate) time_limit_ms: i32,
    /// Maximum memory in kilobytes (-1 for unlimited).
    #[serde(default = "default_memory_limit")]
    pub(crate) memory_limit_kb: i64,
}

fn default_time_limit() -> i32 {
    1000
}

fn default_memory_limit() -> i64 {
    256 * 1024
}

impl Default for ProblemInfo {
    fn default() -> Self {
        Self {
            problem_type: ProblemType::default(),
            checker_type: CheckerType::default(),
            time_limit_ms: default_time_limit(),
            memory_limit_kb: default_memory_limit(),
        }
    }
}

impl ProblemInfo {
    pub(crate) fn to_judger_config(&self) -> Config {
        Config {
            max_cpu_time: self.time_limit_ms,
            max_real_time: self.time_limit_ms * 2,
            max_memory: self.memory_limit_kb * 1024,
            max_stack: self.memory_limit_kb * 1024,
            max_process_number: 1,
            max_output_size: self.memory_limit_kb * 1024,
            ..Default::default()
        }
    }
}

/// Test case configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TestCaseConfig {
    /// Unique identifier for the test case
    pub(crate) id: i32,
    /// Path to input file (relative to problem directory)
    pub(crate) in_path: String,
    /// Path to output/answer file (relative to problem directory)
    pub(crate) ans_path: String,
    /// Weight for scoring (default: 1.0)
    #[serde(default = "default_weight")]
    pub(crate) weight: f64,
    /// Override time limit (milliseconds), null to use global
    #[serde(default)]
    pub(crate) time_limit_ms: Option<i32>,
    /// Override memory limit (kilobytes), null to use global
    #[serde(default)]
    pub(crate) memory_limit_kb: Option<i64>,
}

fn default_weight() -> f64 {
    1.0
}

/// Subtask configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SubtaskConfig {
    /// Unique identifier for the subtask
    pub(crate) id: i32,
    /// List of test case IDs belonging to this subtask
    pub(crate) cases: Vec<i32>,
    /// List of subtask IDs that must be passed before evaluating this one
    #[serde(default)]
    pub(crate) pre_subtasks: Vec<i32>,
    /// Total score awarded if this subtask is passed completely
    pub(crate) score: i32,
    /// Scoring type: "min", "sum", etc.
    #[serde(default = "default_subtask_type")]
    pub(crate) r#type: String,
}

fn default_subtask_type() -> String {
    "min".to_string()
}

/// Custom modules (checker/interactor)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct CustomModules {
    /// Path to the SPJ source/binary (relative to problem directory)
    #[serde(default)]
    pub(crate) checker_path: Option<String>,
    /// Path to the Interactor source/binary (relative to problem directory)
    #[serde(default)]
    pub(crate) interactor_path: Option<String>,
}

/// Type of the problem
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) enum ProblemType {
    /// Standard IO problem
    #[default]
    #[serde(alias = "standard", alias = "STANDARD")]
    Standard,
    /// Interactive problem
    #[serde(alias = "interactive", alias = "INTERACTIVE")]
    Interactive,
    /// SPJ problem
    #[serde(alias = "special", alias = "SPECIAL")]
    Special,
}

/// Type of the checker
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) enum CheckerType {
    /// Standard
    #[default]
    #[serde(alias = "standard", alias = "STANDARD")]
    Standard,
    /// Special judge (if [ProblemType] is SPJ, this must be SPJ)
    #[serde(alias = "special", alias = "SPECIAL")]
    Special,
    /// Interactive checker (if [ProblemType] is Interactive, this must be Interactive)
    #[serde(alias = "interactor", alias = "INTERACTOR")]
    Interactor,
}
