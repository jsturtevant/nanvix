// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::core::ffi::CStr;
use ::hyperlight_guest::fs;
use ::sys::error::ErrorCode;
use ::sysapi::ffi::{
    c_char,
    c_int,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Deletes a name from the filesystem.
///
/// # Parameters
///
/// - `path`: Path to the file to be unlinked.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Otherwise, it returns -1 and sets `errno` to
/// indicate the error.
///
/// # Safety
///
/// This function is unsafe because it dereferences a raw pointer.
///
/// It is safe to call this function if `path` points to a valid null-terminated C string.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unlink(path: *const c_char) -> c_int {
    ::syslog::trace!("unlink(): path={path:?}");

    // Check if path is null.
    if path.is_null() {
        ::syslog::error!("unlink(): path is null");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Convert path to str.
    let path_str: &str = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => {
            ::syslog::error!("unlink(): invalid path");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Delete file via Hyperlight guest filesystem.
    match fs::unlink(path_str) {
        Ok(()) => {
            ::syslog::trace!("unlink(): success (path={path_str:?})");
            0
        },
        Err(fs::FsError::ReadOnly) => {
            ::syslog::error!("unlink(): read-only file system (path={path_str:?})");
            *__errno_location() = ErrorCode::ReadOnlyFileSystem.get();
            -1
        },
        Err(fs::FsError::NotFound) => {
            ::syslog::error!("unlink(): file not found (path={path_str:?})");
            *__errno_location() = ErrorCode::NoSuchEntry.get();
            -1
        },
        Err(fs::FsError::NotAFile) => {
            ::syslog::error!("unlink(): is a directory (path={path_str:?})");
            *__errno_location() = ErrorCode::IsDirectory.get();
            -1
        },
        Err(e) => {
            ::syslog::error!("unlink(): {e:?} (path={path_str:?})");
            *__errno_location() = ErrorCode::IoErr.get();
            -1
        },
    }
}
