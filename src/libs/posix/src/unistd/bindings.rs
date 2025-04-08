// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::errno,
    ffi::{
        c_char,
        c_int,
        c_long,
        c_uint,
        c_void,
    },
    sys::types::{
        gid_t,
        mode_t,
        off_t,
        pid_t,
        size_t,
        ssize_t,
        uid_t,
    },
    unistd::{
        syscall,
        STDERR_FILENO,
        STDIN_FILENO,
        STDOUT_FILENO,
    },
};
use ::core::{
    ffi,
    slice,
};
use ::nvx::sys::error::ErrorCode;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn access(_path: *const c_char, _mode: c_int) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/355
    ::nvx::error!("access(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn chdir(_path: *const c_char) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/358
    ::nvx::error!("chdir(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

///
/// # Description
///
/// Changes the mode of a file.
///
/// # Parameters
///
/// - `path`: Path to the file.
/// - `mode`: Mode of the file.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Otherwise, it returns -1 and sets `errno` to indicate
/// the error.
///
/// # See Also
///
/// - [`crate::unistd::chmod()`]
///
#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn chmod(path: *const c_char, mode: mode_t) -> c_int {
    // Convert C string to Rust string.
    let path: &str = match ffi::CStr::from_ptr(path).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::nvx::error!("chmod(): invalid pathname (path={:?}, mode={:?})", path, mode);
            unsafe {
                errno = ErrorCode::InvalidArgument.get();
            }
            return -1;
        },
    };

    match crate::unistd::chmod(path, mode) {
        Ok(_) => 0,
        Err(e) => {
            ::nvx::error!("chmod(): failed ({:?})", e);
            errno = e.code.get();
            -1
        },
    }
}

///
/// # Description
///
/// Changes the user and group ownership of a file.
///
/// # Parameters
///
/// - `path`: Path to the file.
/// - `owner`: User ID of the new owner.
/// - `group`: Group ID of the new owner.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Otherwise, it returns -1 and sets `errno` to indicate
/// the error.
///
/// # See Also
///
/// - [`crate::unistd::chown()`]
///
#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn chown(path: *const c_char, owner: uid_t, group: gid_t) -> c_int {
    // Convert C string to Rust string.
    let path: &str = match ffi::CStr::from_ptr(path).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::nvx::error!(
                "chown(): invalid pathname (path={:?}, owner={:?}, group={:?})",
                path,
                owner,
                group
            );
            errno = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    match crate::unistd::chown(path, owner, group) {
        Ok(_) => 0,
        Err(e) => {
            ::nvx::error!("chown(): failed ({:?})", e);
            errno = e.code.get();
            -1
        },
    }
}

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub extern "C" fn chroot(_path: *const c_char) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/517
    ::nvx::error!("chroot(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn close(fd: c_int) -> c_int {
    ::nvx::trace!("close(): fd = {}", fd);
    match crate::unistd::close(fd) {
        Ok(()) => 0,
        Err(error) => {
            ::nvx::error!("close(): failed ({:?})", error);
            unsafe {
                errno = error.code.get();
            }
            -1
        },
    }
}

#[no_mangle]
pub extern "C" fn dup2(_oldfd: c_int, _newfd: c_int) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/354
    ::nvx::error!("dup2(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn execve(
    _path: *const c_char,
    _argv: *const *const c_char,
    _envp: *const *const c_char,
) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/320
    ::nvx::error!("execve(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn _exit(status: c_int) -> ! {
    let Err(e) = nvx::sys::kcall::pm::exit(status);
    panic!("failed to terminate process (error={:?})", e);
}

