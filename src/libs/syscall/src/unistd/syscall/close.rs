// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::unistd::{
    STDERR_FILENO,
    STDIN_FILENO,
    STDOUT_FILENO,
};
use hyperlight_guest::fs;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Closes a file descriptor.
///
/// # Description
///
/// Closes the file descriptor `fd` using `hyperlight_guest::fs::free_fd()`.
/// Standard file descriptors (stdin, stdout, stderr) are silently ignored.
///
/// # Parameters
///
/// - `fd`: File descriptor to close.
///
/// # Returns
///
/// Upon successful completion, `Ok(())` is returned. Otherwise, an error is returned.
///
pub fn close(fd: i32) -> Result<(), Error> {
    ::syslog::trace!("close(): fd={}", fd);

    // Ignore standard file descriptors.
    if fd == STDIN_FILENO || fd == STDOUT_FILENO || fd == STDERR_FILENO {
        ::syslog::trace!("close(): ignoring standard file descriptor fd={}", fd);
        return Ok(());
    }

    // Free the file descriptor using hyperlight_guest::fs::free_fd().
    match fs::free_fd(fd) {
        Ok(()) => {
            ::syslog::trace!("close(): fd={} closed successfully", fd);
            Ok(())
        },
        Err(e) => {
            ::syslog::error!("close(): failed to close fd={}, error={:?}", fd, e);
            Err(Error::new(ErrorCode::BadFile, "close() failed"))
        },
    }
}
