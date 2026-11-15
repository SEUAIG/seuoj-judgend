//! Exercise actual pipe EOF, exit statuses, and deadlines in the Linux helper.
use aij_judgend::sandbox;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

fn build(dir: &Path, name: &str, source: &str) {
    let src = dir.join(format!("{name}.c"));
    std::fs::write(&src, source).unwrap();
    assert!(
        Command::new("cc")
            .arg("-O2")
            .arg(src)
            .arg("-o")
            .arg(dir.join(name))
            .status()
            .unwrap()
            .success()
    );
}

#[test]
fn interaction_lifecycle() {
    assert_eq!(
        unsafe { libc::geteuid() },
        0,
        "run in a disposable root container"
    );
    let dir = tempfile::tempdir().unwrap();
    build(
        dir.path(),
        "user",
        r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
int main(int argc, char **argv) {
    int n;
    if (!strcmp(argv[1], "return")) return 0;
    if (!strcmp(argv[1], "eof")) { while(getchar()!=EOF) {} return 0; }
    if (!strcmp(argv[1], "spin")) { for(;;) {} }
    if (scanf("%d", &n)!=1) return 0;
    if (!strcmp(argv[1], "abort")) abort();
    if (!strcmp(argv[1], "early")) return 0;
    puts(!strcmp(argv[1], "good") ? "7" : "0");
    fflush(stdout);
    return 0;
}
"#,
    );
    build(
        dir.path(),
        "interactor",
        r#"
#include <stdio.h>
#include <string.h>
#include <unistd.h>
int main(int argc, char **argv) {
    char mode[32]; int n;
    FILE *f=fopen(argv[1],"r"); fscanf(f,"%31s",mode); fclose(f);
    if (!strcmp(mode,"close")) return 0;
    if (!strcmp(mode,"silent")) return 1;
    if (!strcmp(mode,"fail")) return 3;
    if (!strcmp(mode,"hang")) { for(;;) sleep(1); }
    puts("7"); fflush(stdout);
    if (!strcmp(mode,"verbose")) {
        for(int i=0;i<200000;i++) fputc(255,stderr);
        return 2;
    }
    if (scanf("%d",&n)!=1) { fputs("unexpected EOF",stderr); return 2; }
    if (n!=7) { fputs("wrong answer",stderr); return 1; }
    return 0;
}
"#,
    );
    let executable = std::env::var_os("AIJ_SANDBOX_EXECUTABLE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| env!("CARGO_BIN_EXE_main").into());
    let cases = [
        ("good", "normal", "AC"),
        ("early", "normal", "WA"),
        ("return", "normal", "WA"),
        ("abort", "normal", "RE"),
        ("wrong", "normal", "WA"),
        ("eof", "close", "AC"),
        ("eof", "silent", "WA"),
        ("eof", "verbose", "WA"),
        ("spin", "normal", "TLE"),
        ("eof", "fail", "ERROR"),
        ("eof", "missing", "ERROR"),
        // The user finishes before the session deadline, but the faulty
        // interactor ignores EOF. This must not escape to the outer watchdog.
        ("return", "hang", "ERROR"),
    ];
    for (id, (user, inter, expected)) in cases.iter().enumerate() {
        let input = dir.path().join(format!("{id}.in"));
        std::fs::write(&input, inter).unwrap();
        let config = judger::Config {
            exe_path: dir.path().join("user").to_string_lossy().into_owned(),
            args: vec!["user".into(), user.to_string()],
            input_path: input.to_string_lossy().into_owned(),
            output_path: dir
                .path()
                .join(format!("{id}.out"))
                .to_string_lossy()
                .into_owned(),
            error_path: dir
                .path()
                .join(format!("{id}.err"))
                .to_string_lossy()
                .into_owned(),
            log_path: dir
                .path()
                .join(format!("{id}.log"))
                .to_string_lossy()
                .into_owned(),
            max_memory: 64 * 1024 * 1024,
            max_cpu_time: 1000,
            max_real_time: 500,
            seccomp_rule_name: Some(judger::SeccompRuleName::CCpp),
            ..Default::default()
        };
        let start = Instant::now();
        let result = sandbox::run(
            &executable,
            &config,
            Some(dir.path().join(if *inter == "missing" {
                "missing"
            } else {
                "interactor"
            })),
        );
        println!("{user}/{inter}: {result:?}");
        assert!(
            start.elapsed() < Duration::from_secs(3),
            "{user}/{inter}: supervision escape"
        );
        if *expected == "ERROR" {
            assert!(result.is_err(), "{user}/{inter}");
            continue;
        }
        let result = result.unwrap();
        let actual = match result.result {
            judger::ErrorCode::Success => "AC",
            judger::ErrorCode::WrongAnswer(ref message) => {
                assert!(!message.is_empty());
                assert!(message.len() <= 3 * 64 * 1024);
                "WA"
            }
            judger::ErrorCode::RuntimeError => "RE",
            judger::ErrorCode::RealTimeLimitExceeded | judger::ErrorCode::CpuTimeLimitExceeded => {
                "TLE"
            }
            _ => "OTHER",
        };
        assert_eq!(actual, *expected, "{user}/{inter}: {result:?}");
    }
}
