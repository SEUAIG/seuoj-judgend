//! Execute each sandbox in a fresh, single-threaded copy of the service binary.
//!
//! Linux preserves pre-exec RSS in wait4's ru_maxrss. Forking contestants from
//! the HTTP service therefore charges them for its resident heap. The helper
//! must finish exec before it forks, and report the contestant's rusage rather
//! than its own. It deliberately starts before Tokio/config/logger setup.

use serde::{Deserialize, Serialize};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Internal command-line mode, handled before the server runtime starts.
pub const HELPER_ARG: &str = "--sandbox-helper";

#[derive(Serialize, Deserialize)]
struct Request {
    config: judger::Config,
    interactor: Option<PathBuf>,
}

/// Run one sandbox using a fresh copy of the currently running server binary.
pub fn run_current(
    config: &judger::Config,
    interactor: Option<PathBuf>,
) -> Result<judger::RunResult, String> {
    // In-process API tests embed the library in a test harness, whose main
    // cannot handle our internal mode. They explicitly select the service bin.
    let executable = match std::env::var_os("AIJ_SANDBOX_EXECUTABLE") {
        Some(path) => PathBuf::from(path),
        None => std::env::current_exe().map_err(|e| e.to_string())?,
    };
    run(&executable, config, interactor)
}

/// Run one sandbox with a server executable supporting [`HELPER_ARG`].
pub fn run(
    executable: &Path,
    config: &judger::Config,
    interactor: Option<PathBuf>,
) -> Result<judger::RunResult, String> {
    let invoke = || -> Result<judger::RunResult, Box<dyn std::error::Error>> {
        // Anonymous files avoid blocking on full protocol pipes, including a
        // large interactor error. Contestant stdio is redirected by judger.
        let mut input = tempfile::tempfile()?;
        serde_json::to_writer(
            &mut input,
            &Request {
                config: config.clone(),
                interactor,
            },
        )?;
        input.seek(SeekFrom::Start(0))?;
        let mut output = tempfile::tempfile()?;
        let mut child = Command::new(executable)
            .arg(HELPER_ARG)
            .stdin(Stdio::from(input))
            .stdout(Stdio::from(output.try_clone()?))
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()?;
        let deadline = (config.max_real_time >= 0).then(|| {
            Instant::now()
                + Duration::from_millis(config.max_real_time as u64)
                + Duration::from_secs(5)
        });
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if deadline.is_none_or(|d| Instant::now() < d) => {
                    std::thread::sleep(Duration::from_millis(1));
                }
                outcome => {
                    // The helper has its own group, inherited by contestant
                    // and interactor. Kill all of them before reaping it.
                    unsafe {
                        libc::kill(-(child.id() as i32), libc::SIGKILL);
                    }
                    let _ = child.wait();
                    return Err(match outcome {
                        Err(e) => format!("Waiting for sandbox helper failed: {e}"),
                        _ => "Sandbox helper exceeded its supervision deadline".to_string(),
                    }
                    .into());
                }
            }
        };
        if !status.success() {
            return Err(format!("Sandbox helper exited with {status}").into());
        }
        output.seek(SeekFrom::Start(0))?;
        let result: Result<judger::RunResult, String> = serde_json::from_reader(output)?;
        result.map_err(Into::into)
    };
    invoke().map_err(|e| e.to_string())
}

/// Handle the internal helper mode without initializing Tokio or service state.
pub fn helper_main() -> Result<(), String> {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(|e| e.to_string())?;
    let request: Request = serde_json::from_str(&input).map_err(|e| e.to_string())?;
    let result = judger::run(&request.config, request.interactor);
    let mut output = std::io::stdout().lock();
    serde_json::to_writer(&mut output, &result).map_err(|e| e.to_string())?;
    output.flush().map_err(|e| e.to_string())
}
