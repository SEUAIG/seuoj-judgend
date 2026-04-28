use crate::config::AijConfig;
use crate::error::{AijError, Result};
use crate::fs;
use crate::judger::SupportedLanguages;
use crate::schema::ProblemConfig;
use axum::http::StatusCode;
use std::path::Path;

pub(crate) struct ExecutionPlan {
    pub(crate) judge_config: judger::Config,
    pub(crate) exec_path: String,
    pub(crate) args: Vec<String>,
}

pub(crate) enum PrepareOutcome {
    Plan(Box<ExecutionPlan>),
    CompileError(String),
}

pub(crate) async fn prepare_execution(
    problem_config: &mut ProblemConfig,
    tmp_dir: &Path,
    code: String,
    language: SupportedLanguages,
) -> Result<PrepareOutcome> {
    let source_file_extension = match language {
        SupportedLanguages::C => "c",
        SupportedLanguages::Cpp
        | SupportedLanguages::Cpp11
        | SupportedLanguages::Cpp17
        | SupportedLanguages::Cpp20 => "cpp",
        SupportedLanguages::Python3_12 => "py",
        SupportedLanguages::Nodejs22 => "js",
        SupportedLanguages::Go1_22 => "go",
        SupportedLanguages::Java17 => "java",
    };
    let mut source_file_path = tmp_dir.join("source").with_extension(source_file_extension);
    fs::write_to_file(&source_file_path, code).await?;

    let (exec_path, args, seccomp_rule) = match language {
        SupportedLanguages::C
        | SupportedLanguages::Cpp
        | SupportedLanguages::Cpp11
        | SupportedLanguages::Cpp17
        | SupportedLanguages::Cpp20 => {
            let exec_path = tmp_dir.join("executable").to_string_lossy().to_string();
            let compile_output = if language == SupportedLanguages::C {
                let gcc = AijConfig::get_binary_path("gcc").await?;
                tokio::process::Command::new(gcc)
                    .arg(&source_file_path)
                    .arg("-o")
                    .arg(&exec_path)
                    .output()
                    .await
            } else {
                let gpp = AijConfig::get_binary_path("g++").await?;
                let mut cmd = tokio::process::Command::new(gpp);
                match language {
                    SupportedLanguages::Cpp11 => {
                        cmd.arg("-std=c++11");
                    }
                    SupportedLanguages::Cpp17 => {
                        cmd.arg("-std=c++17");
                    }
                    SupportedLanguages::Cpp20 => {
                        cmd.arg("-std=c++20");
                    }
                    _ => {}
                }
                cmd.arg(&source_file_path)
                    .arg("-o")
                    .arg(&exec_path)
                    .output()
                    .await
            }
            .map_err(|e| {
                AijError::Judge(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "COMPILE_ERROR".to_string(),
                    format!("Failed to compile source code: {}", e),
                )
            })?;
            if !compile_output.status.success() {
                let stderr = String::from_utf8_lossy(&compile_output.stderr);
                return Ok(PrepareOutcome::CompileError(stderr.to_string()));
            }
            (exec_path, vec![], judger::SeccompRuleName::CCpp)
        }
        SupportedLanguages::Python3_12 => {
            problem_config.problem_info.time_limit_ms =
                match problem_config.problem_info.time_limit_ms {
                    -1 => -1,
                    m => m * 2,
                };
            let python3 = AijConfig::get_binary_path("python3").await?;
            (
                python3.to_string_lossy().to_string(),
                vec![
                    python3.to_string_lossy().to_string(),
                    source_file_path.to_string_lossy().to_string(),
                ],
                judger::SeccompRuleName::Python,
            )
        }
        SupportedLanguages::Nodejs22 => {
            problem_config.problem_info.time_limit_ms =
                match problem_config.problem_info.time_limit_ms {
                    -1 => -1,
                    m => m * 2,
                };
            let nodejs = AijConfig::get_binary_path("node").await?;
            (
                nodejs.to_string_lossy().to_string(),
                vec![
                    nodejs.to_string_lossy().to_string(),
                    source_file_path.to_string_lossy().to_string(),
                ],
                judger::SeccompRuleName::Node,
            )
        }
        SupportedLanguages::Go1_22 => {
            let exec_path = tmp_dir.join("executable").to_string_lossy().to_string();
            let go_bin = AijConfig::get_binary_path("go").await?;
            let compile_output = tokio::process::Command::new(go_bin)
                .arg("build")
                .arg("-o")
                .arg(&exec_path)
                .arg(&source_file_path)
                .output()
                .await
                .map_err(|e| {
                    AijError::Judge(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "COMPILE_ERROR".to_string(),
                        format!("Failed to compile source code: {}", e),
                    )
                })?;
            if !compile_output.status.success() {
                let stderr = String::from_utf8_lossy(&compile_output.stderr);
                return Ok(PrepareOutcome::CompileError(stderr.to_string()));
            }
            problem_config.problem_info.memory_limit_kb =
                match problem_config.problem_info.memory_limit_kb {
                    -1 => -1,
                    m => m * 2,
                };
            (exec_path, vec![], judger::SeccompRuleName::Golang)
        }
        SupportedLanguages::Java17 => {
            let new_source_file_path = source_file_path
                .with_file_name("Main")
                .with_extension("java");
            fs::rename(&source_file_path, &new_source_file_path).await?;
            source_file_path = new_source_file_path;
            problem_config.problem_info.time_limit_ms =
                match problem_config.problem_info.time_limit_ms {
                    -1 => -1,
                    m => m * 2,
                };
            let javac = AijConfig::get_binary_path("javac").await?;
            let compile_output = tokio::process::Command::new(javac)
                .arg(&source_file_path)
                .output()
                .await
                .map_err(|e| {
                    AijError::Judge(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "COMPILE_ERROR".to_string(),
                        format!("Failed to compile source code: {}", e),
                    )
                })?;
            if !compile_output.status.success() {
                let stderr = String::from_utf8_lossy(&compile_output.stderr);
                return Ok(PrepareOutcome::CompileError(stderr.to_string()));
            }
            let mut args = vec![
                "java".to_string(),
                "-cp".to_string(),
                tmp_dir.to_string_lossy().to_string(),
                "Main".to_string(),
            ];
            if problem_config.problem_info.memory_limit_kb != -1 {
                args.insert(
                    1,
                    format!("-Xmx{}m", problem_config.problem_info.memory_limit_kb / 512),
                );
            }
            let java = AijConfig::get_binary_path("java").await?;
            (
                java.to_string_lossy().to_string(),
                args,
                judger::SeccompRuleName::Java,
            )
        }
    };

    let mut judge_config = problem_config.problem_info.to_judger_config();
    judge_config.seccomp_rule_name = Some(seccomp_rule);
    Ok(PrepareOutcome::Plan(Box::new(ExecutionPlan {
        judge_config,
        exec_path,
        args,
    })))
}
