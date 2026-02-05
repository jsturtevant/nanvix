// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::platform::Platform,
    kcall::{
        KcallArgs,
        KcallResult,
    },
    pm::ProcessManager,
};
use ::sys::{
    error::Error,
    mm::VirtualAddress,
    pm::ProcessIdentifier,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Gets the filesystem manifest information.
fn do_get_fs_manifest(
    pm: &mut ProcessManager,
    pid: ProcessIdentifier,
    base_addr: VirtualAddress,
    size_addr: VirtualAddress,
) -> Result<(), Error> {
    trace!("pid={pid:?}, base_addr={base_addr:?}, size_addr={size_addr:?}");

    // Get filesystem manifest info from platform.
    let (base, size): (usize, usize) = Platform::get_fs_manifest_info();

    trace!("do_get_fs_manifest(): base={:#x}, size={:#x}", base, size);

    // Copy base address to user space.
    pm.vmcopy_to_user(
        pid,
        base_addr,
        VirtualAddress::new(&base as *const usize as usize),
        ::core::mem::size_of::<usize>(),
    )?;

    // Copy size to user space.
    pm.vmcopy_to_user(
        pid,
        size_addr,
        VirtualAddress::new(&size as *const usize as usize),
        ::core::mem::size_of::<usize>(),
    )?;

    Ok(())
}

///
/// # Description
///
/// Gets the filesystem manifest information.
///
/// # Parameters
///
/// - `pm`: A mutable reference to the process manager.
/// - `args.arg0`: The address where the manifest base address should be stored.
/// - `args.arg1`: The address where the manifest size should be stored.
///
/// # Returns
///
/// If successful, returns `KcallResult::Success`. Otherwise it returns a
/// `KcallResult::Error` to indicate the error.
///
pub fn get_fs_manifest(pm: &mut ProcessManager, args: &KcallArgs) -> KcallResult {
    // Unpack kernel call arguments.
    let base_addr: VirtualAddress = VirtualAddress::new(args.arg0 as usize);
    let size_addr: VirtualAddress = VirtualAddress::new(args.arg1 as usize);

    // Get filesystem manifest and parse result.
    match do_get_fs_manifest(pm, args.pid, base_addr, size_addr) {
        Ok(()) => KcallResult::ok(),
        Err(error) => KcallResult::Error(error.code.into()),
    }
}
