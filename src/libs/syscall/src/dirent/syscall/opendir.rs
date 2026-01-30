// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::dirent::{
    posix_dent,
    DirectoryStream,
};
use ::alloc::{
    boxed::Box,
    vec::Vec,
};
use ::hyperlight_guest::fs::{
    self,
    DirEntry,
};
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::{
    dirent::dirent_file_type::{
        DT_DIR,
        DT_REG,
    },
    limits::NAME_MAX,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Opens a directory stream using Hyperlight's `read_dir()`.
///
/// # Description
///
/// Uses `hyperlight_guest::fs::read_dir()` to read all directory entries at once,
/// then stores them in the `DirectoryStream` for subsequent `readdir()` calls.
///
/// # Parameters
///
/// - `dirname`: Path to the directory to open.
///
/// # Returns
///
/// Upon successful completion, a boxed `DirectoryStream` is returned. Otherwise, an error is
/// returned.
///
pub fn opendir(dirname: &str) -> Result<Box<DirectoryStream>, Error> {
    ::syslog::trace!("opendir(): dirname={:?}", dirname);

    // Read all directory entries using Hyperlight FS.
    let entries: Vec<DirEntry> = match fs::read_dir(dirname) {
        Ok(entries) => entries,
        Err(e) => {
            ::syslog::error!("opendir(): failed to read directory {:?}: {:?}", dirname, e);
            let error_code: ErrorCode = match e {
                fs::FsError::NotFound => ErrorCode::NoSuchEntry,
                fs::FsError::NotADirectory => ErrorCode::InvalidDirectory,
                fs::FsError::InvalidPath => ErrorCode::InvalidArgument,
                _ => ErrorCode::IoErr,
            };
            return Err(Error::new(error_code, "opendir() failed"));
        },
    };

    // Create directory stream with a placeholder fd (-1 since we don't use IPC).
    let mut dir: DirectoryStream = DirectoryStream::new(-1);

    // Convert DirEntry to posix_dent and push to the stream.
    for entry in entries {
        let mut d_name: [u8; NAME_MAX + 1] = [0u8; NAME_MAX + 1];
        let name_bytes: &[u8] = entry.name.as_bytes();
        let copy_len: usize = core::cmp::min(name_bytes.len(), NAME_MAX);
        d_name[..copy_len].copy_from_slice(&name_bytes[..copy_len]);

        #[allow(clippy::as_conversions)]
        let d_type: u8 = if entry.is_dir { DT_DIR } else { DT_REG };

        #[allow(clippy::as_conversions)]
        let d_reclen: u16 = core::mem::size_of::<posix_dent>() as u16;

        let posix_entry: posix_dent = posix_dent {
            d_ino: 0, // Inode not available from Hyperlight FS.
            d_reclen,
            d_type,
            d_name,
            _padding: [0],
        };
        dir.push(posix_entry);
    }

    ::syslog::trace!(
        "opendir(): opened directory {:?} with {} entries",
        dirname,
        dir.entry_count()
    );

    Ok(Box::new(dir))
}
