// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

// TODO: add description
#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pthread_atfork(
    _prepare: Option<extern "C" fn()>,
    _parent: Option<extern "C" fn()>,
    _child: Option<extern "C" fn()>,
) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/483
    ::syslog::warn!("pthread_atfork(): not implemented");
    0
}
