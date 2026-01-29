# FAT Filesystem Support Plan for Nanvix

This document outlines the plan to add FAT filesystem support to Nanvix, enabling
writable file operations within guest VMs.

## Overview

Currently, Nanvix uses Hyperlight's guest filesystem in read-only mode, mounting
a single file. This plan adds:

1. An ephemeral FAT mount for writable storage
2. Updated syscall bindings to support file writes
3. New bindings for directory operations (mkdir, rmdir, etc.)
4. A test suite to validate the functionality

## Important Note: Bindings vs Syscalls

**This plan modifies the BINDINGS layer, not the internal syscall layer.**

The syscall library has two layers:
- **Bindings** (`src/libs/syscall/src/*/bindings/*.rs`): C ABI functions (`#[no_mangle] pub extern "C"`)
  that are called by user programs. These are the public interface.
- **Syscalls** (`src/libs/syscall/src/*/syscall/*.rs`): Internal Rust implementations that may
  communicate with daemons via IPC messages.

For FAT filesystem operations, we bypass the internal syscall layer and call Hyperlight's
guest filesystem APIs directly from the bindings. This is simpler because:
1. FAT operations don't need IPC to daemons - they work directly on shared memory
2. Hyperlight's `fs` module already provides the file/directory APIs we need
3. Fewer layers means less code to maintain

## Current State

- **VMM** ([src/uservm/src/vmm/hyperlight/mod.rs](src/uservm/src/vmm/hyperlight/mod.rs)):
  Uses `HyperlightFSBuilder::add_file()` to add a single read-only file
- **Write binding** ([src/libs/syscall/src/unistd/bindings/write.rs](src/libs/syscall/src/unistd/bindings/write.rs)):
  Calls internal syscall which returns `EROFS` for file descriptors (only stdout/stderr work)
- **No directory operations**: mkdir, rmdir, readdir, etc. bindings are not implemented

## Implementation Plan

### Step 1: Add FAT Mount to VMM

**File**: `src/uservm/src/vmm/hyperlight/mod.rs`

**Changes**:

Replace the current filesystem builder logic (around lines 233-256):

```rust
// CURRENT CODE:
let mut fs_builder: HyperlightFSBuilder = HyperlightFSBuilder::new();

if let Some(ramfs_path) = ramfs_filename.as_ref() {
    // ... validation ...
    fs_builder = fs_builder.add_file(ramfs_path, &guest_path)?;
}

let fs_image: HyperlightFSImage = fs_builder.build()?;
```

**WITH**:

```rust
// Create Hyperlight filesystem with FAT mount for writable storage.
const FAT_MOUNT_SIZE: usize = 10 * 1024 * 1024; // 10MB
let fs_builder = HyperlightFSBuilder::new()
    .add_empty_fat_mount("/data", FAT_MOUNT_SIZE)?;

// Optionally add read-only file as before.
let fs_builder = if let Some(ramfs_path) = ramfs_filename.as_ref() {
    let host_path: &Path = Path::new(ramfs_path);
    if !host_path.is_file() {
        let reason: String = format!("ramfs file not found (path={ramfs_path})");
        error!("hyperlight::new(): {reason}");
        return Err(anyhow::anyhow!(reason));
    }
    let guest_basename: String = host_path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "ramfs".to_string());
    let guest_path: String = format!("/{guest_basename}");

    fs_builder.add_file(ramfs_path, &guest_path)?
} else {
    fs_builder
};

let fs_image: HyperlightFSImage = fs_builder.build()?;
debug!("hyperlight::new(): created filesystem with FAT mount at /data ({} bytes)", FAT_MOUNT_SIZE);
```

**Note**: `add_empty_fat_mount()` transforms the builder type from `NoFat` to `WithFat`,
so `build()` will consume the builder rather than borrowing it.

---

### Step 2: Update Write Binding

**File**: `src/libs/syscall/src/unistd/bindings/write.rs`

**Changes**:

Update the `write()` binding to use Hyperlight's `File::write()` directly for FAT-backed file descriptors,
bypassing the internal syscall layer for file writes:

```rust
use ::hyperlight_guest::fs::{self, File};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn write(fd: c_int, buffer: *const c_void, count: c_size_t) -> c_ssize_t {
    // ... existing validation for buffer/count ...

    // Handle stdout/stderr via existing syscall (VmbusWrite)
    if fd == STDOUT_FILENO || fd == STDERR_FILENO {
        let buffer: &[u8] = slice::from_raw_parts(buffer as *const u8, count as usize);
        match crate::unistd::syscall::write(fd, buffer) {
            Ok(bytes_written) => bytes_written as c_ssize_t,
            Err(error) => {
                *__errno_location() = error.code.get();
                -1
            }
        }
    }

    // Handle stdin (invalid for write)
    if fd == STDIN_FILENO {
        *__errno_location() = ErrorCode::BadFileDescriptor.get();
        return -1;
    }

    // Write to FAT file via Hyperlight guest filesystem directly.
    // File implements embedded_io::Write, so we can call write() directly.
    let buffer: &[u8] = slice::from_raw_parts(buffer as *const u8, count as usize);
    let mut file: File = File::from_fd(fd);
    match file.write(buffer) {
        Ok(bytes_written) => {
            // Don't drop the File (it would close the fd).
            let _ = file.into_raw_fd();
            bytes_written as c_ssize_t
        },
        Err(fs::FsError::ReadOnly) => {
            let _ = file.into_raw_fd();
            *__errno_location() = ErrorCode::ReadOnlyFileSystem.get();
            -1
        },
        Err(_) => {
            let _ = file.into_raw_fd();
            *__errno_location() = ErrorCode::IoError.get();
            -1
        },
    }
}
```

---

### Step 3: Add Directory Bindings

Create new bindings for directory operations. These call Hyperlight's `fs` module directly,
not the internal syscall layer.

#### 3.1 mkdir

**File**: `src/libs/syscall/src/unistd/bindings/mkdir.rs` (new)

```rust
// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use crate::errno::__errno_location;
use ::core::ffi::CStr;
use ::hyperlight_guest::fs;
use ::sysapi::ffi::{c_char, c_int, c_uint};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mkdir(pathname: *const c_char, _mode: c_uint) -> c_int {
    if pathname.is_null() {
        *__errno_location() = crate::ErrorCode::InvalidArgument.get();
        return -1;
    }

    let pathname_str: &str = match CStr::from_ptr(pathname).to_str() {
        Ok(s) => s,
        Err(_) => {
            *__errno_location() = crate::ErrorCode::InvalidArgument.get();
            return -1;
        }
    };

    match fs::mkdir(pathname_str) {
        Ok(()) => 0,
        Err(fs::FsError::ReadOnly) => {
            *__errno_location() = crate::ErrorCode::ReadOnlyFileSystem.get();
            -1
        },
        Err(fs::FsError::AlreadyExists) => {
            *__errno_location() = crate::ErrorCode::FileExists.get();
            -1
        },
        Err(fs::FsError::NotFound) => {
            *__errno_location() = crate::ErrorCode::NoSuchFileOrDirectory.get();
            -1
        },
        Err(_) => {
            *__errno_location() = crate::ErrorCode::IoError.get();
            -1
        },
    }
}
```

#### 3.2 rmdir

**File**: `src/libs/syscall/src/unistd/bindings/rmdir.rs` (new)

```rust
// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use crate::errno::__errno_location;
use ::core::ffi::CStr;
use ::hyperlight_guest::fs;
use ::sysapi::ffi::{c_char, c_int};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rmdir(pathname: *const c_char) -> c_int {
    if pathname.is_null() {
        *__errno_location() = crate::ErrorCode::InvalidArgument.get();
        return -1;
    }

    let pathname_str: &str = match CStr::from_ptr(pathname).to_str() {
        Ok(s) => s,
        Err(_) => {
            *__errno_location() = crate::ErrorCode::InvalidArgument.get();
            return -1;
        }
    };

    match fs::rmdir(pathname_str) {
        Ok(()) => 0,
        Err(fs::FsError::ReadOnly) => {
            *__errno_location() = crate::ErrorCode::ReadOnlyFileSystem.get();
            -1
        },
        Err(fs::FsError::NotFound) => {
            *__errno_location() = crate::ErrorCode::NoSuchFileOrDirectory.get();
            -1
        },
        Err(fs::FsError::NotEmpty) => {
            *__errno_location() = crate::ErrorCode::DirectoryNotEmpty.get();
            -1
        },
        Err(_) => {
            *__errno_location() = crate::ErrorCode::IoError.get();
            -1
        },
    }
}
```

#### 3.3 unlink (delete file)

