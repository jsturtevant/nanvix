# Plan: Add Generic `-mount` Flag and Pre-built FAT Image Support

## Overview

This plan describes two complementary filesystem mounting approaches for Hyperlight:

1. **`-mount` flag**: Runtime file/directory mapping into the guest FAT filesystem (current implementation)
2. **`-fat` flag**: Pre-built FAT image mounting for faster startup (new feature)

The `-mount` flag copies files at sandbox startup, while `-fat` uses pre-built FAT images created out-of-band via `scripts/create-python-fat.sh` or similar tools.

** important ** all all IKC messages are disabled except stdout/err on purpose.  All file system calls should be done with in the vm.

## Background: How Arguments Flow

Understanding the current system flow:

1. **Test TOML** specifies `program` (guest binary) and `program_args` (arguments to pass)
2. **nanvixd** receives these via HTTP or terminal mode
3. **`program_args`** becomes `initrd_args` passed to `UserVmArgs`
4. **VMM** passes `initrd_args` as command-line arguments to the guest binary
5. **Guest binary** (e.g., Python) must read files from the **guest filesystem**

**Key insight**: When Python receives `/__main__.py` as an argument, it needs that file to exist in the guest filesystem — not on the host.

## Current Implementation: `-mount` Flag

The `-mount host:guest` flag is fully implemented and works by:

1. Creating an empty FAT image at sandbox startup
2. Copying files/directories from host paths into the FAT filesystem
3. Mounting the FAT at `/` in the guest

**Limitation**: Large directories (e.g., Python stdlib ~100MB) are copied every sandbox startup, adding latency.

## Proposed Enhancement: `-fat` Flag for Pre-built Images

Add support for pre-built FAT images to eliminate runtime copying overhead.

### Creating Pre-built FAT Images

Use `scripts/create-python-fat.sh` to create FAT images out-of-band:

```bash
# Create Python stdlib FAT image (outputs to lib/fat/python3.12.fat)
./scripts/create-python-fat.sh sysroot-debug

# Creates: lib/fat/python3.12.fat
# Contains: /usr/lib/python3.12/
```

### Steps to Implement `-fat` Flag

#### 1. Add `-fat` CLI option

**File**: `src/uservm/src/args.rs`

- Add constant `OPT_FAT: &str = "-fat"`
- Add field `fat_images: Vec<(String, String)>` to store `(host_fat_path, mount_point)` pairs
- Add parse logic: split argument on `:`, canonicalize host path, validate mount point
- Support multiple `-fat` flags (repeatable)
- Add getter `pub fn fat_images(&self) -> &[(String, String)]`

#### 2. Propagate through VMM args

**File**: `src/uservm/src/vmm/mod.rs`

- Add `pub fat_images: Vec<(String, String)>` to `MicroVmArgs` struct
- Include field in `Debug` impl

#### 3. Propagate in lib.rs

**File**: `src/uservm/src/lib.rs`

- Add `pub fat_images: Vec<(String, String)>` to `UserVmArgs`
- Pass through to `MicroVmArgs` construction

#### 4. Update filesystem builder to use pre-built FAT images

**File**: `src/uservm/src/vmm/hyperlight/mod.rs`

Replace the current approach:

```rust
// Current: Creates empty FAT and copies files at runtime
let fs_builder = HyperlightFSBuilder::new().add_empty_fat_mount("/", fat_mount_size)?;
// ... later copies files via apply_mounts_to_fat()
```

With a hybrid approach that supports both pre-built FAT images and runtime mounts:

```rust
// New: Use pre-built FAT image if provided, otherwise create empty FAT
let fs_builder = if let Some((fat_path, mount_point)) = args.fat_images.first() {
    // Use pre-built FAT image (fast startup)
    let fat_path = std::fs::canonicalize(fat_path)?;
    HyperlightFSBuilder::new().add_fat_image(&fat_path, mount_point)?
} else if !args.mounts.is_empty() {
    // Create empty FAT and populate at runtime (slow but flexible)
    let fat_mount_size = calculate_mount_content_size(&args.mounts)?;
    let fat_mount_size = (1024 * 1024).max(fat_mount_size + (fat_mount_size / 2) + (1024 * 1024));
    HyperlightFSBuilder::new().add_empty_fat_mount("/", fat_mount_size)?
} else {
    // No filesystem needed
    HyperlightFSBuilder::new()
};
```

**Important**: When using `-fat`, additional `-mount` arguments would need to be copied into the pre-built FAT at runtime. This requires getting a mutable reference to the FAT image after `add_fat_image()`.

#### 5. Update test configs

**Files**: `test/test-single_process.toml`, `test/test-multi_process.toml`

For Python tests, use pre-built FAT image:

```toml
[[tests]]
executor = "http"
program = "${sysroot_path}/bin/python3"
program_args = "/__main__.py"
fat_images = [
    "lib/fat/python3.12.fat:/"
]
mounts = [
    "src/user/hello-python/__main__.py:/__main__.py"
]
expected_output = "Hello, from Python!"
```

