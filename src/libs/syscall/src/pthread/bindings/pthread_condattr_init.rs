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
// pthread_condattr_init
//==================================================================================================

/// Initializes a condition variable attributes object.
///
/// TODO (#499): implement this function.
#[no_mangle]
pub unsafe extern "C" fn pthread_condattr_init(_attr: *mut pthread_condattr_t) -> c_int {
    // TODO (#499): implement pthread_condattr_init
    ::syslog::warn!("pthread_condattr_init(): not implemented");
    0
}