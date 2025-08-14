// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    Gateway,
    vmm::ControlCommand,
};
use ::anyhow::Result;
use ::mio::{
    Events,
    Interest,
    Poll,
    Token,
    Waker,
};
use ::std::{
    collections::VecDeque,
    io::ErrorKind,
    sync::{
        Arc,
        mpsc::{
            Receiver,
            Sender,
            TryRecvError,
        },
    },
    thread::{
        self,
        JoinHandle,
    },
};
use ::sys::ipc::Message;
use ::syscomm::SocketStream;

//==================================================================================================
// Constants
//==================================================================================================

/// Token represnting an event notification from the control-plane socket.
const CONTROL_PLANE_TOKEN: Token = Token(0);
/// Token represnting an event notification from inbound queues from the VM.
const WAKER_TOKEN: Token = Token(1);
/// Token represnting an event notification from the system VM socket.
const SYSTEM_VM_TOKEN: Token = Token(2);

//==================================================================================================
// Structure
//==================================================================================================

///
/// # Description
///
/// Private data of the I/O thread.
///
pub struct IoThread {
    /// Poll structure to monitor connections and queues.
    poll: Poll,
    /// Connection to the system VM.
    system_vm_stream: Option<SocketStream>,
    /// Connection to the control-plane.
    // TODO: the I/O thread still cannot consume events from the control-plane.
    #[allow(dead_code)]
    control_plane_stream: Option<SocketStream>,
    /// Gateway receiver.
    microvm_rx: Receiver<Message>,
    /// Gateway sender.
    microvm_tx: Sender<Message>,
    /// Queue of incoming messages.
    incoming: VecDeque<Message>,
    /// Queue of outgoing messages.
    outgoing: VecDeque<Message>,
    /// Command sender to the VMM.
    _vmm_control_tx: Sender<ControlCommand>,
    /// Response receiver from the VMM.
    vmm_control_rx: Receiver<ControlCommandResponse>,
    /// Receiver for control commands from the VMM to the I/O thread.
    io_thread_control_rx: Receiver<IoThreadControlCommand>,
    /// Waker to notify the IO thread that it has messages to read from its monitored queues.
    waker: Arc<Waker>,
    /// Buffer to handle partial reads from the system VM stream.
    partial_read_buffer: VecDeque<u8>,
}

//==================================================================================================
// Enums
//==================================================================================================

///
/// # Description
///
/// I/O thread control plane command responses.
///
#[derive(PartialEq)]
pub enum IoThreadControlCommand {
    Shutdown,
}

