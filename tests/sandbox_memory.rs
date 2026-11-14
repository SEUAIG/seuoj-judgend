//! Regression for pre-exec RSS inherited from a large, long-lived judge service.

use aij_judgend::sandbox;
use std::path::Path;
use std::process::Command;

fn config(dir: &Path, id: usize, mib: usize, limit: i64) -> judger::Config {
    judger::Config {
        exe_path: dir.join("touch-pages").to_str().unwrap().to_owned(),
        args: vec!["touch-pages".into(), mib.to_string()],
        input_path: dir.join("in").to_str().unwrap().to_owned(),
        output_path: dir.join(format!("{id}.out")).to_str().unwrap().to_owned(),
        error_path: dir.join(format!("{id}.err")).to_str().unwrap().to_owned(),
        log_path: dir.join(format!("{id}.log")).to_str().unwrap().to_owned(),
        max_memory: limit * 1024 * 1024,
        max_cpu_time: 2000,
        max_real_time: 4000,
        seccomp_rule_name: Some(judger::SeccompRuleName::CCpp),
        ..Default::default()
    }
}

#[test]
fn memory_is_independent_of_parent_heap_and_still_enforced() {
    assert_eq!(
        unsafe { libc::geteuid() },
        0,
        "sandbox regression must run as root"
    );
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("in"), "").unwrap();
    let source = dir.path().join("touch-pages.c");
    std::fs::write(
        &source,
        r#"
#include <stdlib.h>
int main(int argc, char **argv) {
    size_t n = (size_t)atoi(argv[1]) * 1024 * 1024;
    volatile char *p = malloc(n ? n : 1);
    if (!p) return 2;
    for (size_t i = 0; i < n; i += 4096) p[i] = 1;
    free((void*)p);
    return 0;
}
"#,
    )
    .unwrap();
    assert!(
        Command::new("cc")
            .arg("-O2")
            .arg(&source)
            .arg("-o")
            .arg(dir.path().join("touch-pages"))
            .status()
            .unwrap()
            .success()
    );
    let parent_heap = vec![42u8; 230 * 1024 * 1024];
    std::hint::black_box(&parent_heap);
    let executable = Path::new(env!("CARGO_BIN_EXE_main"));
    for (id, (mib, limit, expected)) in [
        (0, 64, judger::ErrorCode::Success),
        (16, 128, judger::ErrorCode::Success),
        (96, 64, judger::ErrorCode::MemoryLimitExceeded),
        (16, 256, judger::ErrorCode::Success),
    ]
    .into_iter()
    .enumerate()
    {
        let result = sandbox::run(executable, &config(dir.path(), id, mib, limit), None).unwrap();
        println!("touch={mib}MiB limit={limit}MiB: {result:?}");
        assert_eq!(result.result, expected);
        assert!(result.memory >= (mib as i64 - 1) * 1024 * 1024);
        assert!(result.memory < (mib as i64 + 12) * 1024 * 1024);
    }
    std::thread::scope(|scope| {
        let handles: Vec<_> = (10..14)
            .map(|id| {
                let cfg = config(dir.path(), id, 8, 64);
                scope.spawn(move || sandbox::run(executable, &cfg, None).unwrap())
            })
            .collect();
        for handle in handles {
            let result = handle.join().unwrap();
            assert_eq!(result.result, judger::ErrorCode::Success);
            assert!(result.memory < 20 * 1024 * 1024);
        }
    });
    assert_eq!(parent_heap[0], 42);
}

#[test]
fn missing_helper_is_an_error() {
    let result = sandbox::run(
        Path::new("/nonexistent-seuoj-helper"),
        &judger::Config::default(),
        None,
    );
    assert!(result.is_err());
}
