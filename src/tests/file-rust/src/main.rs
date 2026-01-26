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

use ::syslog::{
    error,
    info,
};

//==================================================================================================
// Imports
//==================================================================================================

use ::core::ffi::{
    c_char,
    c_void,
};
use ::sys::error::Error;
use ::sysapi::{
    sys_stat::stat,
    sys_types::{
        gid_t,
        uid_t,
    },
    unistd::{
        STDOUT_FILENO,
        file_seek::{
            SEEK_CUR,
            SEEK_END,
            SEEK_SET,
        },
    },
};
use ::syscall::unistd;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() -> Result<(), Error> {
    let guest_fs_manifest_base: usize = 0x0fa13000;
    let guest_fs_manifest_size: usize = 0x00000058;
    info!(
        "guest_fs_manifest_base={:#x}, guest_fs_manifest_size={:#x}",
        guest_fs_manifest_base, guest_fs_manifest_size
    );
    // Initialize the hyperlight guest filesystem.
    syscall::init(guest_fs_manifest_base, guest_fs_manifest_size);

    let pathname_buf: &[u8] = b"/README.md\0";
    let pathname: *const c_char = pathname_buf.as_ptr().cast::<c_char>();

    // Test 1: Open file using syscall binding.
    info!("Test 1: Opening file via syscall binding");
    let fd: i32 = unsafe { syscall::fcntl::bindings::open::open(pathname, 0, 0) };
    if fd < 0 {
        error!("Test 1 FAILED: failed to open file descriptor: {}", fd);
        panic!("failed to open file descriptor: {}", fd);
    }
    info!("Test 1 PASSED: opened file with fd={}", fd);

    // Test 2: Read first chunk of the file.
    info!("Test 2: Reading file via syscall binding");
    let mut buf: [u8; 64] = [0; 64];
    let read_result: i32 = unsafe {
        syscall::unistd::bindings::read::read(
            fd,
            buf.as_mut_ptr().cast::<c_void>(),
            buf.len() as u32,
        )
    };
    if read_result <= 0 {
        error!("Test 2 FAILED: failed to read file: {}", read_result);
        panic!("failed to read file: {}", read_result);
    }
    let bytes_read: usize = read_result as usize;
    info!("Test 2 PASSED: read {} bytes", bytes_read);

    // Test 3: Test lseek with SEEK_SET to beginning.
    info!("Test 3: Testing lseek SEEK_SET");
    let seek_result: i64 = unsafe { syscall::unistd::bindings::lseek::lseek(fd, 0, SEEK_SET) };
    if seek_result != 0 {
        error!("Test 3 FAILED: lseek SEEK_SET to 0 returned {}", seek_result);
        panic!("lseek SEEK_SET failed");
    }
    info!("Test 3 PASSED: lseek SEEK_SET returned {}", seek_result);

    // Test 4: Test lseek with SEEK_END.
    info!("Test 4: Testing lseek SEEK_END");
    let seek_end_result: i64 = unsafe { syscall::unistd::bindings::lseek::lseek(fd, 0, SEEK_END) };
    if seek_end_result < 0 {
        error!("Test 4 FAILED: lseek SEEK_END returned {}", seek_end_result);
        panic!("lseek SEEK_END failed");
    }
    info!("Test 4 PASSED: lseek SEEK_END returned {} (file size)", seek_end_result);

    // Test 5: Test lseek with SEEK_CUR.
    info!("Test 5: Testing lseek SEEK_CUR");
    // First seek back to beginning.
    let _: i64 = unsafe { syscall::unistd::bindings::lseek::lseek(fd, 0, SEEK_SET) };
    // Now seek forward 10 bytes from current position.
    let seek_cur_result: i64 = unsafe { syscall::unistd::bindings::lseek::lseek(fd, 10, SEEK_CUR) };
    if seek_cur_result != 10 {
        error!("Test 5 FAILED: lseek SEEK_CUR returned {} (expected 10)", seek_cur_result);
        panic!("lseek SEEK_CUR failed");
    }
    info!("Test 5 PASSED: lseek SEEK_CUR returned {}", seek_cur_result);

    // Test 6: Test fstat.
    info!("Test 6: Testing fstat");
    let mut statbuf: stat = stat::default();
    let fstat_result: i32 = unsafe { syscall::sys::stat::bindings::fstat::fstat(fd, &mut statbuf) };
    if fstat_result != 0 {
        error!("Test 6 FAILED: fstat returned {}", fstat_result);
        panic!("fstat failed");
    }
    info!("Test 6 PASSED: fstat returned 0, st_size={}", { statbuf.st_size });

    // Test 7: Test stat on the file path.
    info!("Test 7: Testing stat");
    let mut stat_pathbuf: stat = stat::default();
    let stat_result: i32 =
        unsafe { syscall::sys::stat::bindings::stat::stat(pathname, &mut stat_pathbuf) };
    if stat_result != 0 {
        error!("Test 7 FAILED: stat returned {}", stat_result);
        panic!("stat failed");
    }
    info!("Test 7 PASSED: stat returned 0, st_size={}", { stat_pathbuf.st_size });

    // Test 8: Test close.
    info!("Test 8: Testing close");
    let close_result: i32 = syscall::unistd::bindings::close::close(fd);
    if close_result != 0 {
        error!("Test 8 FAILED: close returned {}", close_result);
        panic!("close failed");
    }
    info!("Test 8 PASSED: close returned 0");

    // Test 9: Test getcwd.
    info!("Test 9: Testing getcwd");
    let mut cwd_buf: [u8; 256] = [0; 256];
    let cwd_ptr: *mut c_char = unsafe {
        syscall::unistd::bindings::getcwd::getcwd(
            cwd_buf.as_mut_ptr().cast::<c_char>(),
            cwd_buf.len() as u32,
        )
    };
    if cwd_ptr.is_null() {
        error!("Test 9 FAILED: getcwd returned null");
        panic!("getcwd failed");
    }
    // Find length of cwd string.
    let mut cwd_len: usize = 0;
    for i in 0..cwd_buf.len() {
        if cwd_buf[i] == 0 {
            cwd_len = i;
            break;
        }
    }
    if let Ok(cwd_str) = core::str::from_utf8(&cwd_buf[..cwd_len]) {
        info!("Test 9 PASSED: getcwd returned {:?}", cwd_str);
    } else {
        info!("Test 9 PASSED: getcwd returned (non-utf8 data)");
    }

    // Test 10: Test isatty.
    info!("Test 10: Testing isatty");
    let isatty_stdout: i32 = unsafe { syscall::unistd::bindings::isatty::isatty(STDOUT_FILENO) };
    info!("Test 10: isatty(stdout)={}", isatty_stdout);
    // Re-open a file and test isatty on it.
    let fd2: i32 = unsafe { syscall::fcntl::bindings::open::open(pathname, 0, 0) };
    if fd2 >= 0 {
        let isatty_file: i32 = unsafe { syscall::unistd::bindings::isatty::isatty(fd2) };
        info!("Test 10: isatty(file fd={})={}", fd2, isatty_file);
        if isatty_stdout == 1 && isatty_file == 0 {
            info!("Test 10 PASSED: isatty correctly identifies terminals");
        } else {
            error!(
                "Test 10 FAILED: unexpected isatty results (stdout={}, file={})",
                isatty_stdout, isatty_file
            );
        }
        // Close the file.
        let _: i32 = syscall::unistd::bindings::close::close(fd2);
    } else {
        error!("Test 10 FAILED: could not re-open file for isatty test");
    }

    // Test 11: Test getuid.
    info!("Test 11: Testing getuid");
    let uid: uid_t = syscall::unistd::bindings::getuid::getuid();
    if uid == 0 {
        info!("Test 11 PASSED: getuid returned {} (root)", uid);
    } else {
        error!("Test 11 FAILED: getuid returned {} (expected 0)", uid);
        panic!("getuid failed");
    }

    // Test 12: Test getgid.
    info!("Test 12: Testing getgid");
    let gid: gid_t = syscall::unistd::bindings::getgid::getgid();
    if gid == 0 {
        info!("Test 12 PASSED: getgid returned {} (root)", gid);
    } else {
        error!("Test 12 FAILED: getgid returned {} (expected 0)", gid);
        panic!("getgid failed");
    }

    // Test 13: Test geteuid.
    info!("Test 13: Testing geteuid");
    let euid: uid_t = syscall::unistd::bindings::geteuid::geteuid();
    if euid == 0 {
        info!("Test 13 PASSED: geteuid returned {} (root)", euid);
    } else {
        error!("Test 13 FAILED: geteuid returned {} (expected 0)", euid);
        panic!("geteuid failed");
    }

    // Test 14: Test getegid.
    info!("Test 14: Testing getegid");
    let egid: gid_t = syscall::unistd::bindings::getegid::getegid();
    if egid == 0 {
        info!("Test 14 PASSED: getegid returned {} (root)", egid);
    } else {
        error!("Test 14 FAILED: getegid returned {} (expected 0)", egid);
        panic!("getegid failed");
    }

    // Test 15: Verify uid/gid consistency (real and effective should match).
    info!("Test 15: Testing uid/gid consistency");
    if uid == euid && gid == egid {
        info!("Test 15 PASSED: real and effective uid/gid are consistent");
    } else {
        error!(
            "Test 15 FAILED: uid/gid mismatch (uid={}, euid={}, gid={}, egid={})",
            uid, euid, gid, egid
        );
        panic!("uid/gid consistency check failed");
    }

    // Print file contents (full read test).
    info!("Reading full file contents:");
    let fd3: i32 = unsafe { syscall::fcntl::bindings::open::open(pathname, 0, 0) };
    if fd3 >= 0 {
        loop {
            let read_result2: i32 = unsafe {
                syscall::unistd::bindings::read::read(
                    fd3,
                    buf.as_mut_ptr().cast::<c_void>(),
                    buf.len() as u32,
                )
            };
            if read_result2 == 0 {
                break;
            }
            if read_result2 < 0 {
                error!("failed to read file: {}", read_result2);
                panic!("failed to read file: {}", read_result2);
            }
            let bytes_read2: usize = read_result2 as usize;
            unistd::write(STDOUT_FILENO, &buf[..bytes_read2])?;
        }
        let _: i32 = syscall::unistd::bindings::close::close(fd3);
    }

    // Magic string to indicate success.
    {
        let magic_string: &[u8] = "\nok 2\n".as_bytes();
        unistd::write(STDOUT_FILENO, magic_string)?;
    }

    Ok(())
}