///
/// # Description
///
/// Control plane command responses.
///
#[derive(PartialEq)]
pub enum ControlCommandResponse {
    MicroVmPaused,
    SnapshotCreated,
    FlushOutput,
    FlushInput,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl IoThread {
    ///
    /// # Description
    ///
    /// Spawns a new I/O thread.
    ///
    /// # Parameters
    ///
    /// - `system_vm_stream`: Connection to system VM.
    /// - `control_plane_stream`: Connection to control-plane.
    /// - `microvm_rx`: MicroVM receiver.
    /// - `microvm_tx`: MicroVM sender.
    /// - `vmm_control_tx`: Command sender.
    /// - `vmm_control_rx`: Response receiver.
    /// - `io_thread_control_rx`: Receiver for control commands for the I/O thread itself.
    ///
    /// # Returns
    ///
    /// A handle to the I/O thread.
    ///
    pub fn spawn(
        system_vm_stream: Option<SocketStream>,
        control_plane_strem: Option<SocketStream>,
        microvm_rx: Receiver<Message>,
        microvm_tx: Sender<Message>,
        vmm_control_tx: Sender<ControlCommand>,
        vmm_control_rx: Receiver<ControlCommandResponse>,
        io_thread_control_rx: Receiver<IoThreadControlCommand>,
    ) -> Result<(JoinHandle<Result<()>>, Arc<Waker>)> {
        let mut io_thread: IoThread = IoThread::new(
            system_vm_stream,
            control_plane_strem,
            microvm_rx,
            microvm_tx,
            vmm_control_tx,
            vmm_control_rx,
            io_thread_control_rx,
        )?;
        let waker: Arc<Waker> = io_thread.waker();

        let io_thread_handle = thread::spawn(move || {
            io_thread.run()?;
            Ok(())
        });

        Ok((io_thread_handle, waker))
    }

    ///
    /// # Description
    ///
    /// Creates a new I/O thread.
    ///
    /// # Parameters
    ///
    /// - `system_vm_stream`: Connection to system VM.
    /// - `control_plane_stream`: Connection to control-plane.
    /// - `microvm_rx`: MicroVM receiver.
    /// - `microvm_tx`: MicroVM sender.
    /// - `vmm_control_tx`: Command sender.
    /// - `vmm_control_rx`: Response receiver.
    /// - `io_thread_control_rx`: Receiver for control commands for the I/O thread itself.
    ///
    /// # Returns
    ///
    /// Upon success, a new I/O thread is returned. Otherwise, an error is returned.
    ///
    fn new(
        mut system_vm_stream: Option<SocketStream>,
        mut control_plane_stream: Option<SocketStream>,
        microvm_rx: Receiver<Message>,
        microvm_tx: Sender<Message>,
        vmm_control_tx: Sender<ControlCommand>,
        vmm_control_rx: Receiver<ControlCommandResponse>,
        io_thread_control_rx: Receiver<IoThreadControlCommand>,
    ) -> Result<Self> {
        let poll: Poll = Poll::new()?;

        // Register system VM and/or control-plane streams. At least one should be present
        // otherwise we would not have spawned the I/O thread.
        if let Some(system_vm_stream) = system_vm_stream.as_mut() {
            poll.registry()
                .register(system_vm_stream, SYSTEM_VM_TOKEN, Interest::READABLE)?;
        }
        if let Some(control_plane_stream) = control_plane_stream.as_mut() {
            poll.registry().register(
                control_plane_stream,
                CONTROL_PLANE_TOKEN,
                Interest::READABLE,
            )?;
        }

        // Register a waker token such that other components can notify us about pending work.
        let waker = Waker::new(poll.registry(), WAKER_TOKEN)?;

        Ok(Self {
            poll,
            system_vm_stream,
            control_plane_stream,
            microvm_rx,
            microvm_tx,
            incoming: VecDeque::new(),
            outgoing: VecDeque::new(),
            _vmm_control_tx: vmm_control_tx,
            vmm_control_rx,
            io_thread_control_rx,
            waker: Arc::new(waker),
            partial_read_buffer: VecDeque::new(),
        })
    }

    ///
    /// # Description
    ///
    /// Runs the I/O thread according to the state in the snapshotting protocol state machine.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Otherwise, an error is returned instead.
    ///
    fn run(&mut self) -> Result<()> {
        let mut events: Events = Events::with_capacity(config::syscomm::MAX_NUM_POLL_EVENTS);
        let system_vm_stream: &mut SocketStream =
            self.system_vm_stream.as_mut().ok_or_else(|| {
                let reason: String = "tried to read from system VM but socket is None".to_string();
                error!("{reason}");
                anyhow::anyhow!("{reason}")
            })?;

        'main_loop: loop {
            self.poll.poll(&mut events, None)?;

            // We must drain each socket/queue until they WouldBlock in order to not miss any
            // messages.
            for event in events.iter() {
                match event.token() {
                    // Prioritize events from the control-plane.
                    CONTROL_PLANE_TOKEN => {
                        // TODO: missing logic to read from the control-plane (nanvixd) and send
                        // them to the VMM.

                        // Now send the control-command responses from the VMM back to nanvixd.
                        // self.try_receive_from_vmm_control()?;
                        // TODO: missing sending reply to nanvixd.
                    },

                    // Then check if there are internal events we need to react to.
                    WAKER_TOKEN => {
                        // First drain the actual I/O thread control queue.
                        match self.io_thread_control_rx.try_recv() {
                            Ok(cmd) => match cmd {
                                IoThreadControlCommand::Shutdown => {
                                    break 'main_loop;
                                },
                            },
                            // No messages, continue to next queue.
                            Err(TryRecvError::Empty) => {},
                            Err(TryRecvError::Disconnected) => {
                                // This is a fatal error.
                                error!("I/O thread control queue disconnected");
                                break 'main_loop;
                            },
                        }

                        // Drain messages from the VM output queue, and forward them to the system
                        // VM.
                        loop {
                            match self.microvm_rx.try_recv() {
                                Ok(mut message) => {
                                    profiler::timestamp_message!(
                                        &mut message.payload,
                                        std::mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                                            + std::mem::offset_of!(
                                                syscall::unistd::message::WriteRequest,
                                                buffer
                                            )
                                    );
                                    log::info!("sending message: {message:?}");
                                    let bytes: [u8; std::mem::size_of::<Message>()] =
                                        message.to_bytes();
                                    if let Err(e) = system_vm_stream.write_all(&bytes) {
                                        let reason: String =
                                            format!("cannot write to the user VM stream (error={e:?})");
                                        error!("{reason}");
                                        return Err(anyhow::anyhow!(reason));
                                    }
                                    log::info!("sent!");
                                },
                                // No more messages to drain.
                                Err(TryRecvError::Empty) => break,
                                Err(TryRecvError::Disconnected) => {
                                    let reason: String = "the microvm has disconnected".to_string();
                                    // When the guest finishes , the vCPU thread will disconnect from this thread. This
                                    // situation is normal and should not create an error log.
                                    debug!("{reason}");
                                    return Err(anyhow::anyhow!(reason));
                                },
                            }
                        }
                    },

                    // Finally check if we have input coming from the system VM.
                    SYSTEM_VM_TOKEN => {
                        // Drain messages from the system VM, and queue them to the VM's input
                        // queue.
                        loop {
                            match Gateway::try_receive(
                                system_vm_stream,
                                &mut self.partial_read_buffer,
                            ) {
                                Ok(mut message) => {
                                    profiler::timestamp_message!(
                                        &mut message.payload,
                                        std::mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                                            + std::mem::offset_of!(
                                                syscall::unistd::message::ReadResponse,
                                                buffer
                                            )
                                    );
                                    self.microvm_tx.send(message)?;
                                },
                                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                                Err(e) if e.kind() == ErrorKind::ConnectionReset => {
                                    // This is a fatal error.
                                    error!("system vm disconnected");
                                    break 'main_loop;
                                }
                                Err(e) => {
                                    let reason: String = format!(
                                        "failed to receive message from the system VM \
                                         (error={e:?})"
                                    );
                                    error!("{reason}");
                                    return Err(anyhow::anyhow!(reason));
                                },
                            }
                        }
                    },

                    token => {
                        // Log the error, but not fatal.
                        error!("received notification from un-registered token (token={token:?})");
                    },
                }
            }
        }

        info!("I/O thread shutting down");
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Attempts to receive a message from the gateway.
    ///
    /// # Returns
    ///
    /// Upon success, the received message is pushed into the `incoming` queue, and `true` is returned.
    /// Otherwise, if it would block, `false` is returned. Otherwise, an error is returned.
    ///
    fn try_receive_from_gateway(&mut self) -> Result<bool> {
        let system_vm_stream: &mut SocketStream =
            self.system_vm_stream.as_mut().ok_or_else(|| {
                let reason: String = "tried to read from system VM but socket is None".to_string();
                error!("{reason}");
                anyhow::anyhow!("{reason}")
            })?;

        match Gateway::try_receive(system_vm_stream, &mut self.partial_read_buffer) {
            Ok(message) => {
                self.incoming.push_back(message);
                Ok(true)
            },
            Err(e) => {
                if e.kind() == ErrorKind::WouldBlock {
                    Ok(false)
                } else {
                    let reason: String =
                        format!("failed to receive message from the gateway (error={e:?})");
                    error!("try_receive_from_gateway(): {reason}");
                    anyhow::bail!(reason)
                }
            },
        }
    }

    ///
    /// # Description
    ///
    /// Attempts to receive a message from the MicroVM.
    ///
    /// # Returns
    ///
    /// Upon success, the received message is pushed into the `outgoing` queue, and `true`is returned.
    /// Otherwise, if the channel is empty, `false` is returned. Otherwise, an error is returned.
    ///
    fn try_receive_from_microvm(&mut self) -> Result<bool> {
        match self.microvm_rx.try_recv() {
            Ok(mut message) => {
                profiler::timestamp_message!(
                    &mut message.payload,
                    std::mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                        + std::mem::offset_of!(syscall::unistd::message::WriteRequest, buffer)
                );
                self.outgoing.push_back(message);
                Ok(true)
            },
            Err(TryRecvError::Empty) => Ok(false),
            Err(TryRecvError::Disconnected) => {
                let reason: String = "the microvm has disconnected".to_string();
                // When the guest finishes , the vCPU thread will disconnect from this thread. This
                // situation is normal and should not create an error log.
                debug!("try_receive_from_microvm(): {reason}");
                anyhow::bail!(reason)
            },
        }
    }

    ///
    /// # Description
    ///
    /// Attempts to send a message to the gateway.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Otherwise, an error is returned.
    ///
    fn try_send_to_gateway(&mut self) -> Result<()> {
        log::info!("have {} messages queued!", self.outgoing.len());
        match self.outgoing.pop_front() {
            Some(message) => {
                let mut message_clone: Message = message.clone();
                profiler::timestamp_message!(
                    &mut message_clone.payload,
                    std::mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                        + std::mem::offset_of!(syscall::unistd::message::WriteRequest, buffer)
                );
                let system_vm_stream: &mut SocketStream =
                    self.system_vm_stream.as_mut().ok_or_else(|| {
                        let reason: String =
                            "tried to read from system VM but socket is None".to_string();
                        error!("{reason}");
                        anyhow::anyhow!("{reason}")
                    })?;

                // TODO: move Gateway::try_send to this module.
                match Gateway::try_send(system_vm_stream, message_clone) {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        if e.kind() == ErrorKind::WouldBlock {
                            log::error!("would block!!");
                            self.outgoing.push_front(message);
                            Ok(())
                        } else {
                            let reason: String =
                                format!("failed to send message to the gateway (error={e:?})");
                            error!("try_send_to_gateway(): {reason}");
                            anyhow::bail!(reason)
                        }
                    },
                }
            },
            None => Ok(()),
        }
    }

