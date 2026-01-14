# Verus `-Zbuild-std` Failure Report

## Summary
`cargo verus` fails whenever the `-Zbuild-std=core,alloc` flag is used, even on the minimal hello-world crate. The failure originates inside `vstd`, which ends up linking against two copies of `core`/`alloc`, producing hundreds of duplicate diagnostic-item errors and the `exchange_malloc` lang-item conflict (E0152). This blocks any effort to verify Nanvix targets that require rebuilding `core` and `alloc`.

## Environment
- Host: Linux x86_64 (Fedora 41 container on GitHub Codespaces).
- Date: January 14, 2026.
- Verus: 0.2026.01.10.531beb1 (`verus --version`).
- Bundled rustc: 1.92.0 (the compiler Verus drives internally).
- rustup toolchains installed locally: `stable-x86_64-unknown-linux-gnu` (1.92.0) and `nightly-2025-10-28-x86_64-unknown-linux-gnu`.
- Working directory for the repro: `/home/ppenna/tmp/verus-hello` (clean hello-world project created via `cargo verus new`).

## Reproduction Steps
1. Ensure Verus is on the PATH: `export PATH="$HOME/verus:$PATH"`.
2. Change into the minimal project: `cd /home/ppenna/tmp/verus-hello`.
3. Run verification with the nightly shim and the `-Z` flag:
   ```bash
   cargo +nightly verus verify -Zbuild-std=core,alloc
   ```

## Observed Behavior
- Cargo re-compiles `core`, `alloc`, and the `vstd` dependency.
- rustc reports 395 errors like `error: duplicate diagnostic item in crate \\`alloc\\
` (for `Vec`, `Arc`, `BinaryHeap`, etc.) and dozens more for `core` macros and traits.
- Compilation terminates with `error[E0152]: duplicate lang item in crate \\`alloc\\
` (item `exchange_malloc`). Full command ends with exit code 101.
- Using an explicit toolchain override (e.g., `RUSTUP_TOOLCHAIN=nightly-2025-10-28-x86_64-unknown-linux-gnu cargo verus verify -Zbuild-std=core,alloc`) shows the identical failure signature.

## Expected Behavior
`cargo verus verify -Zbuild-std=core,alloc` should succeed so that Nanvix crates targeting `x86-user.json` can rebuild `core`/`alloc` for the custom environment.

## Impact
- Nanvix requires `-Zbuild-std` (or equivalent) to produce `core`/`alloc` for the `x86-kernel` and `x86-user` targets. Without it, verification cannot run for `type-safe` or any crate that depends on the Nanvix target JSON files.
- The breakage occurs before any Nanvix-specific code runs, so existing CI can only verify host-target crates (missing critical coverage).

## Interim Workarounds
- Dropping `-Zbuild-std` allows `cargo verus verify` to succeed on both stable and nightly toolchains, but then no custom target can be built (linker fails later because `core`/`alloc` are missing for Nanvix).
- No other workaround is known; manually pre-building a sysroot is not compatible with how Verus invokes its bundled rustc today.

## Next Steps
- Investigate whether Verus can re-use the artifacts produced by `rustup target add --toolchain nightly <target>` instead of forcing `-Zbuild-std`.
- Explore patching `vstd` or adjusting the Verus driver so it does not try to pull in the host `std` copy when `-Zbuild-std` is active.
- Reach out to the Verus maintainers with this report; include the failing command, toolchain versions, and the fact that the issue reproduces on a stock hello-world crate without any Nanvix code.
