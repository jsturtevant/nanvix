// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::core::ffi::CStr;
use ::hyperlight_guest::fs;
use ::sys::error::ErrorCode;
use ::sysapi::{
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
/// Creates a new directory with the specified pathname.
///
/// # Parameters
///
/// - `pathname`: Path to the directory to create.
/// - `mode`: Permission bits for the new directory (currently ignored).
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Otherwise, `-1` is returned and `errno` is set
/// to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it dereferences a raw pointer.
///
/// It is safe to call this function if `pathname` points to a valid null-terminated C string.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mkdir(pathname: *const c_char, _mode: mode_t) -> c_int {
    ::syslog::trace!("mkdir(): pathname={pathname:?}");

    // Check if pathname is null.
    if pathname.is_null() {
        ::syslog::error!("mkdir(): pathname is null");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Convert pathname to str.
    let pathname_str: &str = match CStr::from_ptr(pathname).to_str() {
        Ok(s) => s,
        Err(_) => {
            ::syslog::error!("mkdir(): invalid pathname");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Create directory via Hyperlight guest filesystem.
    match fs::mkdir(pathname_str) {
        Ok(()) => {
            ::syslog::trace!("mkdir(): success (pathname={pathname_str:?})");
            0
        },
        Err(fs::FsError::ReadOnly) => {
            ::syslog::error!("mkdir(): read-only file system (pathname={pathname_str:?})");
            *__errno_location() = ErrorCode::ReadOnlyFileSystem.get();
            -1
        },
        Err(fs::FsError::AlreadyExists) => {
            ::syslog::error!("mkdir(): directory already exists (pathname={pathname_str:?})");
            *__errno_location() = ErrorCode::EntryExists.get();
            -1
        },
        Err(fs::FsError::NotFound) => {
            ::syslog::error!("mkdir(): parent directory not found (pathname={pathname_str:?})");
            *__errno_location() = ErrorCode::NoSuchEntry.get();
            -1
        },
        Err(e) => {
            ::syslog::error!("mkdir(): {e:?} (pathname={pathname_str:?})");
            *__errno_location() = ErrorCode::IoErr.get();
            -1
        },
    }
}