    ///
    /// # Description
    ///
    /// Attempts to send a message to the MicroVM.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Otherwise, an error is returned.
    ///
    fn try_send_to_microvm(&mut self) -> Result<()> {
        match self.incoming.pop_front() {
            Some(mut message) => {
                profiler::timestamp_message!(
                    &mut message.payload,
                    std::mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                        + std::mem::offset_of!(syscall::unistd::message::ReadResponse, buffer)
                );
                // NOTE: calling `send()` on a channel does not block.
                self.microvm_tx.send(message)?;
                Ok(())
            },
            None => Ok(()),
        }
    }

    ///
    /// # Description
    ///
    /// Attempts to receive a response from the VMM.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Otherwise, an error is returned.
    ///
    fn try_receive_from_vmm_control(&mut self) -> Result<()> {
        match self.vmm_control_rx.try_recv() {
            Ok(response) => match response {
                ControlCommandResponse::FlushOutput => self.flush_microvm_output(),
                ControlCommandResponse::FlushInput => self.flush_linuxd_input(),
                _ => Ok(()), // TODO: forward to whoever is interested. This requires having control channels.
            },
            Err(TryRecvError::Empty) => Ok(()),
            Err(TryRecvError::Disconnected) => {
                let reason: String = "the vmm has disconnected".to_string();
                // When the guest finishes , the vCPU thread will disconnect from this thread. This
                // situation is normal and should not create an error log.
                anyhow::bail!(reason)
            },
        }
    }

    /// # Description
    ///
    /// Attempts to flush all outstanding output from the MicroVM to the Linux daemon.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Otherwise, an error is returned.
    ///
    fn flush_microvm_output(&mut self) -> Result<()> {
        while self.try_receive_from_microvm()? {
            // Keep looping until `microvm_rx` is empty, which breaks the loop.
        }
        while !self.outgoing.is_empty() {
            self.try_send_to_gateway()?;
        }
        Ok(())
    }

    /// # Description
    ///
    /// Attempts to flush all outstanding input from the the Linux daemon to the MicroVM.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Otherwise, an error is returned.
    ///
    fn flush_linuxd_input(&mut self) -> Result<()> {
        while self.try_receive_from_gateway()? {
            // Keep looping until receiving from the gateway would block, which breaks the loop.
        }
        Ok(())
    }

    /// # Description
    ///
    /// Get the waker token to notify the I/O thread that there is an event that it must reac to.
    /// An event normally means that a message has been pushed to one of its monitored queues.
    ///
    /// # Returns
    ///
    /// Return a handle to the waker object.
    ///
    fn waker(&self) -> Arc<Waker> {
        self.waker.clone()
    }
}
