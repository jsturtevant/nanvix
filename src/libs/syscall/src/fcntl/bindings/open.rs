// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::core::ffi;
use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::{
        c_char,
        c_int,
    },
    sys_types::mode_t,
};
use hyperlight_guest::fs;

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

    // Open file using hyperlight guest filesystem (read-only).
    let file: fs::File = match fs::open(pathname) {
        Ok(f) => f,
        Err(e) => {
            ::syslog::error!("open(): failed to open file (path={:?}, error={:?})", pathname, e);
            // Map hyperlight error to POSIX errno.
            let errno: i32 = match e {
                fs::FsError::NotFound => ErrorCode::NoSuchEntry.get(),
                fs::FsError::NotAFile => ErrorCode::IsDirectory.get(),
                fs::FsError::NotADirectory => ErrorCode::InvalidDirectory.get(),
                fs::FsError::InvalidPath => ErrorCode::InvalidArgument.get(),
                _ => ErrorCode::IoErr.get(),
            };
            *__errno_location() = errno;
            return -1;
        },
    };
    let fd: c_int = file.into_raw_fd();
    ::syslog::trace!("open(): fd={}", fd);
    fd
}
