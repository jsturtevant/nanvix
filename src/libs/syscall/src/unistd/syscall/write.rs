// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::safe::RawFileDescriptor;
use ::alloc::{
    borrow::Cow,
    string::String,
};
use ::hyperlight_guest::exit::debug_print;
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::{
    sys_types::c_size_t,
    unistd::{
        STDERR_FILENO,
        STDIN_FILENO,
        STDOUT_FILENO,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Writes data to a file descriptor.
///
/// For stdout and stderr, the data is written via VmbusWrite to the host.
/// For regular file descriptors, returns EROFS (read-only filesystem) since
/// the Hyperlight guest filesystem is currently read-only.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `buffer`: Buffer to write.
///
/// # Returns
///
/// Upon successful completion, the `write()` system call returns the number of bytes written.
/// Otherwise, it returns an error.
///
/// # Errors
///
/// - `ErrorCode::ReadOnlyFs` if attempting to write to a regular file (not stdout/stderr).
///
/// # TODO
///
/// Implement via File::write() when FAT filesystem support is added.
///
pub fn write(fd: RawFileDescriptor, buffer: &[u8]) -> Result<c_size_t, Error> {
    // Skip logging for stdout and stderr to avoid spamming the output.
    if fd != STDOUT_FILENO && fd != STDERR_FILENO {
        ::syslog::trace!("write(): fd={:?}, buffer.len={:?}", fd, buffer.len());
    }

    // Handle stdout/stderr via debug_print (direct to host, bypasses IPC).
    if fd == STDOUT_FILENO || fd == STDERR_FILENO {
        let msg: Cow<'_, str> = String::from_utf8_lossy(buffer);
        debug_print(&msg);
        return Ok(buffer.len() as c_size_t);
    }

    // Check if this is a regular file (not stdin/stdout/stderr).
    // Return EROFS since the Hyperlight guest filesystem is read-only.
    // TODO: Implement via File::write() when FAT filesystem support is added.
    if fd != STDIN_FILENO {
        ::syslog::warn!(
            "write(): read-only filesystem, cannot write to fd={} (buffer.len={})",
            fd,
            buffer.len()
        );
        return Err(Error::new(ErrorCode::ReadOnlyFileSystem, "read-only filesystem"));
    }

    // Writing to stdin is not allowed.
    ::syslog::error!("write(): cannot write to stdin (fd={})", fd);
    Err(Error::new(ErrorCode::BadFile, "cannot write to stdin"))
}
