// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]
#![no_main]

//==================================================================================================
// Imports
//==================================================================================================

extern crate libc_string;
extern crate nvx;

use ::sys::{
    error::Error,
    kcall::debug,
};
use ::syslog::{
    error,
    info,
};
use hyperlight_guest::{
    fs,
    Read,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    let msg: &str = "Hello, world from Rust!\n";
    let _ = debug::debug(msg.as_ptr(), msg.len());

    let guest_fs_manifest_base: usize = 0x076e3000;
    let guest_fs_manifest_size: usize = 0x00000058;

    // Dump contents of the manifest

    unsafe {
        if let Err(e) = hyperlight_guest::fs::init(
            guest_fs_manifest_base as *const u8,
            guest_fs_manifest_size as usize,
        ) {
            let reason: &str = "failed to initialize guest filesystem";
            panic!("parse_bootinfo(): {reason}: {e}");
        }
    }

    // Print file contents available through the hyperlight guest filesystem.
    let mut buf: [u8; 64] = [0; 64];
    let mut file = match fs::open("/README.md") {
        Ok(f) => f,
        Err(e) => {
            error!("failed to open file: {:?}", e);
            panic!("failed to open file: {:?}", e);
        },
    };
    loop {
        match file.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => match core::str::from_utf8(&buf[..n]) {
                Ok(text) => info!("file chunk: {}", text.trim_end_matches('\n')),
                Err(_) => info!("file chunk (hex): {:02x?}", &buf[..n]),
            },
            Err(e) => {
                error!("failed to read file: {:?}", e);
                panic!("failed to read file: {:?}", e);
            },
        }
    }

    Ok(())
}
