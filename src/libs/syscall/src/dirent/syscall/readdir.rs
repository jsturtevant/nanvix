// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::dirent::DirectoryStream;
use ::alloc::boxed::Box;
use ::sys::error::Error;
use ::sysapi::dirent::dirent;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Reads the next directory entry from a directory stream.
///
/// # Description
///
/// Uses the pre-populated entries from `opendir()` which called Hyperlight's `read_dir()`.
/// No IPC is needed since all entries were loaded when the directory was opened.
///
/// # Parameters
///
/// - `dir`: Mutable reference to the directory stream.
///
/// # Returns
///
/// Upon successful completion, returns `Ok(Some(dirent))` with the next entry, or `Ok(None)` if
/// the end of the directory stream has been reached. Otherwise, an error is returned.
///
pub fn readdir(dir: &mut Box<DirectoryStream>) -> Result<Option<dirent>, Error> {
    ::syslog::trace!("readdir(): dir.fd={:?}, remaining={}", dir.fd(), dir.entry_count());

    // Pop the next entry from the directory stream (pre-populated by opendir).
    if let Some(posix_dirent) = dir.pop() {
        let dirent: dirent = posix_dirent.into();
        return Ok(Some(dirent));
    }

    // No more entries - end of directory.
    Ok(None)
}
