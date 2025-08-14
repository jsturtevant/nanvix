// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![allow(clippy::module_inception)]

//==================================================================================================
// Modules
//==================================================================================================

mod io;
mod kvm;
mod microvm;
mod pal;

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(target_os = "linux")]
extern crate kvm_bindings;
#[cfg(target_os = "linux")]
extern crate kvm_ioctls;

use crate::vmm::microvm::{
    io::{
        ControlCommandResponse,
        IoThread,
        IoThreadControlCommand,
    },
    kvm::vmem::VirtualMemory,
    microvm::MicroVm,
};
use ::anyhow::Result;
use ::libc::{
    pthread_self,
    pthread_kill,
};
use ::mio::Waker;
use ::std::{
    fs::File,
    io::Write,
    mem,
    sync::{
        Arc,
        Barrier,
        Mutex,
        MutexGuard,
        atomic::{
            AtomicUsize,
            Ordering,
        },
        mpsc,
        mpsc::{
            Receiver,
            RecvError,
            Sender,
            TryRecvError,
        },
    },
    thread::JoinHandle,
    time::Instant,
};
use ::sys::ipc::{
    Message,
    MessageType,
};
use ::syscomm::SocketStream;

//==================================================================================================
// Constants
//==================================================================================================

// This value was chosen so it catches issues without polluting the logs with too many warnings.
const TIMEOUT_WARNING_INTERVAL_IN_MS: usize = 10;

//==================================================================================================
// Structure
//==================================================================================================

pub struct Vmm {
    _gateway_tx: Sender<Message>,
    io_thread: Option<JoinHandle<Result<()>>>,
    _memory_thread: JoinHandle<Result<()>>,
    vcpu_thread: JoinHandle<Result<u16>>,
    vcpu_thread_id: Arc<AtomicUsize>,
    _microvm: Arc<Mutex<MicroVm>>,
    control_input_rx: Receiver<ControlCommand>,
    control_output_tx: Sender<ControlCommandResponse>,
    orchestrator_state: OrchestratorState,
}

///
/// # Description
///
/// States relating to snapshots functionality. Snapshots may be loaded at PreBoot, and created at Paused.
///
#[derive(PartialEq)]
enum OrchestratorState {
    PreBoot,
    Running,
    Paused,
}

