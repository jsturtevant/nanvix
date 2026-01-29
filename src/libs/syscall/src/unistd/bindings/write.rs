// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    ErrorCode,
};
use ::core::slice;
use ::hyperlight_guest::{
    fs::{
        self,
        File,
    },
    Write,
};
use ::sysapi::{
    ffi::{
        c_int,
        c_void,
    },
    sys_types::{
        c_size_t,
        c_ssize_t,
    },
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
/// Writes data to a file descriptor. The `write()` function writes up to `count` bytes from the
/// buffer pointed to by `buffer` to the file referred to by the file descriptor `fd`. The number
/// of bytes written may be less than `count` if, for example, there is insufficient space on the
/// underlying physical medium, or the `RLIMIT_FSIZE` resource limit is encountered, or the call
/// was interrupted by a signal handler after having written less than `count` bytes.
///
/// # Parameters
///
/// - `fd`: File descriptor to which data will be written. This must be a valid file descriptor
///   that has been opened for writing or is a standard output descriptor (stdout/stderr).
/// - `buffer`: Pointer to the buffer containing the data to be written. The buffer must contain
///   at least `count` bytes of valid data.
/// - `count`: Number of bytes to write from the buffer. A count of `0` is invalid and will result
///   in an error.
///
/// # Returns
///
/// The `write()` function returns the number of bytes actually written on success. This may be
/// less than `count` if the write was interrupted or if there was insufficient space. On error,
/// it returns `-1` and sets `errno` to indicate the error. Common error conditions include
/// invalid file descriptor, invalid buffer pointer, or insufficient disk space.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers and modify global state.
///
/// It is safe to call this function if and only if all the following conditions are met:
/// - `buffer` points to a valid memory location containing at least `count` bytes of data.
/// - `buffer` remains valid and readable for the duration of the function call.
/// - `buffer` is properly aligned for byte access.
/// - `fd` refers to a valid, open file descriptor with write permissions.
/// - Access to `errno` is synchronized with other threads that may modify it.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn write(fd: c_int, buffer: *const c_void, count: c_size_t) -> c_ssize_t {
    // Skip logging for stdout and stderr to avoid spamming the output.
    if fd != STDOUT_FILENO && fd != STDERR_FILENO {
        ::syslog::trace!("write(): fd={fd:?}, buffer={buffer:?}, count={count:?}");
    }

    // Check if buffer is invalid.
    if buffer.is_null() {
        ::syslog::error!("write(): invalid buffer (fd={fd:?}, buffer={buffer:?}, count={count:?})");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check if count is invalid.
    if count == 0 {
        ::syslog::error!(
            "write(): invalid write count (fd={fd:?}, buffer={buffer:?}, count={count:?})"
        );
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Handle stdout/stderr via existing syscall (VmbusWrite).
    if fd == STDOUT_FILENO || fd == STDERR_FILENO {
        let buffer: &[u8] = slice::from_raw_parts(buffer as *const u8, count as usize);
        match crate::unistd::syscall::write(fd, buffer) {
            Ok(bytes_written) => return bytes_written as c_ssize_t,
            Err(error) => {
                ::syslog::error!(
                    "write(): {error:?} (fd={fd:?}, buffer={:?}, count={count:?})",
                    buffer.as_ptr()
                );
                *__errno_location() = error.code.get();
                return -1;
            },
        }
    }

    // Handle stdin (invalid for write).
    if fd == STDIN_FILENO {
        ::syslog::error!("write(): cannot write to stdin (fd={fd:?})");
        *__errno_location() = ErrorCode::BadFile.get();
        return -1;
    }

    // Write to FAT file via Hyperlight guest filesystem directly.
    // File implements embedded_io::Write, so we can call write() directly.
    let buffer: &[u8] = slice::from_raw_parts(buffer as *const u8, count as usize);
    let mut file: File = File::from_fd(fd);
    match file.write(buffer) {
        Ok(bytes_written) => {
            // Don't drop the File (it would close the fd).
            let _ = file.into_raw_fd();
            bytes_written as c_ssize_t
        },
        Err(fs::FsError::ReadOnly) => {
            let _ = file.into_raw_fd();
            ::syslog::error!("write(): read-only file system (fd={fd:?})");
            *__errno_location() = ErrorCode::ReadOnlyFileSystem.get();
            -1
        },
        Err(e) => {
            let _ = file.into_raw_fd();
            ::syslog::error!("write(): {e:?} (fd={fd:?})");
            *__errno_location() = ErrorCode::IoErr.get();
            -1
        },
    }
}
