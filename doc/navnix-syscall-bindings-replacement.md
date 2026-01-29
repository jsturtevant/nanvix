# Plan: Replace Nanvix syscall bindings with Hyperlight read-only fs APIs

Replace remaining POSIX syscall implementations in `src/libs/syscall/` with Hyperlight's `hyperlight_guest::fs` APIs for read-only file access, following the established pattern in `open.rs` and `read.rs`. Write operations return `EROFS` with a TODO for FAT support.

> **Note:** All implementations should go in the **bindings** files (e.g., `src/libs/syscall/src/unistd/bindings/*.rs`), not the syscall files. The bindings layer provides the C ABI `extern "C"` functions and should contain the full implementation using Hyperlight APIs directly.

## Steps

### 1. Update `write` binding

**File:** `src/libs/syscall/src/unistd/bindings/write.rs`

Return `-1` with `errno = EROFS` for file fds (keep stdout/stderr working via existing VmbusWrite). Add `// TODO: implement via File::write() when FAT support is added`.

### 2. Update `close` binding

**File:** `src/libs/syscall/src/unistd/bindings/close.rs`

Use `hyperlight_guest::fs::fd::free_fd(fd)` to release the file descriptor.

### 3. Update `lseek` binding

**File:** `src/libs/syscall/src/unistd/bindings/lseek.rs`

Use `File::from_raw_fd()` + `Seek::seek()` with `SeekFrom::Start/Current/End` based on `whence`, then `core::mem::forget()`.

### 4. Update `stat`/`fstat` bindings

**Files:** `src/libs/syscall/src/sys_stat/bindings/`

Use `fs::stat(path)` for `stat`, `File::from_raw_fd()` + `File::size()` for `fstat`. Fill POSIX struct with `size`, `is_dir` → `st_mode`, defaults for rest (uid=0, times=0).

### 5. Update `readdir`/`posix_getdents`

**Files:** `src/libs/syscall/src/dirent/`

Have `opendir` call `fs::read_dir()` immediately, cache all `DirEntry` results in `DirectoryStream.next_entries`, convert to `dirent`/`posix_dent` format on each `readdir`/`getdents` call.

### 6. Stub remaining syscalls

| Syscall                 | Implementation                                      |
| ----------------------- | --------------------------------------------------- |
| `isatty`                | Return 1 for fd 0-2, 0 otherwise                    |
| `fcntl`                 | Return 0 for F_GETFD/F_GETFL, `-1`/EINVAL otherwise |
| `readlink`/`readlinkat` | Return `-1`/EINVAL (no symlinks)                    |
| `getcwd`                | Use `fs::cwd()`                                     |

### 7. Stub process identity syscalls (run as root)

**Files:** `src/libs/syscall/src/unistd/bindings/`

All process identity syscalls should return 0, indicating the process runs as root (uid=0, gid=0):

| Syscall   | File                 | Implementation |
| --------- | -------------------- | -------------- |
| `getuid`  | `bindings/getuid.rs` | Return `0`     |
| `geteuid` | `bindings/getuid.rs` | Return `0`     |
| `getgid`  | `bindings/getgid.rs` | Return `0`     |
| `getegid` | `bindings/getgid.rs` | Return `0`     |

### 8. Update `openat` binding

**File:** `src/libs/syscall/src/unistd/bindings/openat.rs`

Support `AT_FDCWD` (-100) only—resolve path via `fs::cwd()` + relative path, return `ENOTSUP` for other dirfd values.

## Further Considerations

### 1. Directory fd tracking

Current `DIR*` wraps fd + buffer. For Hyperlight: `opendir` caches full `read_dir()` result, no actual fd needed—or allocate a dummy fd for compatibility with code that checks `dirfd(dirp)`.

### 3. Error code mapping

| Hyperlight `FsError` | POSIX errno |
| -------------------- | ----------- |
| `NotFound`           | ENOENT      |
| `NotAFile`           | EISDIR      |
| `NotADirectory`      | ENOTDIR     |
| `InvalidFd`          | EBADF       |
| `ReadOnly`           | EROFS       |

## Reference Implementation

### Pattern from `open.rs`

```rust
use hyperlight_guest::fs;

let file = fs::open(pathname)?;
let fd: c_int = file.into_raw_fd();
```

### File system calls that need to be replaced

Cmdline: python3 src/user/hello-python/__main__.py
Total syscall events: 1022
Filtered events (non-syscall): 24
Skipped lines: 0

All calls (19 total):
    - readdir                                               682 (66.73%)
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