// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::string::String;
use ::sys::error::{
    Error,
    ErrorCode,
};
use hyperlight_guest::fs;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Gets the current working directory using `hyperlight_guest::fs::cwd()`.
///
/// # Returns
///
/// Upon successful completion, returns the current working directory as a String.
/// Otherwise, an error is returned.
///
pub fn getcwd() -> Result<String, Error> {
    ::syslog::trace!("getcwd()");

    match fs::cwd() {
        Ok(cwd) => {
            ::syslog::trace!("getcwd(): cwd={:?}", cwd);
            Ok(cwd)
        },
        Err(e) => {
            ::syslog::error!("getcwd(): failed: {:?}", e);
            Err(Error::new(ErrorCode::IoErr, "getcwd() failed"))
        },
    }
}
