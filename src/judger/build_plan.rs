// Build execution plan from submission source and selected language.
use crate::config::AijConfig;
use crate::error::{AijError, Result};
use crate::fs;
use crate::judger::SupportedLanguages;
use crate::judger::judger_helpers::{CompileResult, run_compile_with_limit};
use crate::schema::ProblemConfig;
use axum::http::StatusCode;
use std::path::{Path, PathBuf};
use tokio::process::Command;

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
    pid: &str,
    problem_config: &mut ProblemConfig,
    tmp_dir: &Path,
    code: String,
    language: SupportedLanguages,
) -> Result<PrepareOutcome> {
    let grader_paths = problem_config
        .custom_modules
        .as_ref()
        .map(|m| m.grader_paths.clone())
        .unwrap_or_default();
    // Graders are linked into the submission, which only the compiled C/C++
    // pipeline knows how to do. Reject other languages through the compile-error
    // channel so the contestant sees why instead of a bare system error.
    if !grader_paths.is_empty()
        && !matches!(
            language,
            SupportedLanguages::C | SupportedLanguages::Cpp | SupportedLanguages::Cpp20
        )
    {
        return Ok(PrepareOutcome::CompileError(format!(
            "本题通过 grader 编译，目前仅支持 C / C++，当前提交语言：{language}"
        )));
    }

    let source_file_extension = match language {
        SupportedLanguages::C => "c",
        SupportedLanguages::Cpp | SupportedLanguages::Cpp20 => "cpp",
        SupportedLanguages::Python => "py",
        SupportedLanguages::Nodejs => "js",
        SupportedLanguages::Go => "go",
        SupportedLanguages::Java => "java",
        SupportedLanguages::Rust => "rs",
    };
    let mut source_file_path = tmp_dir.join("source").with_extension(source_file_extension);
    fs::write_to_file(&source_file_path, code).await?;

    let (exec_path, args, seccomp_rule) = match language {
        SupportedLanguages::C | SupportedLanguages::Cpp | SupportedLanguages::Cpp20 => {
            let exec_path = tmp_dir.join("executable").to_string_lossy().to_string();
            // Problem-provided grader sources are compiled together with the
            // submission, so the contestant only implements the required
            // functions. Headers just land in tmp_dir and are found via `-I`.
            let extra_sources =
                stage_graders(pid, tmp_dir, &grader_paths, &source_file_path).await?;
            let result = if language == SupportedLanguages::C {
                let gcc = AijConfig::get_binary_path("gcc").await?;
                let mut cmd = Command::new(gcc);
                cmd.arg("-O2").arg("-DONLINE_JUDGE");
                if !grader_paths.is_empty() {
                    cmd.arg("-I").arg(tmp_dir);
                }
                cmd.arg(&source_file_path);
                cmd.args(&extra_sources);
                cmd.arg("-o").arg(&exec_path);
                run_compile_with_limit(cmd).await
            } else {
                let gpp = AijConfig::get_binary_path("g++").await?;
                let mut cmd = Command::new(gpp);
                cmd.arg("-O2")
                    .arg("-DONLINE_JUDGE")
                    .arg("-ftemplate-depth=1024");
                if language == SupportedLanguages::Cpp20 {
                    cmd.arg("-std=c++20");
                }
                if !grader_paths.is_empty() {
                    cmd.arg("-I").arg(tmp_dir);
                }
                cmd.arg(&source_file_path);
                cmd.args(&extra_sources);
                cmd.arg("-o").arg(&exec_path);
                run_compile_with_limit(cmd).await
            }?;
            match result {
                CompileResult::CompileError(e) => return Ok(PrepareOutcome::CompileError(e)),
                CompileResult::Success => {}
            }
            (exec_path, vec![], judger::SeccompRuleName::CCpp)
        }
        SupportedLanguages::Rust => {
            let exec_path = tmp_dir.join("executable").to_string_lossy().to_string();
            let rustc = AijConfig::get_binary_path("rustc").await?;
            let mut cmd = Command::new(rustc);
            // Dependencies are built with the same pinned compiler as submissions.
            // No Cargo invocation or dependency downloads occur during judging.
            cmd.args(["--edition=2021", "-O", "-C", "codegen-units=1"])
                .arg("--cfg")
                .arg("ONLINE_JUDGE")
                .arg("--extern")
                .arg("proconio=/opt/rust-libs/libproconio.rlib")
                .arg("-L")
                .arg("dependency=/opt/rust-libs/deps")
                .arg(&source_file_path)
                .arg("-o")
                .arg(&exec_path)
                .kill_on_drop(true);
            match run_compile_with_limit(cmd).await? {
                CompileResult::CompileError(e) => return Ok(PrepareOutcome::CompileError(e)),
                CompileResult::Success => {}
            }
            // Rust startup uses signal/alternate-stack syscalls absent from CCpp.
            // General permits those while blocking threads, sockets and file writes.
            (exec_path, vec![], judger::SeccompRuleName::General)
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
                // Avoid compiler workers reserving memory for every host core.
                // The existing address-space and time limits still apply.
                .arg("-p=1")
                .arg("-o")
                .arg(&exec_path)
                .arg(&source_file_path)
                .env("GOMAXPROCS", "2")
                .env("GOGC", "50")
                .env(
                    "GOMEMLIMIT",
                    format!(
                        "{}KiB",
                        (AijConfig::get().compile_memory_limit_kb / 4).max(1024)
                    ),
                );
            match run_compile_with_limit(cmd).await? {
                CompileResult::CompileError(e) => return Ok(PrepareOutcome::CompileError(e)),
                CompileResult::Success => {}
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
            // Bound JVM reservations so javac fits the existing compile limit.
            cmd.args([
                "-J-Xms16m",
                "-J-Xmx256m",
                "-J-XX:CompressedClassSpaceSize=64m",
                "-J-XX:ReservedCodeCacheSize=64m",
                "-J-XX:MaxMetaspaceSize=128m",
                "-J-XX:ActiveProcessorCount=2",
                "-J-XX:+UseSerialGC",
            ])
            .arg(&source_file_path);
            match run_compile_with_limit(cmd).await? {
                CompileResult::CompileError(e) => return Ok(PrepareOutcome::CompileError(e)),
                CompileResult::Success => {}
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


/// Copy the problem's grader files into the submission tmp dir, returning the
/// ones that are translation units and therefore have to be handed to the
/// compiler alongside the submitted source.
///
/// Headers need no further handling: `prepare_execution` puts `tmp_dir` on
/// the include path, so `#include "foo.h"` from the submission resolves to
/// the staged copy.
async fn stage_graders(
    pid: &str,
    tmp_dir: &Path,
    grader_paths: &[String],
    source_file_path: &Path,
) -> Result<Vec<PathBuf>> {
    let mut extra_sources = Vec::new();
    for rel_path in grader_paths {
        let src = fs::get_path_by_pid_name(pid, format!("data/{rel_path}"), true).await?;
        let file_name = Path::new(rel_path).file_name().ok_or_else(|| {
            AijError::Judge(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INVALID_GRADER_PATH".to_string(),
                format!("Grader path has no file name: {rel_path}"),
            )
        })?;
        let dst = tmp_dir.join(file_name);
        if dst == source_file_path {
            return Err(AijError::Judge(
                StatusCode::INTERNAL_SERVER_ERROR,
                "GRADER_NAME_CONFLICT".to_string(),
                format!("Grader file {rel_path} would overwrite the submitted source"),
            ));
        }
        fs::copy(&src, &dst).await?;
        if matches!(
            dst.extension().and_then(|e| e.to_str()),
            Some("c" | "cpp" | "cc" | "cxx")
        ) {
            extra_sources.push(dst);
        }
    }
    Ok(extra_sources)
}
