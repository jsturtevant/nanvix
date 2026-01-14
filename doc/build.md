# Building Nanvix

> ℹ️ The instructions in this document assume that you have a system with the
development environment already set up. For more information on how to set up
your development environment, please refer to the [setup.md](setup.md) document.

This document guides you through building Nanvix. You can either use the `z` utility script for a
simplified build process or do it manually.

## Table of Contents

- [Building Nanvix with `z` (Preferred Method)](#building-nanvix-with-z-preferred-method)
  - [Getting Started with `z`](#getting-started-with-z)
  - [Using `z` to Build Nanvix with Docker](#using-z-to-build-nanvix-with-docker)
  - [Using `z` to Build Nanvix with a Local Toolchain](#using-z-to-build-nanvix-with-a-local-toolchain)
- [Building Nanvix Manually](#building-nanvix-manually)
  - [Manually Building Nanvix with Docker](#manually-building-nanvix-with-docker)
  - [Manually Building Nanvix with a Local Toolchain](#manually-building-nanvix-with-a-local-toolchain)

## Building Nanvix with `z` (Preferred Method)

`z` is a utility for building Nanvix. It provides you with a simplified interface for building
Nanvix either using Docker or your local toolchain.

### Getting Started with `z`

For more information on how to use the `z` utility, you can run:

```bash
./z help
```

### Using `z` to Build Nanvix with Docker

To build Nanvix using the latest Docker image and default build parameters, run:

```bash
./z build --with-docker -- all
```

### Using `z` to Build Nanvix with a Local Toolchain

To build Nanvix using your local toolchain and default build parameters, run:

```bash
./z build -- all
```

## Building Nanvix Manually

Instead of using the `z` utility, you can build Nanvix manually.

### Manually Building Nanvix with Docker

To build Nanvix using the latest Docker image and default build parameters, run:

```bash
docker run \
  -it --rm -v"$(pwd):/mnt" \
  nanvix/toolchain \
  /bin/bash -l -c "\
    set -e; \
    cd /mnt ; \
    git config --global --add safe.directory '*' ; \
    make TOOLCHAIN_DIR=/opt/nanvix all ; \
    chown -R $(id -u):$(id -g) . "
```

### Manually Building Nanvix with a Local Toolchain

To build Nanvix using your local toolchain and default build parameters, run:

```bash
make all
```

## Verus Formal Verification

Nanvix integrates the [Verus](https://www.verus-lang.org/) toolchain via `make verify`. The target invokes `cargo verus verify` for the same crate lists that drive the regular build, so each artifact keeps the exact `RUSTFLAGS`, target JSON files, and feature combinations it already uses.

### Prerequisites

1. Install Verus and ensure `cargo-verus` is available. By default the build looks in `$(HOME)/verus`, but you can override the location by exporting `VERUS_HOME=/path/to/verus`.
2. Keep the Nanvix toolchain synced; `make verify` reuses the same compiler configuration that `make all` does.
3. Decide whether you want to narrow the crate set. By default, `make verify` walks the same `ALL_*` lists that the standard build uses (guest staticlibs/rlibs/binaries, kernel, and host libraries/binaries). Override the `VERUS_*` variables only if you want to focus on a subset.

### Selecting crates for verification

- Override any of the `VERUS_*` variables when invoking `make` to limit scope if needed. The helper lists can be interpolated directly:

  ```bash
  make VERUS_GUEST_RLIBS="proc raw-array" verify
  make VERUS_KERNEL_PACKAGES="kernel" VERUS_GUEST_RLIBS="$(ALL_GUEST_RUST_LIBS)" verify
  ```

- Host-side lists run only when `MACHINE` is `microvm` or `hyperlight`; other configurations print a skip message.
- If every `VERUS_*` list is empty, `make verify` emits a warning but still exits successfully so it can be left in CI even when no crates are ready.

### Authoring guidelines

- Verus replaces large portions of `core`/`std`, so verified modules must import the Verus standard library (`vstd`) inside their `cfg(verus)` sections. The Verus team recommends starting files with `use vstd::prelude::*;` so common items such as `Result`, `Option`, and `Vec` resolve correctly.
- Factor specification-only code under `#[cfg(verus)]` and keep executable code under the usual configurations so that normal builds remain unaffected.
- Prefer crate-specific verification flags or feature gates instead of reusing production ones; this keeps proof-only dependencies isolated.

After selecting the crate lists you need, run:

```bash
make verify
```

The command exits successfully even when crates have not yet opted into Verus. At the moment you should expect to see the upstream warning:

```
WARNING: You asked for verification, but cargo did not find any crates that opted into verification.
      If this is unexpected, try adding this entry to your Cargo.toml file:
        [package.metadata.verus]
        verify = true
```

This warning does not cause the build to fail and will disappear as crates gain the appropriate metadata.
