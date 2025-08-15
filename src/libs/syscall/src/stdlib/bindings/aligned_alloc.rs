// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    ffi::c_void,
    sys_types::c_size_t,
};

use ::alloc::alloc::{
    alloc as rs_alloc,
    dealloc as rs_dealloc,
    Layout,
};
use ::core::{
    mem,
    ptr,
};

// We use the global Rust allocator via alloc::alloc::{alloc,dealloc}.

//==================================================================================================
// Internal Types
//==================================================================================================

#[repr(C)]
struct Header {
    base: *mut u8,
    alloc_size: usize,
    alloc_align: usize,
}

impl Header {
    #[inline]
    fn layout(&self) -> Option<Layout> {
        Layout::from_size_align(self.alloc_size, self.alloc_align).ok()
    }
}

//==================================================================================================
// Helpers
//==================================================================================================

#[inline]
fn is_power_of_two(x: usize) -> bool {
    x != 0 && (x & (x - 1)) == 0
}

//==================================================================================================
// C Bindings
//==================================================================================================

///
/// Allocates a block of memory with the requested alignment and size.
/// Returns a null pointer on failure. Memory must be freed with `__aligned_free`.
///
/// Safety: C ABI entry point.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __aligned_malloc(size: c_size_t, alignment: c_size_t) -> *mut c_void {
    ::syslog::trace!("__aligned_alloc(): alignment={alignment:?}, size={size:?}");

    let align: usize = alignment as usize;
    let size: usize = size as usize;

    if size == 0 || !is_power_of_two(align) {
        ::syslog::warn!("__aligned_alloc(): invalid args alignment={align}, size={size}");
        return ptr::null_mut();
    }

    // We will return a pointer aligned to `align` with space for a header
    // immediately before it. To do so, allocate extra space for header and
    // potential alignment padding, then choose an aligned user pointer.
    let header_size = mem::size_of::<Header>();
    let alloc_align = core::cmp::max(align, mem::align_of::<Header>());

    let total_size = match header_size
        .checked_add(size)
        .and_then(|s| s.checked_add(alloc_align - 1))
    {
        Some(s) => s,
        None => return ptr::null_mut(),
    };

    let layout = match Layout::from_size_align(total_size, alloc_align) {
        Ok(l) => l,
        Err(_) => return ptr::null_mut(),
    };

    let raw = rs_alloc(layout);
    if raw.is_null() {
        return ptr::null_mut();
    }

    // Compute an aligned user pointer such that there is room for the header
    // immediately before it within the allocated block.
    let start = raw.add(header_size) as usize;
    let mask = alloc_align - 1;
    let aligned_user = ((start + mask) & !mask) as *mut u8;

    // Place header right before the user pointer.
    let header_ptr = (aligned_user as *mut u8).sub(header_size) as *mut Header;
    ptr::write(
        header_ptr,
        Header {
            base: raw,
            alloc_size: total_size,
            alloc_align,
        },
    );

    aligned_user as *mut c_void
}

/// Frees memory allocated by `__aligned_alloc`.
/// Accepts null pointers (no-op).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __aligned_free(ptr: *mut c_void) {
    ::syslog::trace!("__aligned_free(): ptr={ptr:?}");

    if ptr.is_null() {
        return;
    }

    let header_size = mem::size_of::<Header>();

    let header_addr = (ptr as *mut u8).sub(header_size);
    let header = &*(header_addr as *const Header);

    if let Some(layout) = header.layout() {
        // Safety: header.base was the original allocation pointer, matching `layout`.
        rs_dealloc(header.base, layout);
    } else {
        // Layout reconstruction failed; avoid undefined behavior by not freeing.
        ::syslog::error!("__aligned_free(): invalid header, memory leak");
    }
}
