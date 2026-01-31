// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

pub mod guest;

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    orchestrator::VcpuControlResponse,
    vmm::{
        MicroVmArgs,
        guest::Guest,
    },
};
use ::anyhow::Result;
use ::arch::mem::PAGE_SIZE;
use ::core::convert::TryFrom;
use ::hyperlight_host::{
    GuestBinary,
    HyperlightError,
    MultiUseSandbox,
    UninitializedSandbox,
    hyperlight_fs::{
        FatImage,
        HyperlightFSBuilder,
        HyperlightFSImage,
    },
    mem::{
        memory_region::MemoryRegionFlags,
        mgr::SandboxMemoryManager,
        shared_mem::ExclusiveSharedMemory,
    },
    sandbox::{
        SandboxConfiguration,
        uninitialized::{
            GuestBlob,
            GuestEnvironment,
        },
    },
};
use ::std::{
    io::Write,
    os::raw::c_int,
    path::Path,
    sync::Arc,
};
use ::sys::error::ErrorCode;
use ::syslog::{
    debug,
    error,
    info,
};
use ::tokio::{
    runtime::Handle,
    sync::{
        Mutex,
        mpsc::Sender,
    },
    task,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Signal used to interrupt the vCPU thread.
pub const INTERRUPT_SIGNAL: c_int = libc::SIGUSR1;

/// Signal used to kill the vCPU thread.
pub const KILL_SIGNAL: c_int = libc::SIGKILL;

//==================================================================================================
// Types
//==================================================================================================

pub type StdinFn = dyn FnMut() -> Result<Vec<u8>, HyperlightError> + Send;

pub type StdoutFn = dyn FnMut(Vec<u8>) -> Result<i32, HyperlightError> + Send;

pub type StderrFn = dyn Write + Send;

//==================================================================================================
// Structure
//==================================================================================================

pub struct VirtualMemory {
    manager: SandboxMemoryManager<ExclusiveSharedMemory>,
}

#[derive(Clone)]
pub struct Vmm {
    guest: Arc<Mutex<Guest>>,
    inner: Arc<Mutex<InnerVmm>>,
    // Wrapped in Option so we can move the UninitializedSandbox out (evolve consumes self).
    sandbox: Arc<Mutex<Option<UninitializedSandbox>>>,
    vmem: Arc<Mutex<VirtualMemory>>,
}

struct InnerVmm {
    control_tx: Sender<VcpuControlResponse>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Vmm {
    pub fn new(args: MicroVmArgs) -> Result<Self> {
        let guest: Guest = Guest::default();

        let ramfs_filename: Option<String> = args.ramfs_filename.clone();

        // Required values for heap and stack sizes to be used by the kernel.
        let heap_size: usize = 4 * 1024 * 1024;
        let stack_size: usize = 4 * 1024;
        let memory_size: usize = args.memory_size;

        let guest_env: GuestEnvironment = if let Some(initrd_filename) = &args.initrd_filename {
            match std::fs::read(initrd_filename) {
                Ok(bytes) => {
                    let initrd_size: usize = bytes.len();
                    debug!("initrd: {} bytes", initrd_size);

                    let kernel_metadata: ::std::fs::Metadata =
                        std::fs::metadata(&args.kernel_filename).map_err(|e| {
                            let reason: String =
                                format!("failed to read kernel file metadata: {}", e);
                            error!("initrd(): {}", reason);
                            anyhow::anyhow!(reason)
                        })?;
                    let kernel_size: usize =
                        usize::try_from(kernel_metadata.len()).map_err(|_| {
                            let reason: String = format!(
                                "kernel file size {} exceeds supported range",
                                kernel_metadata.len()
                            );
                            error!("initrd(): {}", reason);
                            anyhow::anyhow!(reason)
                        })?;

                    let initrd_args_bytes: Vec<u8> =
                        Self::build_args_bytes(initrd_filename, &args.initrd_args)?;

                    // PEB, I/O buffers, host fxn defs, guard pages, etc.
                    let reserved_pages: usize = 11 * PAGE_SIZE;

                    // Calculate FAT filesystem size from fat_images.
                    let fat_images: &Vec<(String, String)> = &args.fat_images;
                    let fat_size: usize = calculate_fat_images_size(fat_images)?;

                    let required_memory: usize = kernel_size
                        + initrd_size
                        + (heap_size + stack_size)
                        + reserved_pages
                        + ::config::hyperlight::INITRD_SIZE_BYTES
                        + initrd_args_bytes.len()
                        + fat_size;

                    // Check if required memory exceeds memory size.
                    if memory_size <= required_memory {
                        let reason: &str = "not enough memory";
                        error!(
                            "new(): {reason} ({required_memory} bytes required, {memory_size} \
                             bytes total, fat_size={fat_size})"
                        );
                        return Err(anyhow::anyhow!(reason));
                    }

                    let padding_size: usize = memory_size - required_memory;

                    // Create a new vector with size header + original data + padding
                    let mut padded_bytes: Vec<u8> = Vec::with_capacity(
                        ::config::hyperlight::INITRD_SIZE_BYTES
                            + initrd_size
                            + initrd_args_bytes.len(),
                    );

                    // Write the actual size as first INITRD_SIZE_BYTES-bytes (little-endian)
                    padded_bytes.extend_from_slice(&(initrd_size as u64).to_le_bytes());

                    // Add the actual initrd data
                    padded_bytes.extend_from_slice(&bytes);

                    // Append length-prefixed initrd arguments so the guest can consume them.
                    padded_bytes.extend_from_slice(&initrd_args_bytes);

                    debug!(
                        "initrd blob: {} bytes total ({} byte header + {} bytes data + 1 byte \
                         args length + {} bytes args payload), extra memory: {} bytes",
                        padded_bytes.len(),
                        ::config::hyperlight::INITRD_SIZE_BYTES,
                        initrd_size,
                        initrd_args_bytes.len(),
                        padding_size
                    );

                    // Box the data to extend its lifetime
                    let boxed_data: Box<[u8]> = padded_bytes.into_boxed_slice();
                    let data_ref: &'static [u8] = Box::leak(boxed_data);

                    let extra_memory: u64 = padding_size.try_into().map_err(|_| {
                        let reason: String =
                            format!("padding size {} exceeds supported range", padding_size);
                        error!("initrd(): {}", reason);
                        anyhow::anyhow!(reason)
                    })?;

                    GuestEnvironment {
                        guest_binary: GuestBinary::FilePath(args.kernel_filename.to_string()),
                        init_data: Some(GuestBlob {
                            data: data_ref,
                            permissions: MemoryRegionFlags::READ
                                | MemoryRegionFlags::WRITE
                                | MemoryRegionFlags::EXECUTE,
                        }),
                        extra_memory: Some(extra_memory),
                    }
                },
                Err(err) => {
                    let reason: String = format!("failed to read initrd file {err:?}");
                    error!("initrd(): {reason} (args={args:?})");
                    return Err(anyhow::anyhow!("{reason}"));
                },
            }
        } else {
            GuestEnvironment::new(GuestBinary::FilePath(args.kernel_filename.to_string()), None)
        };

        let mut config: SandboxConfiguration = SandboxConfiguration::default();
        let heap_size_u64: u64 = u64::try_from(heap_size).map_err(|_| {
            let reason: String = format!("heap size {} exceeds supported range", heap_size);
            error!("hyperlight::new(): {}", reason);
            anyhow::anyhow!(reason)
        })?;
        config.set_heap_size(heap_size_u64);

        let stack_size_u64: u64 = u64::try_from(stack_size).map_err(|_| {
            let reason: String = format!("stack size {} exceeds supported range", stack_size);
            error!("hyperlight::new(): {}", reason);
            anyhow::anyhow!(reason)
        })?;
        config.set_stack_size(stack_size_u64);

        // Get references to mount configurations.
        let mounts: &Vec<(String, String)> = &args.mounts;
        let fat_images: &Vec<(String, String)> = &args.fat_images;

        // Build Hyperlight filesystem image.
        // Strategy: Use pre-built FAT images if provided, otherwise create an empty FAT at root.
        let use_prebuilt_fat: bool = !fat_images.is_empty();
        let mut fs_image: HyperlightFSImage = if use_prebuilt_fat {
            build_fs_with_prebuilt_fat(fat_images, ramfs_filename.as_ref())?
        } else {
            build_fs_with_empty_fat(mounts, ramfs_filename.as_ref())?
        };
        debug!("hyperlight::new(): built filesystem image (prebuilt={})", use_prebuilt_fat);

        // Apply mounts and ramfs to the FAT filesystem.
        setup_fat_filesystem(
            &mut fs_image,
            use_prebuilt_fat,
            fat_images,
            mounts,
            ramfs_filename.as_ref(),
        )?;

        // Creates Hyperlight sandbox.
        let mut sandbox: UninitializedSandbox =
            UninitializedSandbox::new(guest_env, Some(config))?.with_hyperlight_fs(fs_image);
        let manager: SandboxMemoryManager<ExclusiveSharedMemory> = sandbox.mgr.clone();
        let vmem: Arc<Mutex<VirtualMemory>> = Arc::new(Mutex::new(VirtualMemory {
            manager: manager.clone(),
        }));

        let guest: Arc<Mutex<Guest>> = Arc::new(Mutex::new(guest));

        // Create a closure that takes a String and writes it to stderr.
        // NOTE: underlying writer implements `Write` and requires mutable access.
        let mut stderr_writer: Box<StderrFn> = args.stderr;
        sandbox.register_print(move |s: String| -> i32 {
            if stderr_writer.write_all(s.as_bytes()).is_err() {
                return -1;
            }
            if stderr_writer.flush().is_err() {
                return -1;
            }
            0
        })?;

        // Create a closure for VmbusWrite that matches the expected signature
        // NOTE: output function is FnMut, so we must keep it mutable when captured.
        let mut output_fn: Box<StdoutFn> = args.output;
        sandbox.register("VmbusWrite", move |data: Vec<u8>| -> i32 {
            output_fn(data).unwrap_or(-1)
        })?;

        // Create a closure for VmbusRead that matches the expected signature
        // NOTE: input function is FnMut, so we must keep it mutable when captured.
        let mut input_fn: Box<StdinFn> = args.input;
        sandbox.register("VmbusRead", move || -> Vec<u8> { input_fn().unwrap_or_default() })?;

        Ok(Self {
            vmem,
            sandbox: Arc::new(Mutex::new(Some(sandbox))),
            guest,
            inner: Arc::new(Mutex::new(InnerVmm {
                control_tx: args.control_tx,
            })),
        })
    }

    pub fn spawn(mut self) -> tokio::task::JoinHandle<Result<u16>> {
        task::spawn_blocking(move || {
            let pthread_id: libc::pthread_t = unsafe { libc::pthread_self() };
            Handle::current().block_on(self.send_tid(pthread_id))?;
            self.run()
        })
    }

    ///
    /// # Description
    ///
    /// This function instantiates and runs the virtual machine monitor (VMM) with the given arguments.
    ///
    /// # Parameters
    ///
    /// - `memory_size`: The memory size for the virtual machine in bytes.
    /// - `kernel_filename`: The path to the kernel file to be loaded into the virtual machine.
    /// - `initrd_filename`: An optional path to the initial RAM disk (initrd) file.
    /// - `initrd_args`: Optional arguments to be passed to the initrd.
    /// - `stderr`: An optional path to a file where the virtual machine's standard error output will be written.
    /// - `system_vm_stream`: An optional connection to the system VM for communication with the virtual machine.
    /// - `control_plane_stream`: An optional connection to the nanvixd control-plane.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns the exit status of the virtual machine.
    /// Otherwise, it returns an error.
    ///
    pub fn run(&mut self) -> Result<u16> {
        let uninit: UninitializedSandbox = {
            let mut guard: ::tokio::sync::MutexGuard<'_, Option<UninitializedSandbox>> =
                self.sandbox.blocking_lock();
            guard
                .take()
                .ok_or_else(|| anyhow::anyhow!("sandbox already evolved"))?
        };

        // Run the sandbox.
        let result: Result<MultiUseSandbox, HyperlightError> = uninit.evolve();

        // Communicate shutdown to orchestrator.
        if let Err(error) = self
            .inner
            .blocking_lock()
            .control_tx
            .blocking_send(VcpuControlResponse::Shutdown)
        {
            error!("run(): failed to notify vmm thread (error={error:?})");
            // Don't bail as we are shutting down anyway.
        }

        // Parse result.
        match result {
            Ok(_multiuse_sandbox) => {
                error!("run(): vmm exited");
                Ok(ErrorCode::ConnectionAborted.into())
            },
            Err(error) => {
                // note: this is a bit of a hack to check for the shutdown command.
                if !error
                    .to_string()
                    .contains(&::config::hyperlight::DEFAULT_VMM_SHUTDOWN_CMD.to_string())
                {
                    error!("run(): vmm aborted (error={error:?})");
                    Ok(ErrorCode::ConnectionReset.into())
                } else {
                    // FIXME (#1010): the vCPU thread already returns 0 always, but we will be able to remove
                    // this line once we can join the vCPU thread.
                    Ok(0)
                }
            },
        }
    }

    ///
    /// # Description
    ///
    /// Sends the vCPU thread's tid to the main thread.
    ///
    /// # Parameters
    ///
    /// - `tid`: The vCPU thread's tid.
    ///
    /// # Returns
    ///
    /// Upon success, returns empty. Otherwise, returns an error.
    ///
    async fn send_tid(&self, tid: libc::pthread_t) -> Result<()> {
        Ok(self
            .inner
            .lock()
            .await
            .control_tx
            .send(VcpuControlResponse::Tid(tid))
            .await?)
    }

    pub fn guest(&self) -> Arc<Mutex<Guest>> {
        self.guest.clone()
    }

    pub fn vmem(&self) -> Arc<Mutex<VirtualMemory>> {
        self.vmem.clone()
    }

    pub async fn load_snapshot(&self, filepath: String) -> Result<()> {
        let reason: String = format!("load_snapshot(): not implemented for filepath={}", filepath);
        error!("{}", reason);
        Err(anyhow::anyhow!(reason))
    }

    pub async fn create_snapshot(&self, filepath: String) -> Result<()> {
        let reason: String =
            format!("create_snapshot(): not implemented for filepath={}", filepath);
        error!("{}", reason);
        Err(anyhow::anyhow!(reason))
    }

    ///
    /// # Description
    ///
    /// Encodes the program name and arguments into a byte vector suitable for passing to the guest.
    /// The first byte of the vector indicates the length of the arguments, followed by the program
    /// name and arguments as a null-terminated string.
    ///
    /// # Parameters
    ///
    /// - `program_name`: The name of the program to be executed.
    /// - `program_args`: An optional string containing the arguments to be passed to the program.
    ///
    /// # Returns
    ///
    /// On success, this function returns a vector of bytes representing the encoded arguments.
    /// On failure, it returns an error.
    ///
    fn build_args_bytes(program_name: &String, program_args: &Option<String>) -> Result<Vec<u8>> {
        // Extract filename.
        let mut args_string: String = Path::new(program_name)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| program_name.to_string());

        // Push arguments.
        if let Some(program_args) = program_args
            && !program_args.is_empty()
        {
            args_string.push(' ');
            args_string.push_str(program_args);
        }

        // Encode length-prefixed arguments.
        let args_bytes: Vec<u8> = args_string.into_bytes();
        let args_len: u8 = match u8::try_from(args_bytes.len()) {
            Ok(value) => value,
            Err(_) => {
                let reason: String =
                    format!("initrd arguments too long (len={})", args_bytes.len());
                error!("build_args_bytes(): {}", reason);
                return Err(anyhow::anyhow!(reason));
            },
        };

        Ok([&[args_len], &args_bytes[..]].concat())
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Derives the guest path for a ramfs file from its host path.
/// Returns the basename prefixed with "/" (e.g., "/path/to/file.bin" -> "/file.bin").
///
/// # Parameters
///
/// - `host_path`: The path to the ramfs file on the host.
///
/// # Returns
///
/// Upon successful completion, returns the guest path. Otherwise, returns an error.
///
fn derive_ramfs_guest_path(host_path: &str) -> Result<String> {
    let path: &Path = Path::new(host_path);
    if !path.is_file() {
        let reason: String = format!("ramfs file not found (path={host_path})");
        error!("derive_ramfs_guest_path(): {reason}");
        return Err(anyhow::anyhow!(reason));
    }

    let basename: String = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "ramfs".to_string());

    Ok(format!("/{basename}"))
}

///
/// # Description
///
/// Builds a Hyperlight filesystem using pre-built FAT images.
///
/// # Parameters
///
/// - `fat_images`: Slice of (host_path, mount_point) tuples for pre-built FAT images.
/// - `ramfs_filename`: Optional path to a ramfs file to add as read-only overlay.
///
/// # Returns
///
/// Upon successful completion, returns the built HyperlightFSImage. Otherwise, returns an error.
///
fn build_fs_with_prebuilt_fat(
    fat_images: &[(String, String)],
    ramfs_filename: Option<&String>,
) -> Result<HyperlightFSImage> {
    // Start with the first FAT image.
    let (first_fat_path, first_mount_point): &(String, String) = &fat_images[0];
    debug!(
        "build_fs_with_prebuilt_fat(): adding FAT image (path={}, mount_point={})",
        first_fat_path, first_mount_point
    );
    let mut builder: _ =
        HyperlightFSBuilder::new().add_fat_image(first_fat_path, first_mount_point)?;

    // Add additional FAT images.
    for (fat_path, mount_point) in fat_images.iter().skip(1) {
        debug!(
            "build_fs_with_prebuilt_fat(): adding FAT image (path={}, mount_point={})",
            fat_path, mount_point
        );
        builder = builder.add_fat_image(fat_path, mount_point)?;
    }

    // Add read-only ramfs file if specified.
    if let Some(ramfs_path) = ramfs_filename {
        let guest_path: String = derive_ramfs_guest_path(ramfs_path)?;
        debug!(
            "build_fs_with_prebuilt_fat(): adding ramfs (host={}, guest={})",
            ramfs_path, guest_path
        );
        builder = builder.add_file(ramfs_path, &guest_path)?;
    }

    Ok(builder.build()?)
}

///
/// # Description
///
/// Builds a Hyperlight filesystem with an empty FAT mount at root.
/// Calculates the required size based on mounts and ramfs file.
///
/// # Parameters
///
/// - `mounts`: Slice of (host_path, guest_path) tuples to be mounted.
/// - `ramfs_filename`: Optional path to a ramfs file to include in size calculation.
///
/// # Returns
///
/// Upon successful completion, returns the built HyperlightFSImage. Otherwise, returns an error.
///
fn build_fs_with_empty_fat(
    mounts: &[(String, String)],
    ramfs_filename: Option<&String>,
) -> Result<HyperlightFSImage> {
    // Calculate required FAT size based on content to be added.
    let mut content_size: usize = calculate_mount_content_size(mounts)?;

    // Include ramfs file size.
    if let Some(ramfs_path) = ramfs_filename {
        if let Ok(metadata) = std::fs::metadata(ramfs_path) {
            content_size = content_size.saturating_add(metadata.len() as usize);
            debug!(
                "build_fs_with_empty_fat(): ramfs size={} bytes, total content={}",
                metadata.len(),
                content_size
            );
        }
    }

    // Size calculation: at least 1MB, plus 50% overhead for FAT metadata/fragmentation.
    const MIN_FAT_SIZE: usize = 1024 * 1024;
    let fat_size: usize = MIN_FAT_SIZE.max(content_size + (content_size / 2) + MIN_FAT_SIZE);

    debug!("build_fs_with_empty_fat(): creating empty FAT at / ({} bytes)", fat_size);
    Ok(HyperlightFSBuilder::new()
        .add_empty_fat_mount("/", fat_size)?
        .build()?)
}

///
/// # Description
///
/// Sets up the FAT filesystem by creating directories, copying ramfs, and applying mounts.
///
/// # Parameters
///
/// - `fs_image`: Mutable reference to the filesystem image.
/// - `use_prebuilt`: Whether pre-built FAT images were used.
/// - `fat_images`: Slice of pre-built FAT image configurations.
/// - `mounts`: Slice of mount mappings to apply.
/// - `ramfs_filename`: Optional ramfs file to copy (only for non-prebuilt case).
///
/// # Returns
///
/// Upon successful completion, returns empty. Otherwise, returns an error.
///
fn setup_fat_filesystem(
    fs_image: &mut HyperlightFSImage,
    use_prebuilt: bool,
    fat_images: &[(String, String)],
    mounts: &[(String, String)],
    ramfs_filename: Option<&String>,
) -> Result<()> {
    if !use_prebuilt {
        // For empty FAT mount at root: create /data, copy ramfs, apply mounts.
        let fat_image: &mut FatImage = fs_image.fat_mount_mut("/").ok_or_else(|| {
            let reason: String = "FAT mount at / not found".to_string();
            error!("setup_fat_filesystem(): {reason}");
            anyhow::anyhow!(reason)
        })?;

        // Create /data directory for writable storage.
        create_data_directory(fat_image)?;

        // Copy ramfs file into FAT (can't use builder.add_file() with root mount).
        if let Some(ramfs_path) = ramfs_filename {
            let guest_path: String = derive_ramfs_guest_path(ramfs_path)?;
            debug!(
                "setup_fat_filesystem(): copying ramfs (host={}, guest={})",
                ramfs_path, guest_path
            );
            copy_file_to_fat(fat_image, ramfs_path, &guest_path)?;
        }

        // Apply mount mappings.
        if !mounts.is_empty() {
            apply_mounts_to_fat(fat_image, mounts)?;
        }
    } else {
        // For pre-built FAT: create /data directory if root mount exists, then apply mounts.
        if let Some(fat_image) = fs_image.fat_mount_mut("/") {
            create_data_directory(fat_image)?;
        }

        if !mounts.is_empty() {
            apply_mounts_to_prebuilt_fat(fs_image, fat_images, mounts)?;
        }
    }

    debug!("setup_fat_filesystem(): filesystem setup complete");
    Ok(())
}

///
/// # Description
///
/// Creates the /data directory in the FAT filesystem for writable storage.
///
/// # Parameters
///
/// - `fat_image`: Mutable reference to the FAT image.
///
/// # Returns
///
/// Upon successful completion, returns empty. Otherwise, returns an error.
///
fn create_data_directory(fat_image: &mut FatImage) -> Result<()> {
    match fat_image.create_dir("/data") {
        Ok(()) => {
            debug!("create_data_directory(): created /data directory");
            Ok(())
        },
        Err(error) => {
            let error_str: String = error.to_string();
            if error_str.contains("already exists") || error_str.contains("AlreadyExists") {
                debug!("create_data_directory(): /data directory already exists");
                Ok(())
            } else {
                let reason: String = format!("failed to create /data directory (error={error})");
                error!("create_data_directory(): {reason}");
                Err(anyhow::anyhow!(reason))
            }
        },
    }
}

///
/// # Description
///
/// Applies mount mappings to pre-built FAT images, finding the appropriate FAT image.
///
/// # Parameters
///
/// - `fs_image`: Mutable reference to the filesystem image.
/// - `fat_images`: Slice of pre-built FAT image configurations.
/// - `mounts`: Slice of mount mappings to apply.
///
/// # Returns
///
/// Upon successful completion, returns empty. Otherwise, returns an error.
///
fn apply_mounts_to_prebuilt_fat(
    fs_image: &mut HyperlightFSImage,
    fat_images: &[(String, String)],
    mounts: &[(String, String)],
) -> Result<()> {
    // Try root mount first.
    if let Some(fat_image) = fs_image.fat_mount_mut("/") {
        return apply_mounts_to_fat(fat_image, mounts);
    }

    // Fall back to trying each pre-built mount point.
    for (_, mount_point) in fat_images {
        if let Some(fat_image) = fs_image.fat_mount_mut(mount_point) {
            return apply_mounts_to_fat(fat_image, mounts);
        }
    }

    let reason: String = "no suitable FAT mount found for applying mounts".to_string();
    error!("apply_mounts_to_prebuilt_fat(): {reason}");
    Err(anyhow::anyhow!(reason))
}

///
/// # Description
///
/// Calculates the total size of all FAT images to be mounted.
///
/// # Parameters
///
/// - `fat_images`: A slice of (host_path, mount_point) tuples for FAT images.
///
/// # Returns
///
/// Upon successful completion, this function returns the total size in bytes of all FAT images.
/// Otherwise, it returns an error.
///
fn calculate_fat_images_size(fat_images: &[(String, String)]) -> Result<usize> {
    let mut total_size: usize = 0;

    for (fat_path, _mount_point) in fat_images {
        let metadata: std::fs::Metadata = std::fs::metadata(fat_path).map_err(|error| {
            let reason: String =
                format!("failed to get metadata for FAT image (path={fat_path}, error={error})");
            error!("calculate_fat_images_size(): {reason}");
            anyhow::anyhow!(reason)
        })?;

        total_size = total_size.saturating_add(metadata.len() as usize);
    }

    debug!("calculate_fat_images_size(): total FAT size = {} bytes", total_size);
    Ok(total_size)
}

///
/// # Description
///
/// Calculates the total size of content to be mounted from host paths.
///
/// # Parameters
///
/// - `mounts`: A slice of (host_path, guest_path) tuples representing the mounts.
///
/// # Returns
///
/// Upon successful completion, this function returns the total size in bytes of all files to be
/// mounted. Otherwise, it returns an error.
///
fn calculate_mount_content_size(mounts: &[(String, String)]) -> Result<usize> {
    let mut total_size: usize = 0;

    for (host_path, _guest_path) in mounts {
        let metadata: std::fs::Metadata = std::fs::metadata(host_path).map_err(|error| {
            let reason: String =
                format!("failed to get metadata for mount (path={host_path}, error={error})");
            error!("calculate_mount_content_size(): {reason}");
            anyhow::anyhow!(reason)
        })?;

        if metadata.is_file() {
            total_size = total_size.saturating_add(metadata.len() as usize);
        } else if metadata.is_dir() {
            total_size = total_size.saturating_add(calculate_dir_size(host_path)?);
        }
    }

    debug!("calculate_mount_content_size(): total mount content size = {} bytes", total_size);
    Ok(total_size)
}

///
/// # Description
///
/// Recursively calculates the total size of all files in a directory.
///
/// # Parameters
///
/// - `dir_path`: The path to the directory.
///
/// # Returns
///
/// Upon successful completion, this function returns the total size in bytes of all files in the
/// directory. Otherwise, it returns an error.
///
fn calculate_dir_size(dir_path: &str) -> Result<usize> {
    let mut total_size: usize = 0;

    for entry in std::fs::read_dir(dir_path).map_err(|error| {
        let reason: String = format!("failed to read directory (path={dir_path}, error={error})");
        error!("calculate_dir_size(): {reason}");
        anyhow::anyhow!(reason)
    })? {
        let entry: std::fs::DirEntry = entry.map_err(|error| {
            let reason: String = format!("failed to read directory entry (error={error})");
            error!("calculate_dir_size(): {reason}");
            anyhow::anyhow!(reason)
        })?;

        let metadata: std::fs::Metadata = entry.metadata().map_err(|error| {
            let reason: String =
                format!("failed to get metadata (path={}, error={error})", entry.path().display());
            error!("calculate_dir_size(): {reason}");
            anyhow::anyhow!(reason)
        })?;

        if metadata.is_file() {
            total_size = total_size.saturating_add(metadata.len() as usize);
        } else if metadata.is_dir() {
            let subdir_path: String = entry.path().to_string_lossy().into_owned();
            total_size = total_size.saturating_add(calculate_dir_size(&subdir_path)?);
        }
    }

    Ok(total_size)
}

///
/// # Description
///
/// Applies mount mappings by copying host files/directories into the FAT filesystem.
///
/// # Parameters
///
/// - `fat_image`: A mutable reference to the FAT image.
/// - `mounts`: A slice of (host_path, guest_path) tuples representing the mounts.
///
/// # Returns
///
/// Upon successful completion, this function returns empty. Otherwise, it returns an error.
///
fn apply_mounts_to_fat(fat_image: &mut FatImage, mounts: &[(String, String)]) -> Result<()> {
    for (host_path, guest_path) in mounts {
        info!("Hostpath: {}", { host_path });
        let metadata: std::fs::Metadata = std::fs::metadata(host_path).map_err(|error| {
            let reason: String =
                format!("failed to get metadata for mount (path={host_path}, error={error})");
            error!("apply_mounts_to_fat(): {reason}");
            anyhow::anyhow!(reason)
        })?;

        if metadata.is_file() {
            info!("apply_mounts_to_fat(): mounted file (host={host_path}, guest={guest_path})");
            copy_file_to_fat(fat_image, host_path, guest_path)?;
        } else if metadata.is_dir() {
            info!(
                "apply_mounts_to_fat(): mounted directory (host={host_path}, guest={guest_path})"
            );
            copy_dir_to_fat(fat_image, host_path, guest_path)?;
        } else {
            let reason: String =
                format!("mount path is neither a file nor a directory (path={host_path})");
            error!("apply_mounts_to_fat(): {reason}");
            return Err(anyhow::anyhow!(reason));
        }
    }

    Ok(())
}

///
/// # Description
///
/// Copies a single file from the host into the FAT filesystem.
///
/// # Parameters
///
/// - `fat_image`: A mutable reference to the FAT image.
/// - `host_path`: The path to the file on the host.
/// - `guest_path`: The path where the file should appear in the guest.
///
/// # Returns
///
/// Upon successful completion, this function returns empty. Otherwise, it returns an error.
///
fn copy_file_to_fat(fat_image: &mut FatImage, host_path: &str, guest_path: &str) -> Result<()> {
    // Ensure parent directory exists in FAT.
    if let Some(parent) = Path::new(guest_path).parent() {
        let parent_str: &str = parent.to_str().unwrap_or("/");
        if !parent_str.is_empty() && parent_str != "/" {
            ensure_fat_dir_exists(fat_image, parent_str)?;
        }
    }

    // Read host file content.
    let content: Vec<u8> = std::fs::read(host_path).map_err(|error| {
        let reason: String = format!("failed to read host file (path={host_path}, error={error})");
        error!("copy_file_to_fat(): {reason}");
        anyhow::anyhow!(reason)
    })?;

    // Write to FAT filesystem.
    let mut writer = fat_image.create_file(guest_path).map_err(|error| {
        let reason: String =
            format!("failed to create file in FAT (path={guest_path}, error={error})");
        error!("copy_file_to_fat(): {reason}");
        anyhow::anyhow!(reason)
    })?;

    writer.write_all(&content).map_err(|error| {
        let reason: String =
            format!("failed to write file content to FAT (path={guest_path}, error={error})");
        error!("copy_file_to_fat(): {reason}");
        anyhow::anyhow!(reason)
    })?;

    writer.flush().map_err(|error| {
        let reason: String =
            format!("failed to flush file to FAT (path={guest_path}, error={error})");
        error!("copy_file_to_fat(): {reason}");
        anyhow::anyhow!(reason)
    })?;

    debug!(
        "copy_file_to_fat(): copied file (host={host_path}, guest={guest_path}, size={})",
        content.len()
    );
    Ok(())
}

///
/// # Description
///
/// Recursively copies a directory and its contents from the host into the FAT filesystem.
///
/// # Parameters
///
/// - `fat_image`: A mutable reference to the FAT image.
/// - `host_dir`: The path to the directory on the host.
/// - `guest_dir`: The path where the directory should appear in the guest.
///
/// # Returns
///
/// Upon successful completion, this function returns empty. Otherwise, it returns an error.
///
fn copy_dir_to_fat(fat_image: &mut FatImage, host_dir: &str, guest_dir: &str) -> Result<()> {
    // Ensure the target directory exists in FAT.
    ensure_fat_dir_exists(fat_image, guest_dir)?;

    for entry in std::fs::read_dir(host_dir).map_err(|error| {
        let reason: String = format!("failed to read directory (path={host_dir}, error={error})");
        error!("copy_dir_to_fat(): {reason}");
        anyhow::anyhow!(reason)
    })? {
        let entry: std::fs::DirEntry = entry.map_err(|error| {
            let reason: String = format!("failed to read directory entry (error={error})");
            error!("copy_dir_to_fat(): {reason}");
            anyhow::anyhow!(reason)
        })?;

        let host_entry_path: String = entry.path().to_string_lossy().into_owned();
        let entry_name: String = entry.file_name().to_string_lossy().into_owned();
        let guest_entry_path: String =
            format!("{}/{}", guest_dir.trim_end_matches('/'), entry_name);

        let metadata: std::fs::Metadata = entry.metadata().map_err(|error| {
            let reason: String =
                format!("failed to get metadata (path={host_entry_path}, error={error})");
            error!("copy_dir_to_fat(): {reason}");
            anyhow::anyhow!(reason)
        })?;

        if metadata.is_file() {
            copy_file_to_fat(fat_image, &host_entry_path, &guest_entry_path)?;
        } else if metadata.is_dir() {
            copy_dir_to_fat(fat_image, &host_entry_path, &guest_entry_path)?;
        }
        // Skip symlinks and other special files.
    }

    Ok(())
}

///
/// # Description
///
/// Ensures that a directory path exists in the FAT filesystem, creating it and all parent
/// directories if necessary.
///
/// # Parameters
///
/// - `fat_image`: A mutable reference to the FAT image.
/// - `dir_path`: The directory path to ensure exists.
///
/// # Returns
///
/// Upon successful completion, this function returns empty. Otherwise, it returns an error.
///
fn ensure_fat_dir_exists(fat_image: &mut FatImage, dir_path: &str) -> Result<()> {
    let path: &Path = Path::new(dir_path);
    let mut current_path: String = String::new();

    for component in path.components() {
        match component {
            std::path::Component::RootDir => {
                current_path = "/".to_string();
            },
            std::path::Component::Normal(name) => {
                let name_str: &str = name.to_str().ok_or_else(|| {
                    let reason: String = format!("invalid path component (path={dir_path})");
                    error!("ensure_fat_dir_exists(): {reason}");
                    anyhow::anyhow!(reason)
                })?;

                if current_path == "/" {
                    current_path = format!("/{name_str}");
                } else {
                    current_path = format!("{current_path}/{name_str}");
                }

                // Try to create the directory, ignore if it already exists.
                match fat_image.create_dir(&current_path) {
                    Ok(()) => {
                        debug!("ensure_fat_dir_exists(): created directory {current_path}");
                    },
                    Err(error) => {
                        // Ignore "already exists" errors.
                        let error_str: String = error.to_string();
                        if !error_str.contains("already exists")
                            && !error_str.contains("AlreadyExists")
                        {
                            let reason: String = format!(
                                "failed to create directory (path={current_path}, error={error})"
                            );
                            error!("ensure_fat_dir_exists(): {reason}");
                            return Err(anyhow::anyhow!(reason));
                        }
                    },
                }
            },
            _ => {
                // Skip other components like CurDir (.) or ParentDir (..).
            },
        }
    }

    Ok(())
}
