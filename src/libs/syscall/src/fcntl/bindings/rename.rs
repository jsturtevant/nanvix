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
/// Renames a file or directory.
///
/// # Parameters
///
/// - `oldpath`: The current path of the file or directory.
/// - `newpath`: The new path for the file or directory.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Otherwise, `-1` is returned and `errno` is set
/// to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
///
/// It is safe to call this function if both `oldpath` and `newpath` point to valid
/// null-terminated C strings.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rename(oldpath: *const c_char, newpath: *const c_char) -> c_int {
    ::syslog::trace!("rename(): oldpath={oldpath:?}, newpath={newpath:?}");

    // Check if oldpath is null.
    if oldpath.is_null() {
        ::syslog::error!("rename(): oldpath is null");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check if newpath is null.
    if newpath.is_null() {
        ::syslog::error!("rename(): newpath is null");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Convert oldpath to str.
    let oldpath_str: &str = match CStr::from_ptr(oldpath).to_str() {
        Ok(s) => s,
        Err(_) => {
            ::syslog::error!("rename(): invalid oldpath");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Convert newpath to str.
    let newpath_str: &str = match CStr::from_ptr(newpath).to_str() {
        Ok(s) => s,
        Err(_) => {
            ::syslog::error!("rename(): invalid newpath");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Rename via Hyperlight guest filesystem.
    match fs::rename(oldpath_str, newpath_str) {
        Ok(()) => {
            ::syslog::trace!(
                "rename(): success (oldpath={oldpath_str:?}, newpath={newpath_str:?})"
            );
            0
        },
        Err(fs::FsError::ReadOnly) => {
            ::syslog::error!(
                "rename(): read-only file system (oldpath={oldpath_str:?}, newpath={newpath_str:?})"
            );
            *__errno_location() = ErrorCode::ReadOnlyFileSystem.get();
            -1
        },
        Err(fs::FsError::NotFound) => {
            ::syslog::error!(
                "rename(): source not found (oldpath={oldpath_str:?}, newpath={newpath_str:?})"
            );
            *__errno_location() = ErrorCode::NoSuchEntry.get();
            -1
        },
        Err(fs::FsError::AlreadyExists) => {
            ::syslog::error!(
                "rename(): destination exists (oldpath={oldpath_str:?}, newpath={newpath_str:?})"
            );
            *__errno_location() = ErrorCode::EntryExists.get();
            -1
        },
        Err(fs::FsError::InvalidPath) => {
            ::syslog::error!(
                "rename(): paths on different mounts (oldpath={oldpath_str:?}, \
                 newpath={newpath_str:?})"
            );
            *__errno_location() = ErrorCode::CrossDeviceLink.get();
            -1
        },
        Err(e) => {
            ::syslog::error!(
                "rename(): {e:?} (oldpath={oldpath_str:?}, newpath={newpath_str:?})"
            );
            *__errno_location() = ErrorCode::IoErr.get();
            -1
        },
    }
}
