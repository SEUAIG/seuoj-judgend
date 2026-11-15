use crate::{Config, ErrorCode, LogLevel, Logger, child_process};
use nix::fcntl::OFlag;
use nix::libc;
use nix::sys::signal::Signal;
use nix::unistd::{ForkResult, Uid, fork};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::os::fd::AsRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Result of the program
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct RunResult {
    /// CPU time used in milliseconds.
    pub cpu_time: i32,
    /// Real time used in milliseconds.
    pub real_time: i32,
    /// Memory used in bytes.
    pub memory: i64,
    /// Signal that terminated the process.
    pub signal: i32,
    /// Exit code of the process.
    pub exit_code: i32,
    /// Error code if any error occurred during execution.
    pub result: ErrorCode,
}

/// Runs the judger with the given configuration.
/// Returns a `RunResult` containing the execution results.
/// # Arguments
/// * `config` - A reference to the `Config` struct containing the judger configuration
/// * `interactor` - An optional `PathBuf` for the interactor program
/// # Returns
/// * `Result<RunResult, String>` - On success, returns `Ok(RunResult)`. On failure, returns `Err(String)` with an error message.
pub fn run(config: &Config, interactor: Option<PathBuf>) -> Result<RunResult, String> {
    let mut logger = Logger::new(&config.log_path)
        .map_err(|e| format!("Failed to open log file {}: {:?}", &config.log_path, e))?;
    let mut result = RunResult::default();

    let uid = Uid::current();
    if !uid.is_root() {
        result.result = ErrorCode::RootRequired;
        logger
            .write(
                LogLevel::Fatal,
                file!(),
                line!(),
                format_args!("Error: Root privileges are required to run the judger."),
            )
            .map_err(|e| format!("Failed to write to log file: {:?}", e))?;
        return Ok(result);
    }

    if !config.check() {
        result.result = ErrorCode::InvalidConfig;
        logger
            .write(
                LogLevel::Fatal,
                file!(),
                line!(),
                format_args!("Error: Invalid configuration provided."),
            )
            .map_err(|e| format!("Failed to write to log file: {:?}", e))?;
        return Ok(result);
    }

    // Keep diagnostics out of a pipe: a verbose interactor must not block
    // waiting for us to read stderr while we are waiting for the contestant.
    let inter_error_path = format!("{}.interactor", config.error_path);
    let inter_error = if interactor.is_some() {
        std::fs::File::create(&config.output_path)
            .map_err(|e| format!("Failed to create interaction transcript: {e}"))?;
        Some(
            std::fs::File::create(&inter_error_path)
                .map_err(|e| format!("Failed to create interactor diagnostics: {e}"))?,
        )
    } else {
        None
    };
    let start_time = Instant::now();
    let (user_stdin, inter_stdout) = nix::unistd::pipe2(OFlag::O_CLOEXEC)
        .map_err(|e| format!("Failed to create pipe for interactor: {:?}", e))?;
    let (inter_stdin, user_stdout) = nix::unistd::pipe2(OFlag::O_CLOEXEC)
        .map_err(|e| format!("Failed to create pipe for user program: {:?}", e))?;
    match unsafe { fork() } {
        Ok(ForkResult::Parent { child }) => {
            let inter_child = interactor
                .map(|path| {
                    std::process::Command::new(path)
                        .args(vec![&config.input_path, &config.output_path])
                        .stdin(Stdio::from(inter_stdin))
                        .stdout(Stdio::from(inter_stdout))
                        .stderr(Stdio::from(inter_error.expect("interactive diagnostics")))
                        .spawn()
                })
                .transpose();
            // Only the contestant may own these ends. In particular, keeping
            // user_stdout here prevents the interactor from seeing EOF after
            // an early return, abort, or resource-limit kill.
            drop(user_stdin);
            drop(user_stdout);
            let inter_child = match inter_child {
                Ok(child) => child,
                Err(e) => {
                    let _ = nix::sys::signal::kill(child, Signal::SIGKILL);
                    loop {
                        let waited =
                            unsafe { libc::waitpid(child.as_raw(), std::ptr::null_mut(), 0) };
                        if waited != -1
                            || std::io::Error::last_os_error().raw_os_error() != Some(libc::EINTR)
                        {
                            break;
                        }
                    }
                    return Err(format!("Failed to start interactor: {e}"));
                }
            };
            let shared_child = Arc::new(Mutex::new(inter_child));
            let shared_child_clone = shared_child.clone();
            let cancel_flag = Arc::new(AtomicBool::new(false));
            if config.max_real_time != -1 {
                let cancel_flag_clone = Arc::clone(&cancel_flag);
                let max_real_time = config.max_real_time;
                thread::spawn(move || {
                    thread::sleep(Duration::from_millis(max_real_time as u64));
                    if !cancel_flag_clone.load(Ordering::SeqCst) {
                        let _ = nix::sys::signal::kill(child, Signal::SIGKILL);
                        if let Ok(mut guard) = shared_child.lock()
                            && let Some(inter) = guard.as_mut()
                        {
                            let _ = inter.kill();
                        }
                    }
                });
            }

            let mut status: i32 = 0;
            let mut rusage: libc::rusage = unsafe { std::mem::zeroed() };
            let wait_pid = loop {
                let pid = unsafe { libc::wait4(child.as_raw(), &mut status, 0, &mut rusage) };
                if pid != -1 || std::io::Error::last_os_error().raw_os_error() != Some(libc::EINTR)
                {
                    break pid;
                }
            };
            if wait_pid == -1 {
                cancel_flag.store(true, Ordering::SeqCst);
                if let Ok(mut guard) = shared_child_clone.lock()
                    && let Some(inter) = guard.as_mut()
                {
                    let _ = inter.kill();
                    let _ = inter.wait();
                }
                result.result = ErrorCode::WaitFailed;
                return Ok(result);
            }

            let duration = start_time.elapsed().as_millis();
            result.real_time = duration as i32;
            cancel_flag.store(true, Ordering::SeqCst);

            if libc::WIFSIGNALED(status) {
                result.signal = libc::WTERMSIG(status);
            }

            if result.signal == Signal::SIGUSR1 as i32 {
                result.result = ErrorCode::SystemError;
            } else {
                result.exit_code = libc::WEXITSTATUS(status);
                // RLIMIT_CPU accounts for both user and kernel CPU time.
                // Omitting stime can report SIGXCPU as RE for syscall-heavy
                // programs even though they exceeded the requested CPU limit.
                result.cpu_time = ((rusage.ru_utime.tv_sec as i64 + rusage.ru_stime.tv_sec as i64)
                    * 1000
                    + (rusage.ru_utime.tv_usec as i64 + rusage.ru_stime.tv_usec as i64) / 1000)
                    as i32;
                result.memory = (rusage.ru_maxrss as i64) * 1024;

                if result.exit_code != 0 {
                    result.result = ErrorCode::RuntimeError;
                }
                if result.signal == Signal::SIGSEGV as i32 {
                    if config.max_memory != -1 && result.memory > config.max_memory {
                        result.result = ErrorCode::MemoryLimitExceeded;
                    } else {
                        result.result = ErrorCode::RuntimeError;
                    }
                } else {
                    if result.signal != 0 {
                        result.result = ErrorCode::RuntimeError;
                    }
                    if config.max_memory != -1 && result.memory > config.max_memory {
                        result.result = ErrorCode::MemoryLimitExceeded;
                    }
                    if config.max_real_time != -1 && result.real_time >= config.max_real_time {
                        result.result = ErrorCode::RealTimeLimitExceeded;
                    }
                    if config.max_cpu_time != -1 && result.cpu_time > config.max_cpu_time {
                        result.result = ErrorCode::CpuTimeLimitExceeded;
                    }
                }
            }
            if let Ok(mut guard) = shared_child_clone.lock()
                && let Some(inter) = guard.as_mut()
            {
                // The contestant watchdog is now cancelled, but the other
                // half of the session still needs a bounded wait. Do not hold
                // this mutex across an unbounded Child::wait().
                let status = loop {
                    match inter.try_wait() {
                        Ok(Some(status)) => break status,
                        Ok(None)
                            if config.max_real_time == -1
                                || start_time.elapsed().as_millis()
                                    < config.max_real_time as u128 =>
                        {
                            thread::sleep(Duration::from_millis(1));
                        }
                        outcome => {
                            let _ = inter.kill();
                            let _ = inter.wait();
                            // EOF caused by a contestant crash/limit must not
                            // hide the original RE/TLE/MLE behind judge errors.
                            if result.result != ErrorCode::Success {
                                return Ok(result);
                            }
                            return Err(match outcome {
                                Err(e) => format!("Failed to wait for interactor: {e}"),
                                _ => "Interactor exceeded the interaction session deadline".into(),
                            });
                        }
                    }
                };
                if !status.success() && result.result == ErrorCode::Success {
                    let mut bytes = Vec::new();
                    std::fs::File::open(&inter_error_path)
                        .and_then(|f| f.take(64 * 1024).read_to_end(&mut bytes))
                        .map_err(|e| format!("Failed to read interactor diagnostics: {e}"))?;
                    let err_output = String::from_utf8_lossy(&bytes).into_owned();
                    if (status.code().is_none() && status.signal() != Some(libc::SIGPIPE))
                        || status.code() == Some(3)
                    {
                        return Err(format!("Interactor failed ({status}): {err_output}"));
                    }
                    if !err_output.is_empty() {
                        logger
                            .write(
                                LogLevel::Fatal,
                                file!(),
                                line!(),
                                format_args!("Interactor stderr: {}", err_output),
                            )
                            .map_err(|e| format!("Failed to write to log file: {:?}", e))?;
                    }
                    result.result = ErrorCode::WrongAnswer(if err_output.is_empty() {
                        format!("Interactor rejected the output ({status})")
                    } else {
                        err_output
                    });
                }
            }

            Ok(result)
        }
        Ok(ForkResult::Child) => match child_process(
            config,
            logger,
            interactor.map(|_| (user_stdin.as_raw_fd(), user_stdout.as_raw_fd())),
        ) {
            Ok(_) => std::process::exit(0),
            Err(e) => {
                eprintln!("Child process failed: {:?}", e);
                std::process::exit(e.to_i32());
            }
        },
        Err(_) => Ok(RunResult {
            result: ErrorCode::ForkFailed,
            ..Default::default()
        }),
    }
}
