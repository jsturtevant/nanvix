// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::hyperlight_guest::fs::{
    self,
    FdEntry,
};
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::{
    fcntl::file_control_request::{
        F_DUPFD,
        F_DUPFD_CLOEXEC,
        F_DUPFD_CLOFORK,
        F_GETFD,
        F_GETFL,
        F_GETOWN,
        F_SETFD,
        F_SETFL,
        F_SETOWN,
    },
    ffi::c_int,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Performs a file control operation on the specified file descriptor using hyperlight filesystem.
///
/// # Parameters
///
/// - `fd`: File descriptor to operate on.
/// - `cmd`: File control command (F_DUPFD, F_GETFL, etc.).
/// - `arg`: Optional argument for the command.
///
/// # Returns
///
/// Upon successful completion, returns the result of the operation. Otherwise, returns an error.
///
pub fn fcntl(fd: i32, cmd: i32, arg: Option<c_int>) -> Result<c_int, Error> {
    ::syslog::trace!("fcntl(): fd={:?}, cmd={:?}, arg={:?}", fd, cmd, arg);

    let result: Result<c_int, Error> = match cmd {
        F_DUPFD => {
            // Duplicate file descriptor to lowest available >= arg.
            let min_fd: i32 = arg.unwrap_or(0);
            match fs::dup_fd_to(fd, None, Some(min_fd)) {
                Ok(new_fd) => {
                    ::syslog::trace!("fcntl(): F_DUPFD: new_fd={:?}", new_fd);
                    Ok(new_fd)
                },
                Err(e) => {
                    ::syslog::error!("fcntl(): F_DUPFD failed (fd={:?}, error={:?})", fd, e);
                    Err(map_fs_error_to_error(e))
                },
            }
        },
        F_DUPFD_CLOEXEC | F_DUPFD_CLOFORK => {
            // Duplicate file descriptor to lowest available >= arg.
            // Note: CLOEXEC/CLOFORK flags are not tracked in hyperlight_guest::fs.
            // but we still duplicate the fd. The flags would need to be tracked separately.
            let min_fd: i32 = arg.unwrap_or(0);
            match fs::dup_fd_to(fd, None, Some(min_fd)) {
                Ok(new_fd) => {
                    ::syslog::trace!("fcntl(): F_DUPFD_CLOEXEC/CLOFORK: new_fd={:?}", new_fd);
                    Ok(new_fd)
                },
                Err(e) => {
                    ::syslog::error!(
                        "fcntl(): F_DUPFD_CLOEXEC/CLOFORK failed (fd={:?}, error={:?})",
                        fd,
                        e
                    );
                    Err(map_fs_error_to_error(e))
                },
            }
        },
        F_GETFD => {
            // Get file descriptor flags.
            // In hyperlight_guest::fs, we don't track FD_CLOEXEC separately.
            // Return 0 (no flags set) for now.
            match fs::get_fd_entry(fd) {
                Ok(_) => {
                    ::syslog::trace!("fcntl(): F_GETFD: flags=0");
                    Ok(0)
                },
                Err(e) => {
                    ::syslog::error!("fcntl(): F_GETFD failed (fd={:?}, error={:?})", fd, e);
                    Err(map_fs_error_to_error(e))
                },
            }
        },
        F_SETFD => {
            // Set file descriptor flags.
            // In hyperlight_guest::fs, we don't track FD_CLOEXEC separately.
            // Just validate the fd exists.
            match fs::get_fd_entry(fd) {
                Ok(_) => {
                    ::syslog::trace!("fcntl(): F_SETFD: flags={:?}", arg);
                    Ok(0)
                },
                Err(e) => {
                    ::syslog::error!("fcntl(): F_SETFD failed (fd={:?}, error={:?})", fd, e);
                    Err(map_fs_error_to_error(e))
                },
            }
        },
        F_GETFL => {
            // Get file status flags.
            match fs::get_fd_entry(fd) {
                Ok(entry) => {
                    let flags: c_int = match entry {
                        FdEntry::ReadOnly(_) => {
                            // Read-only files: O_RDONLY (0).
                            0
                        },
                        FdEntry::Fat(fat_entry) => {
                            // FAT files: return stored flags.
                            fat_entry.flags()
                        },
                    };
                    ::syslog::trace!("fcntl(): F_GETFL: flags={:?}", flags);
                    Ok(flags)
                },
                Err(e) => {
                    ::syslog::error!("fcntl(): F_GETFL failed (fd={:?}, error={:?})", fd, e);
                    Err(map_fs_error_to_error(e))
                },
            }
        },
        F_SETFL => {
            // Set file status flags (only O_APPEND can be modified).
            let new_flags: c_int = arg.unwrap_or(0);
            match fs::get_fd_entry(fd) {
                Ok(entry) => match entry {
                    FdEntry::ReadOnly(_) => {
                        // Read-only files cannot have flags modified.
                        ::syslog::trace!("fcntl(): F_SETFL on read-only file: ignored");
                        Ok(0)
                    },
                    FdEntry::Fat(fat_entry) => {
                        // Only O_APPEND (0x0400) can be modified.
                        let o_append: c_int = 0x0400;
                        fat_entry.set_append((new_flags & o_append) != 0);
                        ::syslog::trace!("fcntl(): F_SETFL: new_flags={:?}", new_flags);
                        Ok(0)
                    },
                },
                Err(e) => {
                    ::syslog::error!("fcntl(): F_SETFL failed (fd={:?}, error={:?})", fd, e);
                    Err(map_fs_error_to_error(e))
                },
            }
        },
        F_GETOWN | F_SETOWN => {
            // Owner operations are not supported in hyperlight guest filesystem.
            ::syslog::error!("fcntl(): F_GETOWN/F_SETOWN not supported");
            Err(Error::new(
                ErrorCode::OperationNotSupported,
                "fcntl() owner operations not supported",
            ))
        },
        _ => {
            // Unsupported command.
            ::syslog::error!("fcntl(): unsupported command (cmd={:?})", cmd);
            Err(Error::new(ErrorCode::InvalidArgument, "fcntl() unsupported command"))
        },
    };

    result
}

///
/// # Description
///
/// Maps a hyperlight filesystem error to a system error.
///
/// # Parameters
///
/// - `e`: Hyperlight filesystem error.
///
/// # Returns
///
/// The corresponding system error.
///
fn map_fs_error_to_error(e: fs::FsError) -> Error {
    let error_code: ErrorCode = match e {
        fs::FsError::NotFound => ErrorCode::NoSuchEntry,
        fs::FsError::NotAFile => ErrorCode::IsDirectory,
        fs::FsError::NotADirectory => ErrorCode::InvalidDirectory,
        fs::FsError::InvalidPath => ErrorCode::InvalidArgument,
        fs::FsError::ReadOnly => ErrorCode::ReadOnlyFileSystem,
        fs::FsError::AlreadyExists => ErrorCode::EntryExists,
        fs::FsError::NoSpace => ErrorCode::NoSpaceOnDevice,
        fs::FsError::InvalidFd => ErrorCode::BadFile,
        _ => ErrorCode::IoErr,
    };
    Error::new(error_code, "fcntl() failed")
}
