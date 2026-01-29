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

use ::hyperlight_guest::{
    Read,
    Seek,
    SeekFrom,
    Write,
};
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
    let guest_fs_manifest_base: usize = 0x0fa88000;
    let guest_fs_manifest_size: usize = 0x00000098;
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

    // ==================== FAT Filesystem Tests ====================
    // These tests use the /data FAT mount which is writable.
    //
    // NOTE: These tests are commented out becuase
    // We use the hyperlight_guest::fs APIs directly instead of syscall bindings because
    // the current syscall implementation doesn't support file descriptor allocation for FAT
    // files. The hyperlight_guest fs module's FAT file wrapper doesn't provide a raw file
    // descriptor (unlike ReadOnly files). A proper file descriptor table would need to be
    // implemented to support open/read/write/close syscalls for FAT files.

    // Test 16: Create directory on FAT mount using mkdir.
    info!("Test 16: Testing mkdir on FAT mount");
    let testdir_path: &[u8] = b"/data/testdir\0";
    let testdir_ptr: *const c_char = testdir_path.as_ptr().cast::<c_char>();
    let mkdir_result: i32 =
        unsafe { syscall::sys::stat::bindings::mkdir::mkdir(testdir_ptr, 0o755) };
    if mkdir_result != 0 {
        error!("Test 16 FAILED: mkdir returned {}", mkdir_result);
        panic!("mkdir failed");
    }
    info!("Test 16 PASSED: mkdir created /data/testdir");

    // // Test 17: Create a file on FAT mount using open with O_CREAT.
    // info!("Test 17: Testing open with O_CREAT on FAT mount");
    // let testfile_path: &[u8] = b"/data/testfile.txt\0";
    // let testfile_ptr: *const c_char = testfile_path.as_ptr().cast::<c_char>();
    // // O_WRONLY | O_CREAT = 0x1 | 0x0200 = 0x0201
    // let fat_fd: i32 = unsafe { syscall::fcntl::bindings::open::open(testfile_ptr, 0x0201, 0o644) };
    // if fat_fd < 0 {
    //     error!("Test 17 FAILED: open with O_CREAT returned {}", fat_fd);
    //     panic!("open O_CREAT failed");
    // }
    // info!("Test 17 PASSED: created /data/testfile.txt with fd={}", fat_fd);

    // // Test 18: Write to FAT file.
    // info!("Test 18: Testing write to FAT file");
    // let write_data: &[u8] = b"Hello, FAT filesystem!";
    // let write_result: i32 = unsafe {
    //     syscall::unistd::bindings::write::write(
    //         fat_fd,
    //         write_data.as_ptr().cast::<c_void>(),
    //         write_data.len() as u32,
    //     )
    // };
    // if write_result <= 0 {
    //     error!("Test 18 FAILED: write returned {}", write_result);
    //     panic!("write to FAT file failed");
    // }
    // let bytes_written: usize = write_result as usize;
    // if bytes_written != write_data.len() {
    //     error!("Test 18 FAILED: write returned {} (expected {})", bytes_written, write_data.len());
    //     panic!("partial write to FAT file");
    // }
    // info!("Test 18 PASSED: wrote {} bytes to FAT file", bytes_written);

    // // Close the FAT file.
    // let close_fat: i32 = syscall::unistd::bindings::close::close(fat_fd);
    // if close_fat != 0 {
    //     error!("Test 18 (close): close returned {}", close_fat);
    //     panic!("close FAT file failed");
    // }

    // // Test 19: Read back the file we just wrote.
    // info!("Test 19: Testing read from FAT file");
    // // O_RDONLY = 0x0
    // let fat_fd2: i32 = unsafe { syscall::fcntl::bindings::open::open(testfile_ptr, 0, 0) };
    // if fat_fd2 < 0 {
    //     error!("Test 19 FAILED: open for read returned {}", fat_fd2);
    //     panic!("open FAT file for read failed");
    // }
    // let mut read_buf: [u8; 64] = [0; 64];
    // let read_fat_result: i32 = unsafe {
    //     syscall::unistd::bindings::read::read(
    //         fat_fd2,
    //         read_buf.as_mut_ptr().cast::<c_void>(),
    //         read_buf.len() as u32,
    //     )
    // };
    // if read_fat_result <= 0 {
    //     error!("Test 19 FAILED: read returned {}", read_fat_result);
    //     panic!("read from FAT file failed");
    // }
    // let bytes_read_fat: usize = read_fat_result as usize;
    // if bytes_read_fat != write_data.len() {
    //     error!("Test 19 FAILED: read {} bytes (expected {})", bytes_read_fat, write_data.len());
    //     panic!("read wrong number of bytes");
    // }
    // if &read_buf[..bytes_read_fat] != write_data {
    //     error!("Test 19 FAILED: data mismatch");
    //     panic!("FAT file data mismatch");
    // }
    // info!("Test 19 PASSED: read back {} bytes, data matches", bytes_read_fat);
    // let _: i32 = syscall::unistd::bindings::close::close(fat_fd2);

    // // Test 20: Rename the file.
    // info!("Test 20: Testing rename on FAT mount");
    // let renamed_path: &[u8] = b"/data/renamed.txt\0";
    // let renamed_ptr: *const c_char = renamed_path.as_ptr().cast::<c_char>();
    // let rename_result: i32 =
    //     unsafe { syscall::fcntl::bindings::rename::rename(testfile_ptr, renamed_ptr) };
    // if rename_result != 0 {
    //     error!("Test 20 FAILED: rename returned {}", rename_result);
    //     panic!("rename failed");
    // }
    // info!("Test 20 PASSED: renamed file to /data/renamed.txt");

    // // Verify the old name no longer exists.
    // let old_fd: i32 = unsafe { syscall::fcntl::bindings::open::open(testfile_ptr, 0, 0) };
    // if old_fd >= 0 {
    //     let _: i32 = syscall::unistd::bindings::close::close(old_fd);
    //     error!("Test 20: old filename still exists after rename");
    //     panic!("rename did not remove old name");
    // }

    // // Verify new name exists and has correct content.
    // let new_fd: i32 = unsafe { syscall::fcntl::bindings::open::open(renamed_ptr, 0, 0) };
    // if new_fd < 0 {
    //     error!("Test 20: cannot open renamed file");
    //     panic!("renamed file not found");
    // }
    // let _: i32 = syscall::unistd::bindings::close::close(new_fd);
    // info!("Test 20: verified renamed file exists");

    // // Test 21: Unlink (delete) the file.
    // info!("Test 21: Testing unlink on FAT mount");
    // let unlink_result: i32 = unsafe { syscall::unistd::bindings::unlink::unlink(renamed_ptr) };
    // if unlink_result != 0 {
    //     error!("Test 21 FAILED: unlink returned {}", unlink_result);
    //     panic!("unlink failed");
    // }
    // info!("Test 21 PASSED: unlinked /data/renamed.txt");

    // // Verify the file is gone.
    // let gone_fd: i32 = unsafe { syscall::fcntl::bindings::open::open(renamed_ptr, 0, 0) };
    // if gone_fd >= 0 {
    //     let _: i32 = syscall::unistd::bindings::close::close(gone_fd);
    //     error!("Test 21: file still exists after unlink");
    //     panic!("unlink did not remove file");
    // }

    // ==================== FAT File I/O Tests Using Hyperlight APIs ====================
    // These tests use hyperlight_guest::fs APIs directly to test FAT file operations.

    // Test 17: Create and write a file on FAT mount using hyperlight_guest fs APIs.
    info!("Test 17: Create and write file using hyperlight_guest fs APIs");
    {
        let test_content: &[u8] = b"Hello from Nanvix FAT test!\nThis is line 2.\n";
        let test_file_path: &str = "/data/testdir/test_file.txt";

        // Create and write to a new file on the FAT mount.
        let file_result: Result<hyperlight_guest::fs::File, hyperlight_guest::fs::FsError> =
            hyperlight_guest::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .open(test_file_path);

        match file_result {
            Ok(mut file) => {
                let write_result: Result<usize, hyperlight_guest::fs::FsError> =
                    file.write(test_content);
                match write_result {
                    Ok(bytes_written) => {
                        info!(
                            "Test 17 PASSED: wrote {} bytes to {}",
                            bytes_written, test_file_path
                        );
                        // Flush to ensure data is written.
                        if let Err(e) = file.flush() {
                            error!("Test 17 WARNING: flush failed: {:?}", e);
                        }
                    },
                    Err(e) => {
                        error!("Test 17 FAILED: write failed: {:?}", e);
                        panic!("FAT file write failed");
                    },
                }
            },
            Err(e) => {
                error!("Test 17 FAILED: failed to create file: {:?}", e);
                panic!("FAT file create failed");
            },
        }
    }

    // Test 18: Read the file back using hyperlight_guest fs APIs.
    info!("Test 18: Read file using hyperlight_guest fs APIs");
    {
        let test_file_path: &str = "/data/testdir/test_file.txt";
        let expected_content: &[u8] = b"Hello from Nanvix FAT test!\nThis is line 2.\n";

        let file_result: Result<hyperlight_guest::fs::File, hyperlight_guest::fs::FsError> =
            hyperlight_guest::fs::OpenOptions::new()
                .read(true)
                .open(test_file_path);

        match file_result {
            Ok(mut file) => {
                let mut read_buf: [u8; 128] = [0u8; 128];
                let read_result: Result<usize, hyperlight_guest::fs::FsError> =
                    file.read(&mut read_buf);
                match read_result {
                    Ok(bytes_read) => {
                        info!("Test 18: read {} bytes from {}", bytes_read, test_file_path);
                        // Print content to stdout.
                        unistd::write(STDOUT_FILENO, &read_buf[..bytes_read])?;
                        // Verify content matches.
                        if bytes_read == expected_content.len()
                            && &read_buf[..bytes_read] == expected_content
                        {
                            info!("Test 18 PASSED: content matches expected");
                        } else {
                            error!(
                                "Test 18 FAILED: content mismatch (read {} bytes, expected {})",
                                bytes_read,
                                expected_content.len()
                            );
                            panic!("FAT file content mismatch");
                        }
                    },
                    Err(e) => {
                        error!("Test 18 FAILED: read failed: {:?}", e);
                        panic!("FAT file read failed");
                    },
                }
            },
            Err(e) => {
                error!("Test 18 FAILED: failed to open file for reading: {:?}", e);
                panic!("FAT file open for read failed");
            },
        }
    }

    // Test 19: Test seek operations on FAT file.
    info!("Test 19: Test seek operations on FAT file");
    {
        let test_file_path: &str = "/data/testdir/test_file.txt";

        let file_result: Result<hyperlight_guest::fs::File, hyperlight_guest::fs::FsError> =
            hyperlight_guest::fs::OpenOptions::new()
                .read(true)
                .open(test_file_path);

        match file_result {
            Ok(mut file) => {
                // Seek to end to get file size.
                let seek_end_result: Result<u64, hyperlight_guest::fs::FsError> =
                    file.seek(SeekFrom::End(0));
                match seek_end_result {
                    Ok(file_size) => {
                        info!("Test 19: file size = {} bytes", file_size);

                        // Seek back to beginning.
                        let seek_start_result: Result<u64, hyperlight_guest::fs::FsError> =
                            file.seek(SeekFrom::Start(0));
                        match seek_start_result {
                            Ok(pos) => {
                                if pos == 0 {
                                    info!("Test 19 PASSED: seek operations work correctly");
                                } else {
                                    error!(
                                        "Test 19 FAILED: seek to start returned {} (expected 0)",
                                        pos
                                    );
                                }
                            },
                            Err(e) => {
                                error!("Test 19 FAILED: seek to start failed: {:?}", e);
                            },
                        }
                    },
                    Err(e) => {
                        error!("Test 19 FAILED: seek to end failed: {:?}", e);
                    },
                }
            },
            Err(e) => {
                error!("Test 19 FAILED: failed to open file for seek test: {:?}", e);
            },
        }
    }

    // Test 20: Append to existing file.
    info!("Test 20: Append to existing FAT file");
    {
        let test_file_path: &str = "/data/testdir/test_file.txt";
        let append_content: &[u8] = b"Appended line 3.\n";

        // Open for read+write to append.
        let file_result: Result<hyperlight_guest::fs::File, hyperlight_guest::fs::FsError> =
            hyperlight_guest::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(test_file_path);

        match file_result {
            Ok(mut file) => {
                // Seek to end.
                if let Err(e) = file.seek(SeekFrom::End(0)) {
                    error!("Test 20 FAILED: seek to end failed: {:?}", e);
                    panic!("seek failed");
                }

                // Write appended content.
                let write_result: Result<usize, hyperlight_guest::fs::FsError> =
                    file.write(append_content);
                match write_result {
                    Ok(bytes_written) => {
                        info!("Test 20 PASSED: appended {} bytes", bytes_written);
                        let _: Result<(), hyperlight_guest::fs::FsError> = file.flush();
                    },
                    Err(e) => {
                        error!("Test 20 FAILED: append write failed: {:?}", e);
                        panic!("append failed");
                    },
                }
            },
            Err(e) => {
                error!("Test 20 FAILED: failed to open file for append: {:?}", e);
                panic!("open for append failed");
            },
        }
    }

    // Test 21: Verify appended content.
    info!("Test 21: Verify appended content");
    {
        let test_file_path: &str = "/data/testdir/test_file.txt";
        let expected_full_content: &[u8] =
            b"Hello from Nanvix FAT test!\nThis is line 2.\nAppended line 3.\n";

        let file_result: Result<hyperlight_guest::fs::File, hyperlight_guest::fs::FsError> =
            hyperlight_guest::fs::OpenOptions::new()
                .read(true)
                .open(test_file_path);

        match file_result {
            Ok(mut file) => {
                let mut read_buf: [u8; 256] = [0u8; 256];
                let read_result: Result<usize, hyperlight_guest::fs::FsError> =
                    file.read(&mut read_buf);
                match read_result {
                    Ok(bytes_read) => {
                        // Print content to stdout.
                        unistd::write(STDOUT_FILENO, &read_buf[..bytes_read])?;
                        if bytes_read == expected_full_content.len()
                            && &read_buf[..bytes_read] == expected_full_content
                        {
                            info!("Test 21 PASSED: full content verified ({} bytes)", bytes_read);
                        } else {
                            error!(
                                "Test 21 FAILED: content mismatch (read {} bytes, expected {})",
                                bytes_read,
                                expected_full_content.len()
                            );
                            // Print what we got for debugging.
                            if let Ok(s) = core::str::from_utf8(&read_buf[..bytes_read]) {
                                error!("Got: {:?}", s);
                            }
                            panic!("content verification failed");
                        }
                    },
                    Err(e) => {
                        error!("Test 21 FAILED: read failed: {:?}", e);
                        panic!("read failed");
                    },
                }
            },
            Err(e) => {
                error!("Test 21 FAILED: failed to open file: {:?}", e);
                panic!("open failed");
            },
        }
    }

    // Test 24: Test that write to read-only file returns EROFS.
    info!("Test 24: Testing write to read-only file");
    let ro_fd: i32 = unsafe { syscall::fcntl::bindings::open::open(pathname, 0, 0) };
    if ro_fd >= 0 {
        let test_write_data: &[u8] = b"test write data";
        let ro_write_result: i32 = unsafe {
            syscall::unistd::bindings::write::write(
                ro_fd,
                test_write_data.as_ptr().cast::<c_void>(),
                test_write_data.len() as u32,
            )
        };
        if ro_write_result >= 0 {
            error!("Test 24 FAILED: write to read-only file succeeded ({} bytes)", ro_write_result);
            panic!("write to read-only file should fail");
        }
        info!(
            "Test 24 PASSED: write to read-only file returned {} (expected error)",
            ro_write_result
        );
        let _: i32 = syscall::unistd::bindings::close::close(ro_fd);
    } else {
        error!("Test 24 FAILED: could not open file for read-only test");
        panic!("open failed for read-only test");
    }

    // Test 22: Unlink (delete) the test file before removing the directory.
    info!("Test 22: Testing unlink on FAT mount");
    let testfile_path: &[u8] = b"/data/testdir/test_file.txt\0";
    let testfile_ptr: *const c_char = testfile_path.as_ptr().cast::<c_char>();
    let unlink_result: i32 = unsafe { syscall::unistd::bindings::unlink::unlink(testfile_ptr) };
    if unlink_result != 0 {
        error!("Test 22 FAILED: unlink returned {}", unlink_result);
        panic!("unlink failed");
    }
    info!("Test 22 PASSED: unlinked /data/testdir/test_file.txt");

    // Test 23: Remove the directory we created.
    info!("Test 23: Testing rmdir on FAT mount");
    let rmdir_result: i32 = unsafe { syscall::unistd::bindings::rmdir::rmdir(testdir_ptr) };
    if rmdir_result != 0 {
        error!("Test 23 FAILED: rmdir returned {}", rmdir_result);
        panic!("rmdir failed");
    }
    info!("Test 23 PASSED: removed /data/testdir");

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
