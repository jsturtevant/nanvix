// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![no_std]
#![no_main]
#![deny(clippy::all)]
#![deny(clippy::as_conversions)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

//==================================================================================================
// Modules
//==================================================================================================

/// Must come first.
extern crate alloc;
extern crate libc_string;
extern crate nvx;

// mod safe;
// mod r#unsafe;
use ::syslog::error;

//==================================================================================================
// Imports
//==================================================================================================

use ::core::ffi::{
    c_char,
    c_void,
};
use ::sys::error::Error;
use ::sysapi::unistd::STDOUT_FILENO;
use ::syscall::unistd;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    let guest_fs_manifest_base: usize = 0x076e3000;
    let guest_fs_manifest_size: usize = 0x00000058;

    // Dump contents of the manifest
    syscall::init(guest_fs_manifest_base, guest_fs_manifest_size);

    let pathname_buf: &[u8] = b"/README.md\0";
    let pathname: *const c_char = pathname_buf.as_ptr().cast::<c_char>();

    // Print file contents available through the hyperlight guest filesystem.
    let mut buf: [u8; 64] = [0; 64];
    let fd: i32 = unsafe { syscall::fcntl::bindings::open::open(pathname, 0, 0) };
    if fd < 0 {
        error!("failed to open file descriptor: {}", fd);
        panic!("failed to open file descriptor: {}", fd);
    }
    loop {
        let read_result: i32 = unsafe {
            syscall::unistd::bindings::read::read(
                fd,
                buf.as_mut_ptr().cast::<c_void>(),
                buf.len() as u32,
            )
        };
        if read_result == 0 {
            break;
        }
        if read_result < 0 {
            error!("failed to read file: {}", read_result);
            panic!("failed to read file: {}", read_result);
        }
        let bytes_read: usize = read_result as usize;
        unistd::write(STDOUT_FILENO, &buf[..bytes_read])?;
    }

    // Magic string.
    {
        let magic_string: &[u8] = "ok 2\n".as_bytes();
        unistd::write(STDOUT_FILENO, magic_string)?;
    }

    Ok(())
}