**File**: `src/libs/syscall/src/unistd/bindings/unlink.rs` (new)

```rust
// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use crate::errno::__errno_location;
use ::core::ffi::CStr;
use ::hyperlight_guest::fs;
use ::sysapi::ffi::{c_char, c_int};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn unlink(pathname: *const c_char) -> c_int {
    if pathname.is_null() {
        *__errno_location() = crate::ErrorCode::InvalidArgument.get();
        return -1;
    }

    let pathname_str: &str = match CStr::from_ptr(pathname).to_str() {
        Ok(s) => s,
        Err(_) => {
            *__errno_location() = crate::ErrorCode::InvalidArgument.get();
            return -1;
        }
    };

    match fs::unlink(pathname_str) {
        Ok(()) => 0,
        Err(fs::FsError::ReadOnly) => {
            *__errno_location() = crate::ErrorCode::ReadOnlyFileSystem.get();
            -1
        },
        Err(fs::FsError::NotFound) => {
            *__errno_location() = crate::ErrorCode::NoSuchFileOrDirectory.get();
            -1
        },
        Err(_) => {
            *__errno_location() = crate::ErrorCode::IoError.get();
            -1
        },
    }
}
```

#### 3.4 rename

**File**: `src/libs/syscall/src/unistd/bindings/rename.rs` (new)

```rust
// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use crate::errno::__errno_location;
use ::core::ffi::CStr;
use ::hyperlight_guest::fs;
use ::sysapi::ffi::{c_char, c_int};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rename(oldpath: *const c_char, newpath: *const c_char) -> c_int {
    if oldpath.is_null() || newpath.is_null() {
        *__errno_location() = crate::ErrorCode::InvalidArgument.get();
        return -1;
    }

    let oldpath_str: &str = match CStr::from_ptr(oldpath).to_str() {
        Ok(s) => s,
        Err(_) => {
            *__errno_location() = crate::ErrorCode::InvalidArgument.get();
            return -1;
        }
    };

    let newpath_str: &str = match CStr::from_ptr(newpath).to_str() {
        Ok(s) => s,
        Err(_) => {
            *__errno_location() = crate::ErrorCode::InvalidArgument.get();
            return -1;
        }
    };

    match fs::rename(oldpath_str, newpath_str) {
        Ok(()) => 0,
        Err(fs::FsError::ReadOnly) => {
            *__errno_location() = crate::ErrorCode::ReadOnlyFileSystem.get();
            -1
        },
        Err(fs::FsError::NotFound) => {
            *__errno_location() = crate::ErrorCode::NoSuchFileOrDirectory.get();
            -1
        },
        Err(fs::FsError::AlreadyExists) => {
            *__errno_location() = crate::ErrorCode::FileExists.get();
            -1
        },
        Err(_) => {
            *__errno_location() = crate::ErrorCode::IoError.get();
            -1
        },
    }
}
```

#### 3.5 Update open() for O_CREAT

**File**: `src/libs/syscall/src/fcntl/bindings/open.rs`

Update to use `OpenOptions` for create/write modes:

```rust
// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use crate::errno::__errno_location;
use ::core::ffi::CStr;
use ::hyperlight_guest::fs::{self, OpenOptions};
use ::sysapi::ffi::{c_char, c_int, c_uint};
use ::sysapi::fcntl::{O_CREAT, O_RDONLY, O_RDWR, O_TRUNC, O_WRONLY};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn open(pathname: *const c_char, flags: c_int, _mode: c_uint) -> c_int {
    if pathname.is_null() {
        *__errno_location() = crate::ErrorCode::InvalidArgument.get();
        return -1;
    }

    let pathname_str: &str = match CStr::from_ptr(pathname).to_str() {
        Ok(s) => s,
        Err(_) => {
            *__errno_location() = crate::ErrorCode::InvalidArgument.get();
            return -1;
        }
    };

    let read: bool = (flags & O_RDONLY != 0) || (flags & O_RDWR != 0);
    let write: bool = (flags & O_WRONLY != 0) || (flags & O_RDWR != 0);
    let create: bool = flags & O_CREAT != 0;
    let truncate: bool = flags & O_TRUNC != 0;

    let file_result = OpenOptions::new()
        .read(read || (!read && !write)) // Default to read if nothing specified.
        .write(write)
        .create(create)
        .truncate(truncate)
        .open(pathname_str);

    match file_result {
        Ok(file) => file.into_raw_fd(),
        Err(fs::FsError::NotFound) => {
            *__errno_location() = crate::ErrorCode::NoSuchFileOrDirectory.get();
            -1
        },
        Err(fs::FsError::ReadOnly) => {
            *__errno_location() = crate::ErrorCode::ReadOnlyFileSystem.get();
            -1
        },
        Err(_) => {
            *__errno_location() = crate::ErrorCode::IoError.get();
            -1
        },
    }
}
```

