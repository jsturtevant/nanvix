// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    ffi::c_int,
    sys_types::pthread_condattr_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Destroys a condition variable attributes object.
///
/// # Parameters
///
/// - `attr`: Pointer to the condition variable attributes object to destroy.
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
pub unsafe extern "C" fn pthread_condattr_destroy(_attr: *mut pthread_condattr_t) -> c_int {
    // TODO (#496): implement pthread_condattr_destroy
    ::syslog::warn!("pthread_condattr_destroy(): not implemented");
    0
}