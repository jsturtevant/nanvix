// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::alloc::string::String;
use ::core::ffi;
use ::hyperlight_guest::fs::{
    self,
    OpenOptions,
};
use ::sys::error::ErrorCode;
use ::sysapi::{
    fcntl::{
        atflags::AT_FDCWD,
        file_access_mode::{
            O_RDONLY,
            O_RDWR,
            O_WRONLY,
        },
        file_creation_flags::{
            O_CREAT,
            O_EXCL,
            O_TRUNC,
        },
    },
    ffi::{
        c_char,
        c_int,
    },
    sys_types::mode_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Opens the file specified by `pathname` relative to the directory referred to by `dirfd`.
///
/// If `pathname` is absolute, `dirfd` is ignored.
/// If `pathname` is relative and `dirfd` is `AT_FDCWD`, the path is relative to the current
/// working directory.
///
/// # Parameters
///
/// - `dirfd`: File descriptor of the directory to use as the base for relative paths.
/// - `pathname`: Pathname of the file to open.
/// - `flags`: Flags to open the file.
/// - `mode`: Mode of the file (used when creating).
///
/// # Returns
///
/// Upon successful completion, a non-negative file descriptor is returned. Otherwise, it returns
/// -1 and sets `errno` to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may dereference a raw pointer.
///
/// It is safe to call this function if the following conditions are met:
/// - `pathname` points to a valid null-terminated C string.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openat(
    dirfd: c_int,
    pathname: *const c_char,
    flags: c_int,
    mode: mode_t,
) -> c_int {
    ::syslog::trace!(
        "openat(): dirfd={dirfd:?}, pathname={pathname:?}, flags={flags:?}, mode={mode:?}"
    );

    // Check if `pathname` is null.
    if pathname.is_null() {
        ::syslog::error!(
            "openat(): null pathname pointer (dirfd={dirfd:?}, flags={flags:?}, mode={mode:?})"
        );
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Attempt to convert `pathname`.
    let pathname_str: &str = match ffi::CStr::from_ptr(pathname).to_str() {
        Ok(p) => p,
        Err(_) => {
            ::syslog::error!(
                "openat(): invalid pathname (dirfd={dirfd:?}, flags={flags:?}, mode={mode:?})"
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Resolve the full path.
    let full_path: String = if pathname_str.starts_with('/') {
        // Absolute path - use as-is.
        String::from(pathname_str)
    } else if dirfd == AT_FDCWD {
        // Relative to current working directory.
        match fs::cwd() {
            Ok(cwd) => {
                if cwd.ends_with('/') {
                    alloc::format!("{}{}", cwd, pathname_str)
                } else {
                    alloc::format!("{}/{}", cwd, pathname_str)
                }
            },
            Err(e) => {
                ::syslog::error!("openat(): failed to get cwd: {:?}", e);
                *__errno_location() = ErrorCode::IoErr.get();
                return -1;
            },
        }
    } else {
        // Relative to dirfd - not supported since we don't track directory paths by fd.
        // For now, treat it as relative to cwd (best effort).
        ::syslog::warn!("openat(): dirfd={} not supported, treating as AT_FDCWD", dirfd);
        match fs::cwd() {
            Ok(cwd) => {
                if cwd.ends_with('/') {
                    alloc::format!("{}{}", cwd, pathname_str)
                } else {
                    alloc::format!("{}/{}", cwd, pathname_str)
                }
            },
            Err(e) => {
                ::syslog::error!("openat(): failed to get cwd: {:?}", e);
                *__errno_location() = ErrorCode::IoErr.get();
                return -1;
            },
        }
    };

    ::syslog::trace!("openat(): resolved full_path={:?}", full_path);

    // Parse flags to determine access mode and creation options.
    let access_mode: c_int = flags & 0x3;
    let read: bool = access_mode == O_RDONLY || access_mode == O_RDWR;
    let write: bool = access_mode == O_WRONLY || access_mode == O_RDWR;
    let create: bool = (flags & O_CREAT) != 0;
    let truncate: bool = (flags & O_TRUNC) != 0;
    let create_new: bool = create && (flags & O_EXCL) != 0;
    let final_read: bool = read || (!read && !write);

    ::syslog::trace!(
        "openat(): parsed flags (read={}, write={}, create={}, truncate={}, create_new={}, \
         final_read={})",
        read,
        write,
        create,
        truncate,
        create_new,
        final_read
    );

    // Build OpenOptions based on flags.
    let file_result: Result<fs::File, fs::FsError> = OpenOptions::new()
        .read(final_read)
        .write(write)
        .create(create && !create_new)
        .create_new(create_new)
        .truncate(truncate)
        .open(&full_path);

    match file_result {
        Ok(file) => {
            let fd: c_int = file.into_raw_fd();
            ::syslog::trace!("openat(): fd={}", fd);
            fd
        },
        Err(e) => {
            ::syslog::error!("openat(): failed to open file (path={:?}, error={:?})", full_path, e);
            let errno: i32 = match e {
                fs::FsError::NotFound => ErrorCode::NoSuchEntry.get(),
                fs::FsError::AlreadyExists => ErrorCode::EntryExists.get(),
                fs::FsError::ReadOnly => ErrorCode::ReadOnlyFileSystem.get(),
                fs::FsError::NotADirectory => ErrorCode::InvalidDirectory.get(),
                fs::FsError::InvalidPath => ErrorCode::InvalidArgument.get(),
                _ => ErrorCode::IoErr.get(),
            };
            *__errno_location() = errno;
            -1
        },
    }
}