---

### Step 4: Create FAT Test Suite

**Directory**: `src/tests/fat-rust/`

#### 4.1 Cargo.toml

**File**: `src/tests/fat-rust/Cargo.toml`

```toml
# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

[package]
name = "fat-rust"
version.workspace = true
license-file.workspace = true
edition = "2024"
authors.workspace = true
description = "Tests for FAT Filesystem Operations"
homepage.workspace = true

[[bin]]
name = "fat-rust"

[dependencies]
nvx = { workspace = true }
libc_string = { workspace = true }
sys = { workspace = true, features = ["kcall"] }
sysapi = { workspace = true }
syscall = { workspace = true, features = ["syscall"] }
syslog = { workspace = true, default-features = true }
hyperlight-guest = { workspace = true }

[build-dependencies]
cc = { workspace = true }
cfg-if = { workspace = true }

[features]
default = ["panic"]
trace = ["nvx/trace", "syscall/trace", "syslog/trace"]
debug = ["nvx/debug", "syscall/debug", "syslog/debug"]
info = ["nvx/info", "syscall/info", "syslog/info"]
warn = ["nvx/warn", "syscall/warn", "syslog/warn"]
error = ["nvx/error", "syscall/error", "syslog/error"]
panic = ["nvx/panic", "syscall/panic", "syslog/panic"]

[lints]
workspace = true
```

#### 4.2 Main Test File

**File**: `src/tests/fat-rust/src/main.rs`

