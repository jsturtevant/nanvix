// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::platform,
    PERF_IKC_MESSAGES_RECEIVED,
};
use ::core::{
    mem,
    sync::atomic::Ordering,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::{
        Message,
        MessageType,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Writes an inter-kernel communication message to the kernel's standard output.
///
/// # Parameters
///
/// - `message`: Message to write.
///
/// # Returns
///
/// Upon success, empty is returned. Upon failure, an error is returned instead.
///
pub fn write(message: Message) -> Result<(), Error> {
    // Only handle IKC messages.
    if { message.message_type } != MessageType::Ikc {
        let reason: &str = "unsupported message type";
        error!("{reason}");
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    // Parse the message payload to check if it's a stdout/stderr write.
    // The payload contains a LinuxDaemonMessage which has a header and payload.
    // For WriteRequest: header (1 byte) + fd (4 bytes) + count (4 bytes) + buffer.
    // We need to check if fd is 1 (stdout) or 2 (stderr).
    const HEADER_OFFSET: usize = 0;
    const FD_OFFSET: usize = 1; // After 1-byte header.
    const STDOUT_FILENO: i32 = 1;
    const STDERR_FILENO: i32 = 2;
    const WRITE_REQUEST_HEADER: u8 = 0; // LinuxDaemonMessageHeader::WriteRequest = 0.

    let payload: &[u8] = &message.payload;

    // Extract header from the payload (little-endian u16 at offset 0).
    let header_bytes: [u8; 2] = [payload[HEADER_OFFSET], payload[HEADER_OFFSET + 1]];
    let header_value: u16 = u16::from_le_bytes(header_bytes);

    // Log every IKC message with its header type for debugging.
    trace!(
        "stdio::write(): IKC message header={}, source={:?}, dest={:?}",
        header_value,
        { message.source },
        { message.destination }
    );

    // Check if this is a WriteRequest (header byte == 0).
    if payload[HEADER_OFFSET] != WRITE_REQUEST_HEADER {
        // Not a WriteRequest - drop the message (other IPC is disabled).
        trace!(
            "stdio::write(): dropping non-WriteRequest message (header={})",
            header_value
        );
        return Ok(());
    }

    // Extract fd from the payload (little-endian i32 at offset 1).
    let fd_bytes: [u8; 4] = [
        payload[FD_OFFSET],
        payload[FD_OFFSET + 1],
        payload[FD_OFFSET + 2],
        payload[FD_OFFSET + 3],
    ];
    let fd: i32 = i32::from_le_bytes(fd_bytes);

    // Only forward stdout and stderr writes to the host.
    if fd != STDOUT_FILENO && fd != STDERR_FILENO {
        // Not stdout/stderr - drop the message (other IPC is disabled).
        trace!(
            "stdio::write(): dropping WriteRequest for non-stdout/stderr (fd={})",
            fd
        );
        return Ok(());
    }

    let bytes: [u8; mem::size_of::<Message>()] = message.to_bytes();

    // Write message to the kernel's standard output.
    // SAFETY: The standard output is present, initialized and thread-safe to write.
    unsafe {
        // NOTE: we assume that page is tagged as writethrough-enabled and cache-disabled.
        platform::vmbus_write(&bytes as *const u8);
    }

    Ok(())
}

///
/// # Description
///
/// Reads an inter-kernel communication message from the kernel's standard input.
///
/// # Returns
///
/// Upon success, this function either returns the message read or `None` if there are no more
/// messages.  Upon failure, an error is returned instead.
///
pub fn read() -> Result<Option<Message>, Error> {
    const NBYTES: usize = core::mem::size_of::<Message>();
    let mut message: [u8; NBYTES] = [0; NBYTES];

    cfg_if::cfg_if! {
        if #[cfg(feature = "microvm")] {
            // Read credits register.
            let credits: u32 = unsafe {
                core::ptr::read_volatile(::config::microvm::DEFAULT_MICROVM_CTRL_CREDITS as *const u32)
            };
        }
        else if #[cfg(feature = "hyperlight")] {
            // Read credits register.
            let credits: u64 = unsafe {
                crate::hal::platform::hyperlight::peb::ProcessEnvironmentBlock::get_credits()?
            };
        }
    }

    // No message available.
    if credits == 0 {
        return Ok(None);
    }

    // Read message from the kernel's standard input.
    // SAFETY: The standard input is present, initialized and thread-safe to read.
    unsafe {
        // NOTE: we assume that page is tagged as writethrough-enabled and cache-disabled.
        platform::vmbus_read(&mut message as *mut u8);
    };

    PERF_IKC_MESSAGES_RECEIVED.fetch_add(1, Ordering::Relaxed);

    // Convert message to Message struct.
    match Message::try_from_bytes(message) {
        Ok(message) => Ok(Some(message)),
        // No message available.
        Err(e) if e.code == ErrorCode::NoMessageAvailable => Ok(None),
        Err(e) => {
            warn!("{e:?}");
            Err(e)
        },
    }
}
