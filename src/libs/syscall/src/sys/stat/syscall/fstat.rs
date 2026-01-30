// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::{
    sys_stat::{
        self,
        file_type::S_IFREG,
    },
    unistd::{
        STDERR_FILENO,
        STDIN_FILENO,
        STDOUT_FILENO,
    },
};
use hyperlight_guest::fs::{
    get_fd_entry,
    File,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// The `fstat()` system call obtains information about a file using
/// `hyperlight_guest::fs::File::from_raw_fd()` + `File::size()`.
///
/// # Parameters
///
/// - `fd`: File descriptor of the file.
/// - `buf`: Buffer to store file information.
///
/// # Returns
///
/// Upon successful completion, empty result is returned. Upon failure, an error is returned
/// instead.
///
pub fn fstat(fd: i32, buf: &mut sys_stat::stat) -> Result<(), Error> {
    ::syslog::trace!("fstat(): fd={}", fd);

    // Handle standard file descriptors specially.
    if fd == STDIN_FILENO || fd == STDOUT_FILENO || fd == STDERR_FILENO {
        // For standard streams, return a character device-like stat.
        *buf = sys_stat::stat::default();
        buf.st_dev = 0;
        buf.st_ino = 0;
        #[allow(clippy::as_conversions)]
        {
            buf.st_mode = ::sysapi::sys_stat::file_type::S_IFCHR | 0o666;
        }
        buf.st_nlink = 1;
        buf.st_uid = 0;
        buf.st_gid = 0;
        buf.st_rdev = 0;
        buf.st_size = 0;
        buf.st_atim = ::sysapi::time::timespec::default();
        buf.st_mtim = ::sysapi::time::timespec::default();
        buf.st_ctim = ::sysapi::time::timespec::default();
        buf.st_blksize = 512;
        buf.st_blocks = 0;
        return Ok(());
    }

    // Check if the fd is a FAT file.
    let is_fat: bool = match get_fd_entry(fd) {
        Ok(entry) => entry.is_fat(),
        Err(_) => {
            ::syslog::error!("fstat(): invalid file descriptor (fd={})", fd);
            return Err(Error::new(ErrorCode::BadFile, "invalid file descriptor"));
        },
    };

    // SAFETY: fd comes from C API, validated above. We use from_raw_fd because
    // the fd ownership belongs to the caller, not us.
    let mut file: File = unsafe { File::from_raw_fd(fd, is_fat) };

    // Get the file size.
    let size: u64 = match file.size() {
        Ok(size) => size,
        Err(e) => {
            ::syslog::error!("fstat(): failed to get file size for fd={}: {:?}", fd, e);
            // Prevent Drop from closing the fd - we don't own it, the caller does.
            ::core::mem::forget(file);
            return Err(Error::new(ErrorCode::BadFile, "fstat() failed"));
        },
    };

    // Prevent Drop from closing the fd - we don't own it, the caller does.
    ::core::mem::forget(file);

    // Fill the POSIX stat structure with the metadata.
    // For files opened via fd, we only know size - treat as regular file.
    *buf = sys_stat::stat::default();
    buf.st_dev = 0;
    buf.st_ino = 0;
    buf.st_mode = S_IFREG | 0o644; // Regular file with rw-r--r-- permissions.
    buf.st_nlink = 1;
    buf.st_uid = 0;
    buf.st_gid = 0;
    buf.st_rdev = 0;
    // Convert u64 to i64 with overflow check.
    buf.st_size = match size.try_into() {
        Ok(s) => s,
        Err(_) => {
            ::syslog::error!("fstat(): size overflow (fd={}, size={})", fd, size);
            return Err(Error::new(ErrorCode::ValueOverflow, "file size exceeds i64 range"));
        },
    };
    buf.st_atim = ::sysapi::time::timespec::default();
    buf.st_mtim = ::sysapi::time::timespec::default();
    buf.st_ctim = ::sysapi::time::timespec::default();
    buf.st_blksize = 512;
    // Calculate blocks with saturating add to prevent overflow, then convert.
    let blocks: u64 = size.saturating_add(511) / 512;
    buf.st_blocks = match blocks.try_into() {
        Ok(b) => b,
        Err(_) => {
            ::syslog::error!("fstat(): blocks overflow (fd={}, blocks={})", fd, blocks);
            return Err(Error::new(ErrorCode::ValueOverflow, "block count exceeds i64 range"));
        },
    };

    ::syslog::trace!("fstat(): fd={}, size={}", fd, size);
    Ok(())
}
