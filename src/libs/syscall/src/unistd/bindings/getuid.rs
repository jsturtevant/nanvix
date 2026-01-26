// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::sys_types::uid_t;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Returns the real user ID of the calling process. In Nanvix running on Hyperlight,
/// all processes run as root (uid=0) since the guest environment is single-user.
///
/// # Returns
///
/// Always returns `0` (root user ID).
///
#[unsafe(no_mangle)]
pub extern "C" fn getuid() -> uid_t {
    ::syslog::trace!("getuid(): returning 0 (root)");
    0
}