#[no_mangle]
pub extern "C" fn fchdir(_fd: c_int) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/519
    ::nvx::error!("fchdir(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn fdatasync(_fd: c_int) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/278
    ::nvx::error!("fdatasync(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn fork() -> pid_t {
    // TODO: https://github.com/nanvix/nanvix/issues/321
    ::nvx::error!("fork(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

///
/// # Description
///
/// Synchronizes changes to a file.
///
/// # Parameters
///
/// - `fd`: File descriptor.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Otherwise, it returns -1 and sets `errno` to
/// indicate the error.
///
/// # See Also
///
/// - [`crate::unistd::fsync()`]
///
#[no_mangle]
pub extern "C" fn fsync(fd: c_int) -> c_int {
    match crate::unistd::fsync(fd) {
        Ok(_) => 0,
        Err(e) => {
            unsafe {
                errno = e.code.get();
            }
            -1
        },
    }
}

///
/// # Description
///
/// Truncates a file to a specified length.
///
/// # Parameters
///
/// - `path`: Path to the file.
/// - `length`: New size of the file.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Otherwise, it returns -1 and sets `errno` to
/// indicate the error.
///
/// # See Also
///
/// - [`crate::unistd::ftruncate()`]
///
#[no_mangle]
pub extern "C" fn ftruncate(fd: c_int, length: off_t) -> c_int {
    match crate::unistd::ftruncate(fd, length) {
        Ok(_) => 0,
        Err(e) => {
            unsafe {
                errno = e.code.get();
            }
            -1
        },
    }
}

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn getcwd(buf: *mut c_char, size: size_t) -> *mut c_char {
    ::nvx::trace!("getcwd(): buf = {:?}, size = {}", buf, size);

    // Check if the buffer is valid.
    if buf.is_null() {
        ::nvx::error!("getcwd(): invalid buffer");
        unsafe {
            errno = ErrorCode::InvalidArgument.get();
        }
        return core::ptr::null_mut();
    }

    // Get current working directory and check for errors.
    match syscall::getcwd() {
        // Success.
        Ok(cwd) => {
            // Check if the buffer is large enough.
            if cwd.len() + 1 > size as usize {
                ::nvx::error!("getcwd(): buffer is too small");
                unsafe {
                    errno = ErrorCode::ValueOutOfRange.get();
                }
                return core::ptr::null_mut();
            }

            // Copy current working directory to the buffer.
            let cwd: &[u8] = cwd.as_bytes();
            let buf: &mut [u8] = slice::from_raw_parts_mut(buf as *mut u8, size as usize);
            buf[..cwd.len()].copy_from_slice(cwd);

            // Add null terminator.
            buf[cwd.len()] = 0;

            // Return the buffer.
            buf.as_mut_ptr() as *mut c_char
        },
        // Failure.
        Err(e) => {
            unsafe {
                errno = e.code.get();
            }
            core::ptr::null_mut()
        },
    }
}

///
/// # Safety
///
/// The function has undefined behavior if the `path` points to an invalid memory location.
///
#[no_mangle]
pub unsafe extern "C" fn getentropy(_buffer: *mut c_void, _length: size_t) -> c_int {
    ::nvx::trace!("getentropy(): buffer = {:?}, length = {}", _buffer, _length);

    // Fill buffer with 1s.
    let buffer: &mut [u8] = slice::from_raw_parts_mut(_buffer as *mut u8, _length as usize);
    for byte in buffer.iter_mut() {
        *byte = 1;
    }

    0
}

#[no_mangle]
pub extern "C" fn getpid() -> pid_t {
    match crate::unistd::getpid() {
        Ok(pid) => pid.into(),
        Err(e) => {
            unsafe {
                errno = e.code.get();
            }
            -1
        },
    }
}

#[no_mangle]
pub extern "C" fn isatty(_fd: c_int) -> c_int {
    if STDIN_FILENO == _fd || STDOUT_FILENO == _fd || STDERR_FILENO == _fd {
        1
    } else {
        0
    }
}

///
/// # Description
///
/// Changes the mode of a symbolic link.
///
/// # Parameters
///
/// - `path`: Path to the file.
/// - `mode`: Mode of the file.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Otherwise, it returns -1 and sets `errno` to indicate
/// the error.
///
/// # See Also
///
/// - [`crate::unistd::lchmod()`]
///
#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn lchmod(path: *const c_char, mode: mode_t) -> c_int {
    // Convert C string to Rust string.
    let path: &str = match ffi::CStr::from_ptr(path).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::nvx::error!("lchmod(): invalid pathname (path={:?}, mode={:?})", path, mode);
            errno = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    match crate::unistd::lchmod(path, mode) {
        Ok(_) => 0,
        Err(e) => {
            ::nvx::error!("lchmod(): failed ({:?})", e);
            errno = e.code.get();
            -1
        },
    }
}

///
/// # Description
///
/// Changes the user and group ownership of a symbolic link.
///
/// # Parameters
///
/// - `path`: Path to the file.
/// - `owner`: User ID of the new owner.
/// - `group`: Group ID of the new owner.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Otherwise, it returns -1 and sets `errno` to indicate
/// the error.
///
/// # See Also
///
/// - [`crate::unistd::lchown()`]
///
#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn lchown(path: *const c_char, owner: uid_t, group: gid_t) -> c_int {
    // Convert C string to Rust string.
    let path: &str = match ffi::CStr::from_ptr(path).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::nvx::error!(
                "lchown(): invalid pathname (path={:?}, owner={:?}, group={:?})",
                path,
                owner,
                group
            );
            errno = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    match crate::unistd::lchown(path, owner, group) {
        Ok(_) => 0,
        Err(e) => {
            unsafe {
                ::nvx::error!("lchown(): failed ({:?})", e);
                errno = e.code.get();
            }
            -1
        },
    }
}

///
/// # Description
///
/// Creates a new hard link to an existing file.
///
/// # Parameters
///
/// - `oldpath`: Path to the file to be linked.
/// - `newpath`: Path to the new file.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Otherwise, it returns -1 and sets `errno` to
/// indicate the error.
///
/// # See Also
///
/// - [`crate::unistd::link()`]
///
#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn link(oldpath: *const c_char, newpath: *const c_char) -> c_int {
    // Convert C strings to Rust strings.
    let oldpath: &str = match ffi::CStr::from_ptr(oldpath).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::nvx::error!("link(): invalid oldpath");
            errno = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };
    let newpath: &str = match ffi::CStr::from_ptr(newpath).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::nvx::error!("link(): invalid newpath");
            errno = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    let retcode: c_int = crate::unistd::link(oldpath, newpath);

    // Check if the system call failed.
    if retcode < 0 {
        // System call failed. Set errno.
        errno = match ErrorCode::try_from(retcode) {
            Ok(e) => {
                ::nvx::error!("link(): failed ({:?})", e);
                e.get()
            },
            Err(_) => {
                ::nvx::error!("link(): invalid error code ({})", retcode);
                ErrorCode::ValueOutOfRange.get()
            },
        };
        return -1;
    }

    0
}