```rust
// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![no_std]
#![no_main]
#![deny(clippy::all)]
#![deny(clippy::as_conversions)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

//==================================================================================================
// Modules
//==================================================================================================

extern crate alloc;
extern crate libc_string;
extern crate nvx;

use ::syslog::{error, info};

//==================================================================================================
// Imports
//==================================================================================================

use ::core::ffi::{c_char, c_void};
use ::sys::error::Error;
use ::sysapi::unistd::file_seek::{SEEK_SET, SEEK_END};

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    let guest_fs_manifest_base: usize = 0x0fa13000;
    let guest_fs_manifest_size: usize = 0x00000058;
    info!(
        "guest_fs_manifest_base={:#x}, guest_fs_manifest_size={:#x}",
        guest_fs_manifest_base, guest_fs_manifest_size
    );
    // Initialize the hyperlight guest filesystem.
    syscall::init(guest_fs_manifest_base, guest_fs_manifest_size);

    // ========================================================================
    // Test 1: Create a directory on FAT mount
    // ========================================================================
    info!("Test 1: Creating directory /data/testdir");
    let dir_path: &[u8] = b"/data/testdir\0";
    let dir_pathname: *const c_char = dir_path.as_ptr().cast::<c_char>();
    let mkdir_result: i32 = unsafe { syscall::unistd::bindings::mkdir::mkdir(dir_pathname, 0o755) };
    if mkdir_result != 0 {
        error!("Test 1 FAILED: mkdir returned {}", mkdir_result);
        panic!("mkdir failed");
    }
    info!("Test 1 PASSED: created directory /data/testdir");

    // ========================================================================
    // Test 2: Create and write to a file
    // ========================================================================
    info!("Test 2: Creating and writing to /data/testdir/hello.txt");
    let file_path: &[u8] = b"/data/testdir/hello.txt\0";
    let file_pathname: *const c_char = file_path.as_ptr().cast::<c_char>();
    
    // Open with O_CREAT | O_WRONLY | O_TRUNC
    let flags: i32 = 0x41 | 0x01 | 0x200; // O_CREAT | O_WRONLY | O_TRUNC (Linux values)
    let fd: i32 = unsafe { syscall::fcntl::bindings::open::open(file_pathname, flags, 0o644) };
    if fd < 0 {
        error!("Test 2 FAILED: open for write returned {}", fd);
        panic!("open for write failed");
    }
    info!("Test 2a: opened file for writing with fd={}", fd);

    let write_data: &[u8] = b"Hello from FAT filesystem!";
    let bytes_written: i32 = unsafe {
        syscall::unistd::bindings::write::write(
            fd,
            write_data.as_ptr().cast::<c_void>(),
            write_data.len() as u32,
        )
    };
    if bytes_written <= 0 {
        error!("Test 2 FAILED: write returned {}", bytes_written);
        panic!("write failed");
    }
    info!("Test 2b PASSED: wrote {} bytes", bytes_written);

    // Close file
    let close_result: i32 = syscall::unistd::bindings::close::close(fd);
    if close_result != 0 {
        error!("Test 2 FAILED: close returned {}", close_result);
        panic!("close failed");
    }
    info!("Test 2 PASSED: file created and written successfully");

    // ========================================================================
    // Test 3: Read back the file
    // ========================================================================
    info!("Test 3: Reading back /data/testdir/hello.txt");
    let fd: i32 = unsafe { syscall::fcntl::bindings::open::open(file_pathname, 0, 0) };
    if fd < 0 {
        error!("Test 3 FAILED: open for read returned {}", fd);
        panic!("open for read failed");
    }

    let mut read_buf: [u8; 64] = [0; 64];
    let bytes_read: i32 = unsafe {
        syscall::unistd::bindings::read::read(
            fd,
            read_buf.as_mut_ptr().cast::<c_void>(),
            read_buf.len() as u32,
        )
    };
    if bytes_read <= 0 {
        error!("Test 3 FAILED: read returned {}", bytes_read);
        panic!("read failed");
    }
    info!("Test 3 PASSED: read {} bytes", bytes_read);

    let close_result: i32 = syscall::unistd::bindings::close::close(fd);
    if close_result != 0 {
        error!("Test 3 FAILED: close returned {}", close_result);
    }

    // ========================================================================
    // Test 4: Rename the file
    // ========================================================================
    info!("Test 4: Renaming file to /data/testdir/renamed.txt");
    let new_path: &[u8] = b"/data/testdir/renamed.txt\0";
    let new_pathname: *const c_char = new_path.as_ptr().cast::<c_char>();
    let rename_result: i32 = unsafe {
        syscall::unistd::bindings::rename::rename(file_pathname, new_pathname)
    };
    if rename_result != 0 {
        error!("Test 4 FAILED: rename returned {}", rename_result);
        panic!("rename failed");
    }
    info!("Test 4 PASSED: file renamed successfully");

    // ========================================================================
    // Test 5: Delete the file (unlink)
    // ========================================================================
    info!("Test 5: Deleting /data/testdir/renamed.txt");
    let unlink_result: i32 = unsafe {
        syscall::unistd::bindings::unlink::unlink(new_pathname)
    };
    if unlink_result != 0 {
        error!("Test 5 FAILED: unlink returned {}", unlink_result);
        panic!("unlink failed");
    }
    info!("Test 5 PASSED: file deleted successfully");

    // ========================================================================
    // Test 6: Remove directory
    // ========================================================================
    info!("Test 6: Removing directory /data/testdir");
    let rmdir_result: i32 = unsafe {
        syscall::unistd::bindings::rmdir::rmdir(dir_pathname)
    };
    if rmdir_result != 0 {
        error!("Test 6 FAILED: rmdir returned {}", rmdir_result);
        panic!("rmdir failed");
    }
    info!("Test 6 PASSED: directory removed successfully");

    // ========================================================================
    // All tests passed
    // ========================================================================
    info!("===========================================");
    info!("All FAT filesystem tests PASSED!");
    info!("===========================================");

    Ok(())
}
```

#### 4.3 Build Script

**File**: `src/tests/fat-rust/build.rs`

```rust
// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

fn main() {
    // Empty build script (matches file-rust pattern)
}
```

---

### Step 5: Update Workspace Cargo.toml

Add the new test crate to the workspace members in `/mount/d/nanvix/Cargo.toml`:

```toml
members = [
    # ... existing members ...
    "src/tests/fat-rust",
]
```

---

## Testing

### Build Command

```bash
time ./z build -- BUILD_OPT=no MACHINE=hyperlight LOG_LEVEL=trace
```

### Run FAT Tests

```bash
rm -rf logs && RUST_LOG=trace ./bin/nanvixd.elf -ramfs README.md -- bin/fat-rust.elf
```

