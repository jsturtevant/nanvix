// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::core::ffi;
use ::sys::error::ErrorCode;
use ::sysapi::{
    fcntl::{
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
use hyperlight_guest::fs::{
    self,
    OpenOptions,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Opens the file specified by `pathname`.
///
/// # Parameters
///
/// - `path`:  Pathname of the file to open.
/// - `flags`: Flags to open the file.
/// - `mode`:  Mode of the file.
///
/// # Returns
///
/// Upon successful completion, the `open()` system call returns a non-negative integer representing
/// the lowest numbered unused file descriptor. Otherwise, it returns -1 and sets `errno` to
/// indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may deference a raw pointer.
///
/// It is safe to call this function if the following conditions are met:
///
/// - `path` points to a valid null-terminated C string.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn open(path: *const c_char, flags: c_int, mode: mode_t) -> c_int {
    ::syslog::trace!("open(): path={path:?}, flags={flags:?}, mode={mode:?}");

    // Check if `path` is null.
    if path.is_null() {
        ::syslog::error!(
            "open(): null path pointer (path={path:?}, flags={flags:?}, mode={mode:?})"
        );
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Attempt to convert `path`.
    let pathname: &str = match ffi::CStr::from_ptr(path).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::syslog::error!(
                "open(): invalid pathname (path={path:?}, flags={flags:?}, mode={mode:?})"
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Parse flags to determine access mode and creation options.
    let access_mode: c_int = flags & 0x3;
    let read: bool = access_mode == O_RDONLY || access_mode == O_RDWR;
    let write: bool = access_mode == O_WRONLY || access_mode == O_RDWR;
    let create: bool = (flags & O_CREAT) != 0;
    let truncate: bool = (flags & O_TRUNC) != 0;
    let create_new: bool = create && (flags & O_EXCL) != 0;

    ::syslog::trace!(
        "open(): parsed flags (read={read}, write={write}, create={create}, truncate={truncate}, \
         create_new={create_new})"
    );

    // Build OpenOptions based on flags.
    let file_result: Result<fs::File, fs::FsError> = OpenOptions::new()
        .read(read || (!read && !write)) // Default to read if nothing specified.
        .write(write)
        .create(create && !create_new) // create_new overrides create.
        .create_new(create_new)
        .truncate(truncate)
        .open(pathname);

    match file_result {
        Ok(file) => {
            let fd: c_int = file.into_raw_fd();
            ::syslog::trace!("open(): fd={}", fd);
            fd
        },
        Err(e) => {
            ::syslog::error!("open(): failed to open file (path={:?}, error={:?})", pathname, e);
            // Map hyperlight error to POSIX errno.
            let errno: i32 = match e {
                fs::FsError::NotFound => ErrorCode::NoSuchEntry.get(),
                fs::FsError::NotAFile => ErrorCode::IsDirectory.get(),
                fs::FsError::NotADirectory => ErrorCode::InvalidDirectory.get(),
                fs::FsError::InvalidPath => ErrorCode::InvalidArgument.get(),
                fs::FsError::ReadOnly => ErrorCode::ReadOnlyFileSystem.get(),
                fs::FsError::AlreadyExists => ErrorCode::EntryExists.get(),
                fs::FsError::NoSpace => ErrorCode::NoSpaceOnDevice.get(),
                _ => ErrorCode::IoErr.get(),
            };
            *__errno_location() = errno;
            -1
        },
    }
}
