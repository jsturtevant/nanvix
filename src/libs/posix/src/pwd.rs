// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[cfg(all(feature = "syscall", feature = "staticlib"))]
mod bindings {
    use ::sysapi::{
        ffi::c_int,
        pwd::passwd,
    };

    /// Static root user name.
    static ROOT_NAME: &[u8] = b"root\0";

    /// Static root user password (empty).
    static ROOT_PASSWD: &[u8] = b"\0";

    /// Static root user GECOS field.
    static ROOT_GECOS: &[u8] = b"root\0";

    /// Static root user home directory.
    static ROOT_DIR: &[u8] = b"/root\0";

    /// Static root user login shell.
    static ROOT_SHELL: &[u8] = b"/bin/sh\0";

    /// Static passwd entry for the root user.
    static mut ROOT_PASSWD_ENTRY: passwd = passwd {
        pw_name: ROOT_NAME.as_ptr() as *const i8,
        pw_passwd: ROOT_PASSWD.as_ptr() as *const i8,
        pw_uid: 0,
        pw_gid: 0,
        pw_gecos: ROOT_GECOS.as_ptr() as *const i8,
        pw_dir: ROOT_DIR.as_ptr() as *const i8,
        pw_shell: ROOT_SHELL.as_ptr() as *const i8,
    };

    ///
    /// # Description
    ///
    /// Returns the password database entry for the given user ID. In Nanvix, all processes run as
    /// root (uid=0) in a single-user guest environment, so this always returns a static root entry.
    ///
    /// # Parameters
    ///
    /// - `_uid`: User ID to look up (ignored, always returns root).
    ///
    /// # Returns
    ///
    /// A pointer to a static `passwd` structure for the root user.
    ///
    /// # Safety
    ///
    /// This function returns a pointer to a mutable static. The caller must ensure no concurrent
    /// mutation occurs.
    ///
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn getpwuid(_uid: c_int) -> *mut passwd {
        ::syslog::trace!("getpwuid(): returning root user entry (uid={})", _uid);
        &raw mut ROOT_PASSWD_ENTRY
    }
}
