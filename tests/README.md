# Isolated judge tests

Run only in a disposable checkout or container without production data mounts.
The fixture preparation refuses to overwrite assets/problems and requires an
explicit disposable-test marker:

    SEUOJ_DISPOSABLE_TESTS=1 bash tests/prepare-fixtures.sh
    cargo test --locked

Install libseccomp development headers, pkg-config, a C++ compiler and zip.
The runner requires the privilege configured by .cargo/config.toml; a root
test container can override CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUNNER=env.
All fixture accounts/problems are synthetic; no production database is used.

rust_submission.py additionally exercises the complete runtime image, all
eight languages, Rust limits and sandbox restrictions, graders, SPJ CPU-time
arguments, interactive judging and subtasks. Run it with uv in a disposable
container with --network none and no production mounts. It starts its own
runner and loopback callback server, and stores data in a temporary directory.
The test image must provide uv and /app/runner; use uv run --no-project
--offline --python /usr/bin/python3 /checks/rust_submission.py.
