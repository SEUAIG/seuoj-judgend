# Local judger 0.6.2 patch

Source: crates.io package judger 0.6.2 (MIT, license retained in judger/LICENSE).
The Python and General seccomp filters now reject clone3 alongside clone.
Modern libc can create threads through clone3, bypassing the previous blacklist.
All other upstream code is unchanged. Rust uses General; its smoke test must
reject file writes, sockets and thread creation. Keep this patch until an
upstream version has the equivalent fix, then remove the Cargo patch.

The unused prebuilt interactor test executable and the duplicate original
manifest from the crate package are omitted; build the test helper from
assets/interactor.cpp when running upstream interactor tests.
