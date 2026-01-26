// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::sys_stat::{
    self,
    file_type::{
        S_IFDIR,
        S_IFREG,
    },
};
use hyperlight_guest::fs;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// The `stat()` system call obtains information about a file using `hyperlight_guest::fs::stat()`.
///
/// # Parameters
///
/// - `pathname`: Path to the file.
/// - `statbuf`: Buffer to store file information.
///
/// # Returns
///
/// Upon successful completion, empty result is returned. Upon failure, an error is returned
/// instead.
///
pub fn stat(pathname: &str, statbuf: &mut sys_stat::stat) -> Result<(), Error> {
    ::syslog::trace!("stat(): pathname = {:?}", pathname);

    // Use hyperlight_guest::fs::stat() to get file metadata.
    let file_stat: fs::Stat = match fs::stat(pathname) {
        Ok(stat) => stat,
        Err(e) => {
            ::syslog::error!("stat(): failed to stat file {:?}: {:?}", pathname, e);
            let error_code: ErrorCode = match e {
                fs::FsError::NotFound => ErrorCode::NoSuchEntry,
                fs::FsError::NotAFile => ErrorCode::IsDirectory,
                fs::FsError::NotADirectory => ErrorCode::InvalidDirectory,
                fs::FsError::InvalidFd => ErrorCode::BadFile,
                fs::FsError::ReadOnly => ErrorCode::ReadOnlyFileSystem,
                fs::FsError::InvalidPath => ErrorCode::InvalidArgument,
                _ => ErrorCode::IoErr,
            };
            return Err(Error::new(error_code, "stat() failed"));
        },
    };

    // Fill the POSIX stat structure with the metadata.
    // Use defaults for fields that aren't available from hyperlight_guest::fs::stat().
    *statbuf = sys_stat::stat::default();
    statbuf.st_dev = 0; // Device ID (not applicable).
    statbuf.st_ino = 0; // Inode number (not applicable).
    #[allow(clippy::as_conversions)]
    if file_stat.is_dir {
        statbuf.st_mode = S_IFDIR | 0o755; // Directory with rwxr-xr-x permissions.
    } else {
        statbuf.st_mode = S_IFREG | 0o644; // Regular file with rw-r--r-- permissions.
    }
    statbuf.st_nlink = 1; // Link count (always 1).
    statbuf.st_uid = 0; // User ID (root).
    statbuf.st_gid = 0; // Group ID (root).
    statbuf.st_rdev = 0; // Device ID (if special file, not applicable).
                         // Convert u64 to i64 with overflow check.
    statbuf.st_size = match file_stat.size.try_into() {
        Ok(s) => s,
        Err(_) => {
            ::syslog::error!(
                "stat(): size overflow (pathname={:?}, size={})",
                pathname,
                file_stat.size
            );
            return Err(Error::new(ErrorCode::ValueOverflow, "file size exceeds i64 range"));
        },
    };
    // Time fields set to 0 (not available from hyperlight_guest::fs).
    statbuf.st_atim = ::sysapi::time::timespec::default();
    statbuf.st_mtim = ::sysapi::time::timespec::default();
    statbuf.st_ctim = ::sysapi::time::timespec::default();
    statbuf.st_blksize = 512; // Block size.
                              // Calculate blocks with saturating add to prevent overflow, then convert.
    let blocks: u64 = file_stat.size.saturating_add(511) / 512;
    statbuf.st_blocks = match blocks.try_into() {
        Ok(b) => b,
        Err(_) => {
            ::syslog::error!(
                "stat(): blocks overflow (pathname={:?}, blocks={})",
                pathname,
                blocks
            );
            return Err(Error::new(ErrorCode::ValueOverflow, "block count exceeds i64 range"));
        },
    };

    ::syslog::trace!(
        "stat(): pathname={:?}, size={}, is_dir={}",
        pathname,
        file_stat.size,
        file_stat.is_dir
    );
    Ok(())
}
