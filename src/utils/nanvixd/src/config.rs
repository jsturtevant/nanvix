// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::log::error;

//==================================================================================================
// Constants
//==================================================================================================

/// Default binary directory path.
pub const DEFAULT_BIN_DIRECTORY: &str = "./bin";

/// Suffix for Unix sockets.
#[cfg(debug_assertions)]
const UNIX_SOCKET_SUFFIX: &str = ".debug.socket";
#[cfg(not(debug_assertions))]
const UNIX_SOCKET_SUFFIX: &str = ".socket";

/// IP address for the services inside the L2 system VM.
///
/// This is the address that services running inside the L2 VM can bind to. For example, linuxd
/// will bind its gateway and user VM listener sockets to this IP.
///
/// FIXME(#838): this IP is hard-coded in the L2 VM's initramfs (and snapshot) creation process.
const DEFAULT_L2_SYSTEM_VM_GUEST_IP: &str = "192.168.249.2";

/// Name of the directory containing the L2 snapshot.
const L2_VM_SNAPSHOT_NAME: &str = "l2-sysvm-snapshot";

/// Default ports for TCP sockets.
const DEFAULT_RESTORE_GATE_PORT: u32 = 5555;
const DEFAULT_CONTROL_PLANE_PORT: u32 = 9000;
const DEFAULT_USER_VM_PORT: u32 = 9001;
const DEFAULT_GATEWAY_PORT: u32 = 9002;

/// Path to the temporary directory.
pub const DEFAULT_TMP_DIRECTORY: &str = "/tmp";

pub const HTTP_HEADER_MESSAGE_TYPE: &str = "X-NVX-Message-Type";

/// Maximum length for a Unix socket name, including the null terminator.
/// This is a workaround for the fact that `libc::UNIX_PATH_MAX` is not available.
/// On Linux, this is defined in `<linux/un.h>`.
/// TODO: replace this with `libc::UNIX_PATH_MAX` when it becomes available.
const UNIX_PATH_MAX: usize = 108;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Builds the control plane socket address for a given tenant ID. If nanvixd is configured to
/// spawn linuxd in an L2 VM, it will return a TCP socket address, otherwise a Unix socket one.
///
/// # Arguments
///
/// - tmp_str: Temporary directory path.
/// - tenant_id: Tenant ID.
/// - l2: Flag to enable deploying linuxd inside an L2 VM.
///
/// # Returns
///
/// On success, returns the name of the control plane socket. On failure, returns an error.
///
pub fn restore_gate_sockaddr_builder() -> String {
    format!("{DEFAULT_L2_SYSTEM_VM_GUEST_IP}:{DEFAULT_RESTORE_GATE_PORT}")
}

///
/// # Description
///
/// Builds the control plane socket address for a given tenant ID. If nanvixd is configured to
/// spawn linuxd in an L2 VM, it will return a TCP socket address, otherwise a Unix socket one.
///
/// When binding to a TCP address we want to make sure that any L2 VM can connect to us, so we bind
/// to 0.0.0.0.
///
/// # Arguments
///
/// - tmp_str: Temporary directory path.
/// - tenant_id: Tenant ID.
/// - l2: Flag to enable deploying linuxd inside an L2 VM.
///
/// # Returns
///
/// On success, returns the name of the control plane socket. On failure, returns an error.
///
pub fn control_plane_sockaddr_builder(tmp_str: &str, tenant_id: &str, l2: bool) -> Result<String> {
    if l2 {
        return Ok(format!("0.0.0.0:{DEFAULT_CONTROL_PLANE_PORT}"));
    }

    let unix_socket_name: String =
        format!("{tmp_str}/control-plane:{tenant_id}:cp{UNIX_SOCKET_SUFFIX}");

    // Check if socket name exceeds the maximum length.
    if unix_socket_name.len() > UNIX_PATH_MAX {
        let error: String = format!(
            "unix socket name '{unix_socket_name}' exceeds maximum length ({:?} > {:?})",
            unix_socket_name.len(),
            UNIX_PATH_MAX
        );
        error!("control_plane_sockaddr_builder(): {error}");
        anyhow::bail!(error);
    }

    Ok(unix_socket_name)
}

///
/// # Description
///
/// Builds the user VM socket address for a given tenant ID.
///
/// # Arguments
///
/// - tmp_str: Temporary directory path.
/// - tenant_id: Tenant ID.
/// - l2: Flag to enable deploying linuxd inside an L2 VM.
///
/// # Returns
///
/// On success, returns the name of the user VM Unix socket. On failure, returns an error.
///
pub fn user_vm_sockaddr_builder(tmp_str: &str, tenant_id: &str, l2: bool) -> Result<String> {
    if l2 {
        return Ok(format!("{DEFAULT_L2_SYSTEM_VM_GUEST_IP}:{DEFAULT_USER_VM_PORT}"));
    }

    let unix_socket_name: String = format!("{tmp_str}/{tenant_id}:uvm{UNIX_SOCKET_SUFFIX}");

    // Check if socket name exceeds the maximum length.
    if unix_socket_name.len() > UNIX_PATH_MAX {
        let error: String = format!(
            "unix socket name '{unix_socket_name}' exceeds maximum length ({:?} > {:?})",
            unix_socket_name.len(),
            UNIX_PATH_MAX
        );
        error!("user_vm_sockaddr_builder(): {error}");
        anyhow::bail!(error);
    }

    Ok(unix_socket_name)
}

///
/// # Description
///
/// Builds the gateway socket address for a given tenant ID.
///
/// # Arguments
///
/// - tmp_str: Temporary directory path.
/// - tenant_id: Tenant ID.
/// - l2: Flag to enable deploying linuxd inside an L2 VM.
///
/// # Returns
///
/// On success, returns the name of the gateway Unix socket. On failure, returns an error.
///
pub fn gateway_sockaddr_builder(tmp_str: &str, tenant_id: &str, l2: bool) -> Result<String> {
    if l2 {
        return Ok(format!("{DEFAULT_L2_SYSTEM_VM_GUEST_IP}:{DEFAULT_GATEWAY_PORT}"));
    }

    let unix_socket_name: String = format!("{tmp_str}/{tenant_id}:gw{UNIX_SOCKET_SUFFIX}");

    // Check if socket name exceeds the maximum length.
    if unix_socket_name.len() > UNIX_PATH_MAX {
        let error: String = format!(
            "unix socket name '{unix_socket_name}' exceeds maximum length ({:?} > {:?})",
            unix_socket_name.len(),
            UNIX_PATH_MAX
        );
        error!("gateway_sockaddr_builder(): {error}");
        anyhow::bail!(error);
    }

    Ok(unix_socket_name)
}

///
/// # Description
///
/// Gets the absolute path for the source root.
///
/// # Returns
///
/// The absolute path to the source code root.
///
fn get_proj_root() -> String {
    format!("{}/../../..", env!("CARGO_MANIFEST_DIR"))
}

///
/// # Description
///
/// Gets the absolute path for cloud-hypervisor's binary directory.
///
/// # Returns
///
/// The absolute path to cloud-hypervisor's binary directory.
///
pub fn get_clh_bin_dir() -> String {
    format!("{}/toolchain/bin/", get_proj_root())
}

///
/// # Description
///
/// Gets the absolute path for cloud-hypervisor's snapshot directory.
///
/// # Returns
///
/// The absolute path to cloud-hypervisor's snapshot directory.
///
pub fn get_clh_snapshot_path() -> String {
    format!("{}/images/{L2_VM_SNAPSHOT_NAME}", get_proj_root())
}
