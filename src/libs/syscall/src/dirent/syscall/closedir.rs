// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    dirent::DirectoryStream,
    unistd,
};
use ::alloc::boxed::Box;
use ::sys::error::Error;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Closes a directory stream.
///
/// # Description
///
/// Drains all entries from the directory stream and closes the underlying file descriptor
/// (if one was allocated). Directory streams created via Hyperlight FS use fd=-1 and don't
/// need to close a real file descriptor.
///
pub fn closedir(dirp: &mut Box<DirectoryStream>) -> Result<(), Error> {
    // Drain all entries in the directory stream.
    while let Some(_posix_dirent) = dirp.pop() {}

    // Only close fd if it's a valid file descriptor (not -1).
    // Hyperlight FS-based opendir uses fd=-1 since it doesn't allocate a real fd.
    let fd: i32 = dirp.fd();
    if fd >= 0 {
        unistd::close(fd)
    } else {
        // No real fd to close - just succeed.
        Ok(())
    }
}
