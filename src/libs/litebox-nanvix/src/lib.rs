// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![no_std]
#![allow(unused_variables)]

//==================================================================================================
// Imports
//==================================================================================================

use core::{
    ops::Range,
    panic,
};
use posix::nvx::{
    self,
    mm::{
        AccessPermission,
        VirtualAddress,
    },
    pm::ProcessIdentifier,
    sys::{
        arch::mem,
        kcall,
    },
};

use litebox::platform::{
    page_mgmt::MemoryRegionPermissions,
    trivial_providers::TransparentMutPtr,
};

extern crate alloc;

//==================================================================================================
// Modules
//==================================================================================================

pub mod exit;
pub mod instant;
pub mod mutex;
pub mod network;
pub mod stdio;

//==================================================================================================

pub struct NanvixUserland;

impl litebox::platform::RawPointerProvider for NanvixUserland {
    type RawConstPointer<T: Clone> = litebox::platform::trivial_providers::TransparentConstPtr<T>;
    type RawMutPointer<T: Clone> = litebox::platform::trivial_providers::TransparentMutPtr<T>;
}

impl<const ALIGN: usize> litebox::platform::PageManagementProvider<ALIGN> for NanvixUserland {
    fn allocate_pages(
        &self,
        range: Range<usize>,
        initial_permissions: MemoryRegionPermissions,
        can_grow_down: bool,
    ) -> Result<Self::RawMutPointer<u8>, litebox::platform::page_mgmt::AllocationError> {
        nvx::trace!("allocate_pages(): range={:X?}", range);
        if can_grow_down {
            // TODO: implement this functionality.
            nvx::debug!("allocate_pages(): can_grow_down is not supported");
        }

        // Check if range is page-aligned.
        if range.start % mem::PAGE_SIZE != 0 {
            return Err(litebox::platform::page_mgmt::AllocationError::Unaligned);
        }
        if range.end % mem::PAGE_SIZE != 0 {
            return Err(litebox::platform::page_mgmt::AllocationError::Unaligned);
        }

        let start: usize = range.start;
        let end: usize = range.end;
        let pid: ProcessIdentifier = kcall::pm::getpid().unwrap();
        for vaddr in (start..end).step_by(mem::PAGE_SIZE) {
            debug_assert!(vaddr != end);

            // FIXME: do not use 0b111u8 as permission.
            let permission: AccessPermission = initial_permissions.bits().try_into().unwrap();
            let permission: AccessPermission = 0b111u8.try_into().unwrap();

            // Attempt to map page.
            let vaddr: VirtualAddress = VirtualAddress::new(vaddr);
            if let Err(error) = kcall::mm::mmap(pid, vaddr, permission) {
                return Err(litebox::platform::page_mgmt::AllocationError::OutOfMemory);
            }

            // FIXME: we are leaking memory if we fail.

            // NOTE: pages allocated with mmap() are always zeroed.
        }

        Ok(TransparentMutPtr {
            inner: range.start as *mut u8,
        })
    }

    unsafe fn deallocate_pages(
        &self,
        range: Range<usize>,
    ) -> Result<(), litebox::platform::page_mgmt::DeallocationError> {
        // Check if range is page-aligned.
        if range.start % mem::PAGE_SIZE != 0 {
            return Err(litebox::platform::page_mgmt::DeallocationError::Unaligned);
        }
        if range.end % mem::PAGE_SIZE != 0 {
            return Err(litebox::platform::page_mgmt::DeallocationError::Unaligned);
        }

        let pid: ProcessIdentifier = kcall::pm::getpid().unwrap();
        let start: usize = range.start;
        let end: usize = range.end;
        let ret: Result<(), litebox::platform::page_mgmt::DeallocationError> = Ok(());

        for vaddr in (start..end).step_by(mem::PAGE_SIZE) {
            debug_assert!(vaddr != end);

            let vaddr: VirtualAddress = VirtualAddress::from_raw_value(vaddr);

            if let Err(error) = kcall::mm::munmap(pid, vaddr) {
                // Save error.
                panic!(
                    "unmap_range(): failed to unmap page at {:X?}, skipping (error={:?})",
                    vaddr, error
                );
            }
        }

        ret
    }

    unsafe fn remap_pages(
        &self,
        old_range: Range<usize>,
        new_range: Range<usize>,
    ) -> Result<(), litebox::platform::page_mgmt::RemapError> {
        unimplemented!("remap_pages() not implemented")
    }

    unsafe fn update_permissions(
        &self,
        range: Range<usize>,
        new_permissions: MemoryRegionPermissions,
    ) -> Result<(), litebox::platform::page_mgmt::PermissionUpdateError> {
        // TODO: implement this function.
        Ok(())
    }
}