///
/// # Description
///
/// Control plane commands.
///
#[derive(PartialEq)]
pub enum ControlCommand {
    _StartMicroVm,
    _LoadSnapshotAndRun,
    _PauseMicroVm,
    _PauseAndCreateSnapshot,
    _CreateSnapshot,
    _ResumeMicroVm,
    LinuxDaemonFlushed,
    /// VM has powered-off.
    MicroVmPowerOff,
    /// Send a shutdown command to the VM.
    Shutdown,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Vmm {
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
    /// - `control_plane_stream`: An optional connection to the control-plane for communication with nanvix.
    /// - `system_vm_stream`: An optional connection to the system VM.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns the exit status of the virtual machine.
    /// Otherwise, it returns an error.
    ///
    pub fn spawn(
        memory_size: usize,
        kernel_filename: &str,
        initrd_filename: Option<String>,
        initrd_args: Option<String>,
        stderr: Option<String>,
        control_plane_stream: Option<SocketStream>,
        system_vm_stream: Option<SocketStream>,
    ) -> Result<u16> {
        crate::timer!("vmm_creation");

        let (vm_tx, gateway_rx) = mpsc::channel::<Message>();
        let (gateway_tx, memory_thread_rx) = mpsc::channel::<Message>();
        let (memory_thread_tx, vm_rx) = mpsc::channel::<Message>();
        let (control_input_tx, control_input_rx) = mpsc::channel::<ControlCommand>();
        let (control_output_tx, control_output_rx) = mpsc::channel::<ControlCommandResponse>();
        let (io_thread_control_tx, io_thread_control_rx) =
            mpsc::channel::<IoThreadControlCommand>();

        // Spawn I/O thread. We need an I/O thread if either the system VM and/or control-plane are
        // configured.
        let (io_thread, io_thread_waker): (Option<JoinHandle<Result<()>>>, Option<Arc<Waker>>) =
            if system_vm_stream.is_some() || control_plane_stream.is_some() {
                let (io_thread, io_thread_waker): (JoinHandle<Result<()>>, Arc<Waker>) =
                    IoThread::spawn(
                        system_vm_stream,
                        control_plane_stream,
                        gateway_rx,
                        gateway_tx.clone(),
                        control_input_tx.clone(),
                        control_output_rx,
                        io_thread_control_rx,
                    )?;
                (Some(io_thread), Some(io_thread_waker))
            } else {
                (None, None)
            };

        // Input function used for emulating I/O port reads.
        let input: Box<microvm::InputFn> = Self::build_input_fn(vm_rx);

        // Output function used for emulating I/O port writes.
        let output: Box<microvm::OutputFn> = Self::build_output_fn(
            Self::get_stderr_writer(stderr.clone())?,
            vm_tx,
            io_thread_waker.clone(),
        );

        let mut microvm: MicroVm = MicroVm::new(memory_size, input, output)?;

        let rip: u64 = microvm.load_kernel(kernel_filename)?;
        if let Some(ref initrd_filename) = initrd_filename {
            microvm.load_initrd(initrd_filename)?;

            // Write arguments to the virtual machine. For now, just pass the initrd filename.
            let mut args: String = initrd_filename
                .split('/')
                .next_back()
                .unwrap_or(initrd_filename)
                .to_string();

            // Add initrd arguments if provided.
            if let Some(ref initrd_args) = initrd_args {
                args.push_str(&format!(" {initrd_args}"));
            }

            microvm.write_args(&args)?;
        }

        microvm.reset(rip)?;

        let microvm: Arc<Mutex<MicroVm>> = Arc::new(Mutex::new(microvm));

        let vmem: Arc<Mutex<VirtualMemory>> = microvm
            .lock()
            .map_err(|e| anyhow::anyhow!("failed to acquire lock {e:?}"))?
            .vmem();

        vmem.lock()
            .map_err(|e| anyhow::anyhow!("failed to acquire lock {e:?}"))?
            .reset_credits()?;

        // Create a thread that reads from vm_rx and writes to vm_rx2.
        let memory_thread_tx: Sender<Message> = memory_thread_tx.clone();
        let memory_thread: JoinHandle<Result<(), anyhow::Error>> = std::thread::spawn(move || {
            loop {
                match memory_thread_rx.recv() {
                    Ok(mut msg) => {
                        profiler::timestamp_message!(
                            &mut msg.payload,
                            mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                                + mem::offset_of!(syscall::unistd::message::ReadResponse, buffer)
                        );
                        if let Err(e) = memory_thread_tx.send(msg) {
                            let reason: String = format!("failed to send message: {e:?}");
                            error!("{reason}");
                            continue;
                        }
                        vmem.lock()
                            .map_err(|e| anyhow::anyhow!("failed to acquire lock {e:?}"))?
                            .add_credit()?;
                    },
                    Err(RecvError) => {
                        // When the guest finishes , the vCPU thread will disconnect from this
                        // thread. This situation is normal and should not create an error log.
                        debug!("memory_thread(): channel has been disconnected");
                        break Ok(());
                    },
                }
            }
        });

        // We use an atomic to pass the id of the created thread back to the caller context. We
        // need this because std::thread's JoinHandle does not expose the tid. We synchronize the
        // update using a barrier.
        let pthread_id_holder: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let barrier: Arc<Barrier> = Arc::new(Barrier::new(2));

        let microvm_clone: Arc<Mutex<MicroVm>> = microvm.clone();
        let barrier_clone: Arc<Barrier> = Arc::clone(&barrier);
        let control_input_tx_clone: Sender<ControlCommand> = control_input_tx.clone();
        let pthread_id_holder_clone: Arc<AtomicUsize> = pthread_id_holder.clone();
        let vcpu_thread: JoinHandle<Result<u16>> = std::thread::spawn(move || {
            // Store the tid so that the caller can send signals to the vCPU thread.
            // SAFETY: we are calling pthread_self() right after creating the thread so this is
            // safe.
            let pthread_id: libc::pthread_t = unsafe { pthread_self() };
            pthread_id_holder_clone.store(pthread_id as usize, Ordering::Relaxed);

            // Notify the outside thread that the thread id is ready.
            barrier_clone.wait();

            let join_handle = microvm_clone
                .lock()
                .map_err(|e| anyhow::anyhow!("failed to acquire lock {e:?}"))?
                .run();

            // After the VM is done running, send a notification to the VMM.
            control_input_tx_clone.send(ControlCommand::MicroVmPowerOff)?;

            join_handle
        });

        // Wait right after spawning the vCPU thread such that we populate the pthread id holder
        // before actually starting the vCPU.
        barrier.wait();

        let mut vmm: Vmm = Self {
            _gateway_tx: gateway_tx,
            io_thread,
            _memory_thread: memory_thread,
            vcpu_thread,
            vcpu_thread_id: pthread_id_holder,
            _microvm: microvm,
            control_input_rx,
            control_output_tx,
            orchestrator_state: OrchestratorState::PreBoot,
        };

        // Main control loop for the VMM thread.
        if let Err(e) = vmm.control_loop() {
            error!("VMM exit control-loop with error: {e:?}");
        }

        // After the VMM thread has finished (i.e. vCPU thread has exitted) shutdown the I/O thread.
        if let Some(waker) = &io_thread_waker {
            io_thread_control_tx.send(IoThreadControlCommand::Shutdown)?;
            waker.clone().wake()?;
        }

        if let Some(io_thread) = vmm.io_thread {
            if let Err(e) = io_thread.join() {
                // This is a fatal error, but continue with the clean-up.
                error!("failed to join I/O thread (error={e:?})");
            }
        }

        match vmm.vcpu_thread.join() {
            Ok(exit_code) => exit_code,
            Err(e) => {
                let reason: String = format!("failed to join vCPU thread (error={e:?})");
                error!("run(): {reason}");
                anyhow::bail!(reason)
            },
        }
    }

    ///
    /// # Description
    ///
    /// Obtains a buffered writer for the virtual machine's standard error device. If the standard
    /// error device is set to a file, the function attempts to open the file and create a buffered
    /// writer. If the standard error device is not set to a file, the function falls back to stderr.
    ///
    /// # Parameters
    ///
    /// * `vm_stderr` - The path to the file where the standard error device is set.
    ///
    /// # Returns
    ///
    /// On success, the function returns a buffered writer for the virtual machine's standard error
    ///
    fn get_stderr_writer(vm_stderr: Option<String>) -> Result<Box<dyn Write>> {
        // Obtain a buffered writer for the virtual machine's standard error device.
        let file_writer: Box<dyn Write> = if let Some(vm_stderr) = vm_stderr {
            // Standard error was set to a file. Attempt to open file and create a writer.
            let file = File::options()
                .read(false)
                .write(true)
                .create(true)
                .truncate(true)
                .open(&vm_stderr)?;
            Box::new(file)
        } else {
            // Standard error was not set to a file. Fallback to stderr.
            Box::new(std::io::stderr())
        };
        Ok(file_writer)
    }

    fn build_input_fn(input_queue: Receiver<Message>) -> Box<microvm::InputFn> {
        // Input function used for emulating I/O port reads.
        let input = move |vmem: &Arc<Mutex<VirtualMemory>>, data, size| -> Result<()> {
            // Check for invalid operand size.
            if size != mem::size_of::<u32>() {
                let reason: String = format!("invalid operand size (size={size:?})");
                error!("input(): {reason}");
                anyhow::bail!(reason);
            }

            match input_queue.recv() {
                Ok(mut msg) => {
                    log::trace!("build_input_fn: {msg:?}");
                    profiler::timestamp_message!(
                        &mut msg.payload,
                        mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                            + mem::offset_of!(syscall::unistd::message::ReadResponse, buffer)
                    );
                    msg.message_type = MessageType::Ikc;
                    let mut locked_vm: MutexGuard<'_, VirtualMemory> = vmem
                        .lock()
                        .map_err(|e| anyhow::anyhow!("failed to acquire lock {e:?}"))?;
                    profiler::timestamp_message!(
                        &mut msg.payload,
                        mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                            + mem::offset_of!(syscall::unistd::message::ReadResponse, buffer)
                    );
                    locked_vm.write_bytes(data as u64, &msg.to_bytes())?;
                    locked_vm.consume_credit().unwrap();
                },
                // Channel has disconnected.
                Err(RecvError) => {
                    let reason: String = "channel has been disconnected".to_string();
                    error!("input(): {reason}");
                    anyhow::bail!(reason);
                },
            }

            Ok(())
        };

        Box::new(input)
    }

    fn build_output_fn(
        mut file_writer: Box<dyn Write>,
        queue: Sender<Message>,
        io_thread_waker: Option<Arc<Waker>>,
    ) -> Box<microvm::OutputFn> {
        let io_thread_waker = io_thread_waker.clone();
        // Output function used for emulating I/O port writes.
        let output = move |vm: &Arc<Mutex<VirtualMemory>>, data, size| -> Result<()> {
            // Parse operand size do determine how to handle the operation.
            if size == 1 {
                // Write to the standard error device.

                // Convert data to a character.
                let ch: char = match char::from_u32(data) {
                    // Valid character.
                    Some(ch) => ch,
                    // Invalid character.
                    None => {
                        let reason: String = format!("invalid character (data={data:?})");
                        error!("output(): {reason}");
                        anyhow::bail!(reason);
                    },
                };

                let buf: &[u8] = &[ch as u8];

                file_writer.write_all(buf)?;

                Ok(())
            } else {
                // Write to the standard output device.
                let mut bytes: [u8; mem::size_of::<Message>()] = [0; mem::size_of::<Message>()];
                vm.lock()
                    .map_err(|e| anyhow::anyhow!("failed to acquire lock {e:?}"))?
                    .read_bytes(data as u64, &mut bytes)?;

                let mut message: Message = match Message::try_from_bytes(bytes) {
                    Ok(message) => message,
                    Err(err) => {
                        let reason: String = format!("failed to parse message: {err:?}");
                        error!("output(): {reason}");
                        anyhow::bail!(reason);
                    },
                };
                profiler::timestamp_message!(
                    &mut message.payload,
                    std::mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                        + std::mem::offset_of!(syscall::unistd::message::WriteRequest, buffer)
                );

                log::info!("queueing message: {message:?}");
                if let Err(e) = queue.send(message) {
                    let reason: String = format!("failed to send message: {e:?}");
                    error!("output(): {reason}");
                    anyhow::bail!(reason);
                }

                // Notify the I/O thread that it has a pending message.
                if let Some(waker) = &io_thread_waker {
                    log::info!("wake wake!");
                    waker.clone().wake()?;
                }

                Ok(())
            }
        };

        Box::new(output)
    }

    ///
    /// # Description
    ///
    /// Main control loop for the VMM thread. We monitor the control queue where both the vCPU and
    /// I/O threads send commands.
    ///
    fn control_loop(&mut self) -> Result<()> {
        info!("VMM entering control loop...");

        'control_loop: loop {
            match self.control_input_rx.recv() {
                Ok(command) => match command {
                    ControlCommand::_StartMicroVm => {
                        if self.orchestrator_state == OrchestratorState::PreBoot {
                            // TODO: separate starting logic from `spawn()` and put it here
                            // This TODO could be done right now, but it's a major refactor.
                            self.orchestrator_state = OrchestratorState::Running;
                            trace!("OrchestratorState: PreBoot -> Running");
                        }
                    },
                    ControlCommand::_LoadSnapshotAndRun => {
                        if self.orchestrator_state == OrchestratorState::PreBoot {
                            // TODO: load snapshot
                            // This TODO requires being able to create snapshots.

                            // The Linux daemon should send messages to PreBoot VMMs by default,
                            // so there's no need to tell it to resume sending messages.

                            if let Err(e) = self.resume_microvm() {
                                let reason: String =
                                    format!("LoadSnapshotAndRun: failed to resume microvm: {e:?}");
                                error!("handle_command(): {reason}");
                                return Err(anyhow::anyhow!(reason));
                            }
                            trace!("OrchestratorState: PreBoot -> Running");
                        }
                    },
                    ControlCommand::_PauseMicroVm => {
                        if self.orchestrator_state == OrchestratorState::Running {
                            if let Err(e) = self.pause_protocol() {
                                let reason: String =
                                    format!("PauseMicroVm: failed to pause microvm: {e:?}");
                                error!("handle_command(): {reason}");
                                return Err(anyhow::anyhow!(reason));
                            }
                        }
                    },
                    ControlCommand::_PauseAndCreateSnapshot => {
                        if self.orchestrator_state == OrchestratorState::Running {
                            if let Err(e) = self.pause_protocol() {
                                let reason: String =
                                    format!("PauseAndCreateSnapshot: failed to pause microvm: {e:?}");
                                error!("handle_command(): {reason}");
                                return Err(anyhow::anyhow!(reason));
                            }
                            if let Err(e) = self.create_snapshot() {
                                let reason: String =
                                    format!("PauseAndCreateSnapshot: failed to create snapshot: {e:?}");
                                error!("handle_command(): {reason}");
                                return Err(anyhow::anyhow!(reason));
                            }
                        }
                    },
                    ControlCommand::_CreateSnapshot => {
                        if self.orchestrator_state == OrchestratorState::Paused {
                            if let Err(e) = self.create_snapshot() {
                                let reason: String =
                                    format!("CreateSnapshot: failed to create snapshot: {e:?}");
                                error!("handle_command(): {reason}");
                                return Err(anyhow::anyhow!(reason));
                            }
                        }
                    },
                    ControlCommand::_ResumeMicroVm => {
                        if self.orchestrator_state == OrchestratorState::Paused {
                            // TODO: tell linuxd it's fine to send more messages
                            // This TODO requires having a control plane connection with linuxd
                            if let Err(e) = self.resume_microvm() {
                                let reason: String =
                                    format!("ResumeMicroVm: failed to resume microvm: {e:?}");
                                error!("handle_command(): {reason}");
                                return Err(anyhow::anyhow!(reason));
                            }
                            trace!("OrchestratorState: Paused -> Running");
                        }
                    },
                    ControlCommand::LinuxDaemonFlushed => {
                        // NOTE: this will be unreachable once the communication is fully implemented
                        // `LinuxDaemonFlushed` should only be sent in the middle of `pause_protocol`.
                        // In fact, it should already be unreachable, but it cannot be tested ATM.
                    },
                    // Notification fromm the vCPU that it has been powered off.
                    ControlCommand::MicroVmPowerOff => {
                        // The micro VM has been powered off, exit the loop.
                        trace!("vCPU poweroff");
                        break 'control_loop;
                    },
                    // External request to shut-down the VM.
                    ControlCommand::Shutdown => {
                        // Send a signal to the vCPU thread, and wait for it to send the micro VM
                        // power-off command indicating a graceful shutdown.
                        //
                        // If this call does not succeed, error-out as the vCPU will never
                        // gracefully shut-down.
                        self.interrupt_vcpu()?;
                    }
                },
                Err(RecvError) => {
                    let reason: String =
                        ("disconnected from the input control command channel").to_string();
                    error!("handle_command(): {reason}");
                    return Err(anyhow::anyhow!(reason));
                },
            }
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Attempts to pause the execution of the MicroVM and the communication with the Linux daemon.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Otherwise, an error is returned.
    ///
    pub fn interrupt_vcpu(&self) -> Result<()> {
        let raw_tid = self.vcpu_thread_id.load(Ordering::Relaxed);

        if raw_tid == 0 {
            let reason = "trying to stop vcpu thread with tid 0";
            error!("{reason}");
            return Err(anyhow::anyhow!(reason));
        }

        // SAFETY: we call pthread_kill on a non-zero TID after we have managed to send a message
        // to its reception queue, so the thread is alive and safe.
        let pthread_id = raw_tid as libc::pthread_t;
        unsafe { pthread_kill(pthread_id, microvm::INTERRUPT_SIGNAL) };

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Attempts to pause the execution of the MicroVM and the communication with the Linux daemon.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Otherwise, an error is returned.
    ///
    fn pause_protocol(&mut self) -> Result<()> {
        // TODO: pause MicroVM (Running -> Paused)
        // and tell linuxd to flush (Running -> Flushing)
        // This TODO requires pausing the vCPU and a control plane communication with linuxd
        trace!("MicroVM paused");
        // Flush output to linuxd
        self.control_output_tx
            .send(ControlCommandResponse::FlushOutput)?;
        // TODO: tell linuxd to stop sending messages (Flushing -> Paused)
        // TODO: get a response from linuxd
        // These TODOs require a control plane communication with linuxd
        self.control_output_tx
            .send(ControlCommandResponse::FlushInput)?;
        self.receive_linux_daemon_flushed()?;
        self.orchestrator_state = OrchestratorState::Paused;
        self.control_output_tx
            .send(ControlCommandResponse::MicroVmPaused)?;
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Attempts to receive a `LinuxDaemonFlushed` message from the control input.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Otherwise, an error is returned instead.
    ///
    fn receive_linux_daemon_flushed(&mut self) -> Result<()> {
        // Check how long it takes to receive a response
        let start: Instant = Instant::now();
        let mut counter: usize = 1;
        // Loop until `LinuxDaemonFlushed` arrives.
        // Different kinds of messages can be ignored,
        // as they wouldn't do anything while the VMM is pausing.
        while match self.control_input_rx.try_recv() {
            Ok(command) => command != ControlCommand::LinuxDaemonFlushed,
            Err(TryRecvError::Empty) => true,
            Err(TryRecvError::Disconnected) => {
                let reason: String = "the vmm has disconnected".to_string();
                error!("receive_linux_daemon_flushed(): {reason}");
                anyhow::bail!(reason)
            },
        } {
            // Log a warning and increment the counter every TIMEOUT_WARNING_INTERVAL_IN_MS ms.
            let elapsed_time: usize = start.elapsed().as_millis() as usize;
            if elapsed_time > TIMEOUT_WARNING_INTERVAL_IN_MS * counter {
                warn!(
                    "{}ms have passed waiting for `LinuxDaemonFlushed`",
                    TIMEOUT_WARNING_INTERVAL_IN_MS * counter
                );
                counter += 1;
            }
        }
        Ok(())
    }

    fn create_snapshot(&self) -> Result<()> {
        // TODO: create snapshot
        trace!("Snapshot created");
        self.control_output_tx
            .send(ControlCommandResponse::SnapshotCreated)?;
        Ok(())
    }

    fn resume_microvm(&self) -> Result<()> {
        // TODO: resume MicroVM
        trace!("MicroVM resumed");
        Ok(())
    }
}