### Run Existing File Tests (to verify no regressions)

```bash
rm -rf logs && RUST_LOG=trace ./bin/nanvixd.elf -ramfs README.md -- bin/file-rust.elf
```

---

## Summary of Files to Modify/Create

| File                                             | Action | Description                               |
| ------------------------------------------------ | ------ | ----------------------------------------- |
| `src/uservm/src/vmm/hyperlight/mod.rs`           | Modify | Add FAT mount via `add_empty_fat_mount()` |
| `src/libs/syscall/src/unistd/syscall/write.rs`   | Modify | Use `File::write()` for FAT fds           |
| `src/libs/syscall/src/fcntl/syscall/open.rs`     | Modify | Use `OpenOptions` for create/write modes  |
| `src/libs/syscall/src/unistd/syscall/mkdir.rs`   | Create | New mkdir syscall                         |
| `src/libs/syscall/src/unistd/bindings/mkdir.rs`  | Create | New mkdir binding                         |
| `src/libs/syscall/src/unistd/syscall/rmdir.rs`   | Create | New rmdir syscall                         |
| `src/libs/syscall/src/unistd/bindings/rmdir.rs`  | Create | New rmdir binding                         |
| `src/libs/syscall/src/unistd/syscall/unlink.rs`  | Create | New unlink syscall                        |
| `src/libs/syscall/src/unistd/bindings/unlink.rs` | Create | New unlink binding                        |
| `src/libs/syscall/src/unistd/syscall/rename.rs`  | Create | New rename syscall                        |
| `src/libs/syscall/src/unistd/bindings/rename.rs` | Create | New rename binding                        |
| `src/tests/fat-rust/Cargo.toml`                  | Create | New test crate config                     |
| `src/tests/fat-rust/src/main.rs`                 | Create | FAT test suite                            |
| `src/tests/fat-rust/build.rs`                    | Create | Build script                              |
| `Cargo.toml`                                     | Modify | Add fat-rust to workspace members         |

---

## Hyperlight APIs Used

| API                     | Location               | Purpose                              |
| ----------------------- | ---------------------- | ------------------------------------ |
| `add_empty_fat_mount()` | `HyperlightFSBuilder`  | Create ephemeral FAT at mount point  |
| `OpenOptions::new()`    | `hyperlight_guest::fs` | Open files with create/write options |
| `File::write()`         | `hyperlight_guest::fs` | Write to FAT files                   |
| `fs::mkdir()`           | `hyperlight_guest::fs` | Create directories                   |
| `fs::rmdir()`           | `hyperlight_guest::fs` | Remove empty directories             |
| `fs::unlink()`          | `hyperlight_guest::fs` | Delete files                         |
| `fs::rename()`          | `hyperlight_guest::fs` | Rename files/directories             |

---

## Notes

1. **FAT Mount Size**: Currently hardcoded to 10MB. Consider making this configurable later.

2. **Mount Point**: Using `/data` as the FAT mount point. Read-only files are still mounted
   at root (e.g., `/README.md`).

3. **O_* Flags**: The test uses Linux flag values. May need to verify these match sysapi definitions.

4. **Error Mapping**: Each Hyperlight `FsError` variant should map to appropriate POSIX errno values.

5. **File Descriptor Management**: When using `File::from_fd()`, remember to call `into_raw_fd()`
   to prevent the `File` destructor from closing the fd prematurely.

# required by python

 readdir                                               682 (66.73%)
      - stat                                                  120 (11.74%)
      - posix_getdents                                         51 ( 4.99%)
      - lseek                                                  42 ( 4.11%)
      - read                                                   36 ( 3.52%)
      - open                                                   21 ( 2.05%)
      - fstat                                                  17 ( 1.66%)
      - openat                                                 13 ( 1.27%)
      - isatty                                                 11 ( 1.08%)
      - fcntl                                                   6 ( 0.59%)
      - close                                                   5 ( 0.49%)
      - readlinkat                                              4 ( 0.39%)
      - getcwd                                                  3 ( 0.29%)
      - getegid                                                 2 ( 0.20%)
      - geteuid                                                 2 ( 0.20%)
      - getgid                                                  2 ( 0.20%)
      - getuid                                                  2 ( 0.20%)
      - readlink                                                2 ( 0.20%)
  - write                                                   1 ( 0.10%)