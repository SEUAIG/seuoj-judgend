// Build execution plan from submission source and selected language.
use crate::config::AijConfig;
use crate::error::{AijError, Result};
use crate::fs;
use crate::judger::SupportedLanguages;
use crate::schema::ProblemConfig;
use axum::http::StatusCode;
use std::path::Path;
use std::process::Output;
use std::time::Duration;
use tokio::process::Command;
use tracing::warn;

pub(crate) struct ExecutionPlan {
    pub(crate) judge_config: judger::Config,
    pub(crate) exec_path: String,
    pub(crate) args: Vec<String>,
}

pub(crate) enum PrepareOutcome {
    Plan(Box<ExecutionPlan>),
    CompileError(String),
    Success(()),
}

async fn run_compile_with_limit(mut cmd: Command) -> Result<PrepareOutcome> {
    let config = AijConfig::get();
    let time_limit = Duration::from_millis(config.compile_time_limit_ms);

    unsafe {
        cmd.pre_exec(move || {
            let limit = config.compile_memory_limit_kb * 1024;
            let rlimit = libc::rlimit {
                rlim_cur: limit,
                rlim_max: limit,
            };
            libc::setrlimit(libc::RLIMIT_AS, &rlimit);
            Ok(())
        });
    }

    match tokio::time::timeout(time_limit, cmd.output()).await {
        Ok(result) => {
            let output = result.map_err(|e| {
                AijError::Judge(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "COMPILE_ERROR".to_string(),
                    format!("Failed to compile source code: {}", e),
                )
            })?;
            if let Some(err) = check_compile_output(&output) {
                return Ok(err);
            }
            Ok(PrepareOutcome::Success(()))
        }
        Err(_) => {
            warn!(
                "Compilation timed out after {}ms",
                config.compile_time_limit_ms
            );
            Ok(PrepareOutcome::CompileError(format!(
                "Compilation timed out after {}ms",
                config.compile_time_limit_ms
            )))
        }
    }
}

fn check_compile_output(output: &Output) -> Option<PrepareOutcome> {
    if output.status.success() {
        return None;
    }
    let max_error_len = AijConfig::get().compile_error_truncate_length;
    let stderr_truncated = if output.stderr.len() > max_error_len {
        &output.stderr[0..max_error_len]
    } else {
        &output.stderr
    };
    let mut stderr = String::from_utf8_lossy(stderr_truncated).to_string();
    if output.stderr.len() > max_error_len {
        stderr = format!(
            "\n\n[System Notice]: Error output truncated to {} bytes to avoid overwhelming the server.",
            max_error_len
        ) + &stderr;
    }
    Some(PrepareOutcome::CompileError(stderr))
}

pub(crate) async fn prepare_execution(
    problem_config: &mut ProblemConfig,
    tmp_dir: &Path,
    code: String,
    language: SupportedLanguages,
) -> Result<PrepareOutcome> {
    let source_file_extension = match language {
        SupportedLanguages::C => "c",
        SupportedLanguages::Cpp | SupportedLanguages::Cpp20 => "cpp",
        SupportedLanguages::Python => "py",
        SupportedLanguages::Nodejs => "js",
        SupportedLanguages::Go => "go",
        SupportedLanguages::Java => "java",
    };
    let mut source_file_path = tmp_dir.join("source").with_extension(source_file_extension);
    fs::write_to_file(&source_file_path, code).await?;

    let (exec_path, args, seccomp_rule) = match language {
        SupportedLanguages::C | SupportedLanguages::Cpp | SupportedLanguages::Cpp20 => {
            let exec_path = tmp_dir.join("executable").to_string_lossy().to_string();
            let result = if language == SupportedLanguages::C {
                let gcc = AijConfig::get_binary_path("gcc").await?;
                let mut cmd = Command::new(gcc);
                cmd.arg("-O2")
                    .arg(&source_file_path)
                    .arg("-o")
                    .arg(&exec_path);
                run_compile_with_limit(cmd).await
            } else {
                let gpp = AijConfig::get_binary_path("g++").await?;
                let mut cmd = Command::new(gpp);
                cmd.arg("-O2").arg("-ftemplate-depth=1024");
                if language == SupportedLanguages::Cpp20 {
                    cmd.arg("-std=c++20");
                }
                cmd.arg(&source_file_path).arg("-o").arg(&exec_path);
                run_compile_with_limit(cmd).await
            }?;
            match result {
                PrepareOutcome::CompileError(e) => return Ok(PrepareOutcome::CompileError(e)),
                PrepareOutcome::Success(_) => {}
                PrepareOutcome::Plan(_) => unreachable!(),
            }
            (exec_path, vec![], judger::SeccompRuleName::CCpp)
        }
        SupportedLanguages::Python => {
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
        SupportedLanguages::Nodejs => {
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
        SupportedLanguages::Go => {
            let exec_path = tmp_dir.join("executable").to_string_lossy().to_string();
            let go_bin = AijConfig::get_binary_path("go").await?;
            let mut cmd = Command::new(go_bin);
            cmd.arg("build")
                .arg("-o")
                .arg(&exec_path)
                .arg(&source_file_path);
            match run_compile_with_limit(cmd).await? {
                PrepareOutcome::CompileError(e) => return Ok(PrepareOutcome::CompileError(e)),
                PrepareOutcome::Success(_) => {}
                PrepareOutcome::Plan(_) => unreachable!(),
            }
            problem_config.problem_info.memory_limit_kb =
                match problem_config.problem_info.memory_limit_kb {
                    -1 => -1,
                    m => m * 2,
                };
            (exec_path, vec![], judger::SeccompRuleName::Golang)
        }
        SupportedLanguages::Java => {
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
            let mut cmd = Command::new(javac);
            cmd.arg(&source_file_path);
            match run_compile_with_limit(cmd).await? {
                PrepareOutcome::CompileError(e) => return Ok(PrepareOutcome::CompileError(e)),
                PrepareOutcome::Success(_) => {}
                PrepareOutcome::Plan(_) => unreachable!(),
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