## Usage Examples

### Using pre-built FAT image (fast startup)

```bash
# First, create the FAT image (one-time, at build time)
./scripts/create-python-fat.sh sysroot-debug

# Run Python with pre-built FAT (fast)
RUST_LOG=trace ./bin/nanvixd.elf \
  -fat lib/fat/python3.12.fat:/ \
  -mount src/user/hello-python/__main__.py:/__main__.py \
  -- sysroot-debug/bin/python3 /__main__.py
```

### Using runtime mounts only (current behavior)

```bash
# Run Python with runtime copying (slow for large directories)
RUST_LOG=trace ./bin/nanvixd.elf \
  -mount sysroot-debug/lib/python3.12:/usr/lib/python3.12 \
  -mount src/user/hello-python/__main__.py:/__main__.py \
  -- sysroot-debug/bin/python3 /__main__.py
```

### Comparison with existing file-rust test

```bash
# Existing pattern (uses -ramfs for single file)
RUST_LOG=trace ./bin/nanvixd.elf -ramfs README.md -- bin/file-rust.elf

# New pattern (uses -mount for flexible mapping)
RUST_LOG=trace ./bin/nanvixd.elf \
  -mount README.md:/README.md \
  -- bin/file-rust.elf
```

## Data Flow Diagram

```
Build Time (out-of-band)                Runtime (sandbox startup)
────────────────────────                ─────────────────────────

./scripts/create-python-fat.sh
         │
         ▼
lib/fat/python3.12.fat  ─────────────►  -fat lib/fat/python3.12.fat:/
   (pre-built, 127MB)                        │
                                             │ mmap'd directly (no copy)
                                             ▼
                                        Guest FAT at /
                                        └── usr/lib/python3.12/
                                            ├── os.py
                                            ├── json/
                                            └── ...

                                        -mount script.py:/__main__.py
                                             │
                                             │ copied at startup (small file)
                                             ▼
                                        /__main__.py
```

## Performance Comparison

| Approach | Python stdlib mount time | Notes |
|----------|-------------------------|-------|
| `-mount` (runtime copy) | ~2-5 seconds | Copies ~100MB every startup |
| `-fat` (pre-built) | ~10ms | mmap'd directly, no copy |

## CLI Help Output (proposed)

```
Usage: nanvixd.elf [OPTIONS] -- <PROGRAM> [PROGRAM_ARGS...]

Options:
  -fat <path>:<mount_point>         Mount pre-built FAT image at mount point
                                    Can be specified multiple times
  -mount <host_path>:<guest_path>   Mount host file/directory into guest filesystem
                                    Can be specified multiple times
  -ramfs <file>                     Mount single file (legacy, use -mount instead)
  -kernel <file>                    Kernel binary path
  -memory <size>                    Memory size in bytes
  -h, --help                        Print help

Examples:
  # Run Python with pre-built FAT image (recommended for large dependencies)
  nanvixd.elf -fat lib/fat/python3.12.fat:/ \
              -mount script.py:/__main__.py \
              -- sysroot-debug/bin/python3 /__main__.py

  # Run with runtime mounts only (for small files or development)
  nanvixd.elf -mount config.json:/etc/config.json \
              -- myapp.elf
```

## Files Changed

| File | Changes |
|------|---------|
| `src/uservm/src/args.rs` | Add `-fat` option parsing |
| `src/uservm/src/vmm/mod.rs` | Add `fat_images` to `MicroVmArgs` |
| `src/uservm/src/lib.rs` | Add `fat_images` to `UserVmArgs` |
| `src/uservm/src/vmm/hyperlight/mod.rs` | Use `add_fat_image()` when `-fat` provided |
| `scripts/create-python-fat.sh` | Already implemented |
| `test/test-*.toml` | Update Python tests to use `-fat` |

## tests to validate it works
build:
time ./z build -- BUILD_OPT=no MACHINE=hyperlight LOG_LEVEL=trace
then run:
rm -rf logs ; RUST_LOG=trace ./bin/nanvixd.elf -ramfs README.md -- bin/file-rust.elf 
rm -rf logs ; RUST_LOG=trace ./bin/nanvixd.elf -mount README.md:/README.md -- bin/file-rust.elf 
rm -rf logs ; RUST_LOG=debug ./bin/nanvixd.elf -fat lib/fat/README.md.fat:/ -- bin/file-rust.elf

Then run 

./z build --with-cached-options -- all LOG_LEVEL=trace
rm -rf logs ; RUST_LOG=trace ./bin/nanvixd.elf \
  -fat lib/fat/python3.12.fat:/ \
  -mount src/user/hello-python/__main__.py:/__main__.py \
  -- sysroot-debug/bin/python3 /__main__.py