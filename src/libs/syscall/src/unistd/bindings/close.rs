// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    unistd,
};
use ::sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Closes a file descriptor. The `close()` function closes the file descriptor `fd`,
/// freeing it for reuse. This function uses `hyperlight_guest::fs::free_fd()` to
/// release the file descriptor.
///
/// # Parameters
///
/// - `fd`: File descriptor to close.
///
/// # Returns
///
/// Upon successful completion, `close()` returns `0`. Otherwise, it returns `-1` and
/// sets `errno` to indicate the error.
///
#[unsafe(no_mangle)]
pub extern "C" fn close(fd: c_int) -> c_int {
    ::syslog::trace!("close(): fd = {}", fd);
    match unistd::close(fd) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!("close(): failed ({:?})", error);
            unsafe {
                *__errno_location() = error.code.get();
            }
            -1
        },
    }
}
