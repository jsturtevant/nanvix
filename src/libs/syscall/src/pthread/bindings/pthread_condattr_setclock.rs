// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    ffi::c_int,
    sys_types::{
        clockid_t,
        pthread_condattr_t,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Sets the clock attribute in a condition variable attributes object.
///
/// # Parameters
///
/// - `attr`: Pointer to the condition variable attributes object.
/// - `clock_id`: The clock identifier to set.
///
/// # Returns
///
/// Upon successful completion, returns 0. Otherwise, returns an error number.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pthread_condattr_setclock(
    _attr: *mut pthread_condattr_t,
    _clock_id: clockid_t,
) -> c_int {
    // TODO (#500): implement pthread_condattr_setclock
    ::syslog::warn!("pthread_condattr_setclock(): not implemented");
    0
}