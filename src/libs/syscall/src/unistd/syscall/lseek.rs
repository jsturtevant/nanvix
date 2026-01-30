// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::safe::RawFileDescriptor;
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::{
    ffi::c_int,
    sys_types::off_t,
    unistd::file_seek::{
        SEEK_CUR,
        SEEK_END,
        SEEK_SET,
    },
};
use hyperlight_guest::{
    fs::{
        get_fd_entry,
        File,
    },
    Seek,
    SeekFrom,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Repositions the file offset of the open file description associated with `fd`.
///
/// # Description
///
/// Uses `hyperlight_guest::fs::File::from_raw_fd()` + `Seek::seek()` with
/// `SeekFrom::Start/Current/End` based on `whence`, then `core::mem::forget()`.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `offset`: Offset to set.
/// - `whence`: Reference point for the offset (SEEK_SET, SEEK_CUR, or SEEK_END).
///
/// # Returns
///
/// Upon successful completion, the resulting offset is returned. Otherwise, an error is returned.
///
pub fn lseek(fd: RawFileDescriptor, offset: off_t, whence: c_int) -> Result<off_t, Error> {
    ::syslog::trace!("lseek(): fd={:?}, offset={}, whence={}", fd, offset, whence);

    // Convert whence to SeekFrom.
    let seek_from: SeekFrom = match whence {
        SEEK_SET => {
            if offset < 0 {
                ::syslog::error!(
                    "lseek(): negative offset with SEEK_SET (fd={}, offset={})",
                    fd,
                    offset
                );
                return Err(Error::new(
                    ErrorCode::InvalidArgument,
                    "negative offset with SEEK_SET",
                ));
            }
            #[allow(clippy::as_conversions)]
            SeekFrom::Start(offset as u64)
        },
        SEEK_CUR => SeekFrom::Current(offset),
        SEEK_END => SeekFrom::End(offset),
        _ => {
            ::syslog::error!("lseek(): invalid whence value (fd={}, whence={})", fd, whence);
            return Err(Error::new(ErrorCode::InvalidArgument, "invalid whence value"));
        },
    };

    // Check if the fd is a FAT file.
    let is_fat: bool = match get_fd_entry(fd) {
        Ok(entry) => entry.is_fat(),
        Err(_) => {
            ::syslog::error!("lseek(): invalid file descriptor (fd={})", fd);
            return Err(Error::new(ErrorCode::BadFile, "invalid file descriptor"));
        },
    };

    // SAFETY: fd comes from C API, validated above. We use from_raw_fd because
    // the fd ownership belongs to the caller, not us.
    let mut file: File = unsafe { File::from_raw_fd(fd, is_fat) };

    // Perform the seek operation.
    let new_pos: u64 = match file.seek(seek_from) {
        Ok(pos) => pos,
        Err(e) => {
            ::syslog::error!("lseek(): seek failed (fd={}, error={:?})", fd, e);
            // Prevent Drop from closing the fd - we don't own it, the caller does.
            ::core::mem::forget(file);
            return Err(Error::new(ErrorCode::InvalidArgument, "seek failed"));
        },
    };

    // Convert u64 to off_t (i64) with overflow check.
    let result: off_t = match new_pos.try_into() {
        Ok(pos) => pos,
        Err(_) => {
            ::syslog::error!("lseek(): position overflow (fd={}, new_pos={})", fd, new_pos);
            ::core::mem::forget(file);
            return Err(Error::new(ErrorCode::ValueOverflow, "position exceeds off_t range"));
        },
    };

    // Prevent Drop from closing the fd - we don't own it, the caller does.
    ::core::mem::forget(file);

    ::syslog::trace!("lseek(): fd={}, new_offset={}", fd, result);
    Ok(result)
}
