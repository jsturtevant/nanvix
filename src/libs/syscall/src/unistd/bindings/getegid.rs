// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::sys_types::gid_t;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Returns the effective group ID of the calling process. In Nanvix running on Hyperlight,
/// all processes run as root (gid=0) since the guest environment is single-user.
///
/// # Returns
///
/// Always returns `0` (root group ID).
///
#[unsafe(no_mangle)]
pub extern "C" fn getegid() -> gid_t {
    ::syslog::trace!("getegid(): returning 0 (root)");
    0
}
