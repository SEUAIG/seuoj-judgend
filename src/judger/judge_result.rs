// Shared result models returned by judge flows.
use crate::schema::SubtaskConfig;
use serde::{Deserialize, Serialize};

/// Type of the judging result for a single test case
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub enum JudgeResultType {
    /// Accepted: the output matches the expected answer
    #[default]
    Accepted,
    /// Skipped: the test case was skipped due to a previous error in the same subtask
    Skipped,
    /// Partially Accepted: the output is partially correct, with a score between 0 and 100
    PartiallyAccepted,
    /// Wrong Answer: the output does not match the expected answer
    WrongAnswer,
    /// Time Limit Exceeded: the program exceeded the time limit for execution
    TimeLimitExceeded,
    /// Memory Limit Exceeded: the program exceeded the memory limit for execution
    MemoryLimitExceeded,
    /// Runtime Error: the program crashed or encountered a runtime error during execution
    RuntimeError,
    /// System Error: an error occurred in the judger system itself, not related to the user's code
    SystemError,
}

/// Result of once judging
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct JudgeResultItem {
    /// count of the test case
    pub(crate) id: i32,
    /// real time used in milliseconds
    pub(crate) time: i32,
    /// memory used in bytes
    pub(crate) mem: i64,
    /// output of system
    pub(crate) sys: String,
    /// input of test case
    pub(crate) r#in: String,
    /// expected answer of test case
    pub(crate) ans: String,
    /// output of user code
    pub(crate) out: String,
    /// type of the result
    pub(crate) r#type: JudgeResultType,
    /// score of the test case
    pub(crate) score: i32,
}

/// Result of the judging process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum JudgeResult {
    CompileError {
        detail: String,
    },
    CodeTooLong {
        detail: String,
    },
    MaybeError {
        results: Vec<JudgeResultItem>,
        subtask_configs: Vec<SubtaskConfig>,
    },
}