#[no_mangle]
pub extern "C" fn lseek(fd: c_int, offset: off_t, whence: c_int) -> off_t {
    ::nvx::trace!("lseek(): fd = {}, offset = {}, whence = {}", fd, offset, whence);
    crate::unistd::lseek(fd, offset, whence)
}

///
/// # Description
///
/// Creates a pipe.
///
/// # Parameters
///
/// - `fds`: Array to store the file descriptors of the pipe.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Otherwise, it returns -1 and sets `errno` to
/// indicate the error.
///
#[no_mangle]
pub extern "C" fn pipe(fds: &mut [c_int; 2]) -> c_int {
    ::nvx::trace!("pipe(): fds = {:?}", fds);

    match crate::unistd::pipe() {
        Ok([read_fd, write_fd]) => {
            fds[0] = read_fd;
            fds[1] = write_fd;
            0
        },
        Err(error) => {
            ::nvx::error!("pipe(): failed (error={:?})", error);
            unsafe {
                errno = error.code.get();
            }
            -1
        },
    }
}

///
/// # Safety
///
/// The function has undefined behavior if the `buffer` points to an invalid memory location.
///
#[no_mangle]
pub unsafe extern "C" fn read(fd: c_int, buffer: *mut c_void, count: size_t) -> ssize_t {
    ::nvx::trace!("read(): fd = {}, buffer = {:?}, count = {}", fd, buffer, count);
    crate::unistd::read(fd, buffer as *mut u8, count)
}

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn rmdir(_path: *const c_char) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/348
    ::nvx::error!("rmdir(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub extern "C" fn setgroups(_size: size_t, _list: *const gid_t) -> c_int {
    // TODO: https://github.com/nanvix/nanvix/issues/523
    ::nvx::error!("setgroups(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

///
/// # Description
///
/// Increments the program break.
///
/// # Parameters
///
/// - `size`: Number of bytes to increment the program break.
///
/// # Returns
///
/// Upon successful completion, the `sbrk()` function returns the address of the start of the newly
/// allocated memory. Otherwise, it returns `(void *) -1` and sets `errno` to indicate the error.
///
/// # See Also
///
/// - [`crate::unistd::syscall::sbrk()`]
///
#[no_mangle]
pub extern "C" fn sbrk(size: isize) -> *mut u8 {
    match crate::unistd::sbrk(size) {
        // Succeeded to increment the program break.
        Ok(ptr) => ptr,
        // Failed to increment the program break.
        Err(e) => {
            // Set errno.
            unsafe {
                errno = e.code.get();
            }
            (-1_isize) as *mut u8
        },
    }
}

#[no_mangle]
pub extern "C" fn sleep(_seconds: c_uint) -> c_uint {
    // TODO: https://github.com/nanvix/nanvix/issues/453
    ::nvx::error!("sleep(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    0
}

///
/// # Description
///
/// Creates a symbolic link named `linkpath` which contains the string `target`.
///
/// # Parameters
///
/// - `target`: Path to the file to be linked.
/// - `linkpath`: Path to the new file.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Otherwise, it returns -1 and sets `errno` to
/// indicate the error.
///
/// # See Also
///
/// - [`crate::unistd::syscall::symlink()`]
///
#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn symlink(target: *const c_char, linkpath: *const c_char) -> c_int {
    // Convert C strings to Rust strings.
    let target: &str = match ffi::CStr::from_ptr(target).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::nvx::error!("symlink(): invalid target");
            errno = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };
    let linkpath: &str = match ffi::CStr::from_ptr(linkpath).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::nvx::error!("symlink(): invalid linkpath");
            errno = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    let retcode: c_int = crate::unistd::symlink(target, linkpath);

    // Check if the system call failed.
    if retcode < 0 {
        // System call failed. Set errno.
        errno = match ErrorCode::try_from(retcode) {
            Ok(e) => e.get(),
            Err(_) => {
                ::nvx::error!("symlink(): invalid error code ({})", retcode);
                ErrorCode::ValueOutOfRange.get()
            },
        };
        return -1;
    }

    0
}

#[no_mangle]
pub extern "C" fn sysconf(_name: c_int) -> c_long {
    // TODO: https://github.com/nanvix/nanvix/issues/342
    ::nvx::error!("sysconf(): not implemented");
    0
}

///
/// # Description
///
/// Deletes a name from the filesystem.
///
/// # Parameters
///
/// - `path`: Path to the file to be unlinked.
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Otherwise, it returns -1 and sets `errno` to
/// indicate the error.
///
/// # See Also
///
/// - [`crate::unistd::unlink()`]
///
#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn unlink(path: *const c_char) -> c_int {
    // Convert C string to Rust string.
    let path: &str = match ffi::CStr::from_ptr(path).to_str() {
        Ok(pathname) => pathname,
        Err(_) => {
            ::nvx::error!("unlink(): invalid path");
            errno = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    let retcode: c_int = crate::unistd::unlink(path);

    // Check if the system call failed.
    if retcode < 0 {
        // System call failed. Set errno.
        errno = match ErrorCode::try_from(retcode) {
            Ok(e) => e.get(),
            Err(_) => {
                ::nvx::error!("unlink(): invalid error code ({})", retcode);
                ErrorCode::ValueOutOfRange.get()
            },
        };
        return -1;
    }

    0
}

///
/// # Safety
///
/// The function has undefined behavior if the `buffer` points to an invalid memory location.
///
#[no_mangle]
pub unsafe extern "C" fn write(fd: c_int, buffer: *const c_void, count: size_t) -> ssize_t {
    // Skip logging for stdout and stderr to avoid spamming the output.
    if fd != STDOUT_FILENO && fd != STDERR_FILENO {
        ::nvx::trace!("write(): fd = {}, buffer = {:?}, count = {}", fd, buffer, count);
    }
    crate::unistd::write(fd, buffer as *const u8, count)
}

#[no_mangle]
pub extern "C" fn cfmakeraw(_termios_p: *mut c_void) {
    ::nvx::error!("cfmakeraw(): not implemented");
}

#[no_mangle]
pub extern "C" fn execvp(_file: *const c_char, _argv: *const *const c_char) -> c_int {
    ::nvx::error!("execvp(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn fchown(_fd: c_int, _owner: uid_t, _group: gid_t) -> c_int {
    ::nvx::error!("fchown(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn freeifaddrs(_ifa: *mut c_void) {
    ::nvx::error!("freeifaddrs(): not implemented");
}

#[no_mangle]
pub extern "C" fn geteuid() -> uid_t {
    ::nvx::error!("geteuid(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    0 as uid_t
}

#[no_mangle]
pub extern "C" fn getgrgid_r(
    _gid: gid_t,
    _grp: *mut c_void,
    _buf: *mut c_char,
    _bufsize: size_t,
    _result: *mut *mut c_void,
) -> c_int {
    ::nvx::error!("getgrgid_r(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn gethostname(_name: *mut c_char, _len: size_t) -> c_int {
    ::nvx::error!("gethostname(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn getifaddrs(_ifa: *mut *mut c_void) -> c_int {
    ::nvx::error!("getifaddrs(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn getpagesize() -> c_int {
    ::nvx::error!("getpagesize(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn getppid() -> pid_t {
    ::nvx::error!("getppid(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

type id_t = c_int;

#[no_mangle]
pub extern "C" fn getpriority(_which: c_int, _who: id_t) -> c_int {
    ::nvx::error!("getpriority(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn getpwuid_r(
    _uid: uid_t,
    _pwd: *mut c_void,
    _buf: *mut c_char,
    _bufsize: size_t,
    _result: *mut *mut c_void,
) -> c_int {
    ::nvx::error!("getpwuid_r(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn getrusage(_who: c_int, _usage: *mut c_void) -> c_int {
    ::nvx::error!("getrusage(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn getsockopt(
    _sockfd: c_int,
    _level: c_int,
    _optname: c_int,
    _optval: *mut c_void,
    _optlen: *mut crate::sys::socket::socklen_t,
) -> c_int {
    ::nvx::error!("getsockopt(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn if_nametoindex(_ifname: *const c_char) -> c_uint {
    ::nvx::error!("if_nametoindex(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    0
}

#[no_mangle]
pub extern "C" fn in6addr_any() -> *const c_void {
    ::nvx::error!("in6addr_any(): not implemented");
    core::ptr::null()
}

#[no_mangle]
pub extern "C" fn nanosleep(_req: *const c_void, _rem: *mut c_void) -> c_int {
    ::nvx::error!("nanosleep(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn pathconf(_path: *const c_char, _name: c_int) -> c_long {
    ::nvx::error!("pathconf(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn poll(_fds: *mut c_void, _nfds: c_int, _timeout: c_int) -> c_int {
    ::nvx::error!("poll(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn pread(_fd: c_int, _buf: *mut c_void, _count: size_t, _offset: off_t) -> ssize_t {
    ::nvx::error!("pread(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn preadv(
    _fd: c_int,
    _iov: *const c_void,
    _iovcnt: c_int,
    _offset: off_t,
) -> ssize_t {
    ::nvx::error!("preadv(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn pthread_atfork(
    _prepare: Option<unsafe extern "C" fn()>,
    _parent: Option<unsafe extern "C" fn()>,
    _child: Option<unsafe extern "C" fn()>,
) -> c_int {
    ::nvx::error!("pthread_atfork(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn pthread_condattr_destroy(_attr: *mut c_void) -> c_int {
    ::nvx::error!("pthread_condattr_destroy(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn pthread_condattr_init(_attr: *mut c_void) -> c_int {
    ::nvx::error!("pthread_condattr_init(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn pthread_condattr_setclock(_attr: *mut c_void, _clock_id: c_int) -> c_int {
    ::nvx::error!("pthread_condattr_setclock(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn pthread_getschedparam(
    _thread: c_int,
    _policy: *mut c_int,
    _param: *mut c_void,
) -> c_int {
    ::nvx::error!("pthread_getschedparam(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn pthread_mutexattr_destroy(_attr: *mut c_void) -> c_int {
    ::nvx::error!("pthread_mutexattr_destroy(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn pthread_mutexattr_init(_attr: *mut c_void) -> c_int {
    ::nvx::error!("pthread_mutexattr_init(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn pthread_mutexattr_settype(_attr: *mut c_void, _type: c_int) -> c_int {
    ::nvx::error!("pthread_mutexattr_settype(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn pthread_once(
    _once_control: *mut c_void,
    _init_routine: unsafe extern "C" fn(),
) -> c_int {
    ::nvx::error!("pthread_once(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn pthread_rwlock_destroy(_rwlock: *mut c_void) -> c_int {
    ::nvx::error!("pthread_rwlock_destroy(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn pthread_rwlock_init(_rwlock: *mut c_void, _attr: *const c_void) -> c_int {
    ::nvx::error!("pthread_rwlock_init(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn pthread_rwlock_rdlock(_rwlock: *mut c_void) -> c_int {
    ::nvx::error!("pthread_rwlock_rdlock(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn pthread_rwlock_tryrdlock(_rwlock: *mut c_void) -> c_int {
    ::nvx::error!("pthread_rwlock_tryrdlock(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn pthread_rwlock_trywrlock(_rwlock: *mut c_void) -> c_int {
    ::nvx::error!("pthread_rwlock_trywrlock(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn pthread_rwlock_unlock(_rwlock: *mut c_void) -> c_int {
    ::nvx::error!("pthread_rwlock_unlock(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn pthread_rwlock_wrlock(_rwlock: *mut c_void) -> c_int {
    ::nvx::error!("pthread_rwlock_wrlock(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn pthread_setschedparam(
    _thread: c_int,
    _policy: c_int,
    _param: *const c_void,
) -> c_int {
    ::nvx::error!("pthread_setschedparam(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn pthread_sigmask(_how: c_int, _set: *const c_void, _oldset: *mut c_void) -> c_int {
    ::nvx::error!("pthread_sigmask(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn ptsname(_fd: c_int) -> *const c_char {
    ::nvx::error!("ptsname(): not implemented");
    core::ptr::null()
}

#[no_mangle]
pub extern "C" fn pwrite(
    _fd: c_int,
    _buf: *const c_void,
    _count: size_t,
    _offset: off_t,
) -> ssize_t {
    ::nvx::error!("pwrite(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn pwritev(
    _fd: c_int,
    _iov: *const c_void,
    _iovcnt: c_int,
    _offset: off_t,
) -> ssize_t {
    ::nvx::error!("pwritev(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn readlink(_path: *const c_char, _buf: *mut c_char, _bufsize: size_t) -> ssize_t {
    ::nvx::error!("readlink(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn readv(_fd: c_int, _iov: *const c_void, _iovcnt: c_int) -> ssize_t {
    ::nvx::error!("readv(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn realpath(_path: *const c_char, _resolved_path: *mut c_char) -> *mut c_char {
    ::nvx::error!("realpath(): not implemented");
    core::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn recvmsg(_sockfd: c_int, _msg: *mut c_void, _flags: c_int) -> ssize_t {
    ::nvx::error!("recvmsg(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn scandir(
    _dirp: *const c_char,
    _namelist: *mut *mut c_void,
    _filter: Option<unsafe extern "C" fn(*const c_void) -> c_int>,
    _compar: Option<unsafe extern "C" fn(*const c_void, *const c_void) -> c_int>,
) -> c_int {
    ::nvx::error!("scandir(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn sched_get_priority_max(_policy: c_int) -> c_int {
    ::nvx::error!("sched_get_priority_max(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn sched_get_priority_min(_policy: c_int) -> c_int {
    ::nvx::error!("sched_get_priority_min(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn sem_destroy(_sem: *mut c_void) -> c_int {
    ::nvx::error!("sem_destroy(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn sem_init(_sem: *mut c_void, _pshared: c_int, _value: c_uint) -> c_int {
    ::nvx::error!("sem_init(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn sem_post(_sem: *mut c_void) -> c_int {
    ::nvx::error!("sem_post(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn sem_trywait(_sem: *mut c_void) -> c_int {
    ::nvx::error!("sem_trywait(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn sem_wait(_sem: *mut c_void) -> c_int {
    ::nvx::error!("sem_wait(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn sendmsg(_sockfd: c_int, _msg: *const c_void, _flags: c_int) -> ssize_t {
    ::nvx::error!("sendmsg(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn setgid(_gid: gid_t) -> c_int {
    ::nvx::error!("setgid(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn setpriority(_which: c_int, _who: id_t, _prio: c_int) -> c_int {
    ::nvx::error!("setpriority(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn setsid() -> pid_t {
    ::nvx::error!("setsid(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn setsockopt(
    _sockfd: c_int,
    _level: c_int,
    _optname: c_int,
    _optval: *const c_void,
    _optlen: crate::sys::socket::socklen_t,
) -> c_int {
    ::nvx::error!("setsockopt(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn setuid(_uid: uid_t) -> c_int {
    ::nvx::error!("setuid(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn sigaction(_signum: c_int, _act: *const c_void, _oldact: *mut c_void) -> c_int {
    ::nvx::error!("sigaction(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn sigprocmask(_how: c_int, _set: *const c_void, _oldset: *mut c_void) -> c_int {
    ::nvx::error!("sigprocmask(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn statvfs(_path: *const c_char, _buf: *mut c_void) -> c_int {
    ::nvx::error!("statvfs(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn tcgetattr(_fd: c_int, _termios_p: *mut c_void) -> c_int {
    ::nvx::error!("tcgetattr(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn tcsetattr(
    _fd: c_int,
    _optional_actions: c_int,
    _termios_p: *const c_void,
) -> c_int {
    ::nvx::error!("tcsetattr(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn ttyname_r(_fd: c_int, _buf: *mut c_char, _bufsize: size_t) -> c_int {
    ::nvx::error!("ttyname_r(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[no_mangle]
pub extern "C" fn waitpid(_pid: pid_t, _status: *mut c_int, _options: c_int) -> pid_t {
    ::nvx::error!("waitpid(): not implemented");
    unsafe {
        errno = ErrorCode::InvalidSysCall.get();
    }
    -1
}
