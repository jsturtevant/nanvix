// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::{
        c_char,
        c_int,
    },
    sys_types::{
        c_size_t,
        c_ssize_t,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Reads the value of a symbolic link relative to a directory file descriptor.
///
/// # Note
///
/// FAT filesystems do not support symbolic links. This function always returns -1
/// with errno set to EINVAL (Invalid argument) since symlinks cannot exist.
///
/// # Parameters
///
/// - `dirfd`: Directory file descriptor.
/// - `path`: Path to the symbolic link.
/// - `buf`: Buffer to store the value of the symbolic link.
/// - `bufsize`: Size of the buffer.
///
/// # Returns
///
/// Always returns `-1` and sets `errno` to `EINVAL` because FAT does not support symlinks.
///
/// # Safety
///
/// The function is unsafe because it may dereference pointers.
///
/// It is safe to use this function if the following conditions are met:
/// - `path` points to a valid null-terminated string.
/// - `buf` points to a valid memory location of `bufsize` bytes.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn readlinkat(
    dirfd: c_int,
    path: *const c_char,
    _buf: *mut c_char,
    _bufsize: c_size_t,
) -> c_ssize_t {
    ::syslog::trace!(
        "readlinkat(): dirfd={dirfd:?}, path={path:?} - FAT does not support symlinks"
    );

    // FAT filesystem does not support symbolic links.
    // Return EINVAL immediately without going through IPC.
    ::syslog::debug!("readlinkat(): returning EINVAL (FAT has no symlink support)");
    *__errno_location() = ErrorCode::InvalidArgument.get();
    -1
}
