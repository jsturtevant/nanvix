// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    event::EventManager,
    kcall::{
        KcallArgs,
        KcallResult,
    },
    pm::{
        self,
        ProcessManager,
        SleepError,
    },
};
use ::sys::{
    error::Error,
    ipc::{
        Message,
        MessageSender,
        MessageType,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn do_send(pm: &mut ProcessManager, message: Message) -> Result<(), Error> {
    trace!("src={:?}, dst={:?}", { message.source }, { message.destination });

    // TODO: Check if source process has permission to send message to destination process.

    // Post message.
    EventManager::post_message(pm, message.destination, message)
}

pub fn send(pm: &mut ProcessManager, args: &KcallArgs) -> KcallResult {
    let src_pid: ProcessIdentifier = args.pid;
    let src_tid: ThreadIdentifier = args.tid;

    // Copy message to kernel space.
    let mut message: Message = Message::default();
    if let Err(e) = pm::copy_from_user(pm, src_pid, &mut message, args.arg0 as *const Message) {
        return KcallResult::Error(e.code.into());
    }

    // Check if message source is invalid.
    if { message.source } != MessageSender::from(src_tid) && { message.source }
        != MessageSender::from(src_pid)
    {
        let reason: &str = "invalid message source";
        error!("{reason:?} (message={message:?})");
    }

    // Log IKC messages for debugging (extract header from payload).
    if { message.message_type } == MessageType::Ikc {
        let header_bytes: [u8; 2] = [message.payload[0], message.payload[1]];
        let header_value: u16 = u16::from_le_bytes(header_bytes);
        trace!(
            "send(): IKC message from tid={:?}, pid={:?}, header={}",
            src_tid,
            src_pid,
            header_value
        );
    }

    // Route message based on its type.
    match message.message_type {
        // Inter-kernel communication.
        MessageType::Ikc => {
            cfg_if::cfg_if! {
                // Check if standard input/output is available.
                if #[cfg(feature = "stdio")] {
                    // It is, so write message to standard output.
                    match crate::stdio::write(message) {
                        Ok(_) => KcallResult::ok(),
                        Err(e) => KcallResult::Error(e.code.into()),
                    }
                } else {
                    // Standard input/output is not available.
                    error!("stdio is not available");
                    KcallResult::Error(sys::error::ErrorCode::ProtocolNotSupported.into())
                }
            }
        },
        // Local-host communication.
        _ => {
            // Post message.
            match do_send(pm, message) {
                Ok(()) => KcallResult::ok(),
                Err(e) => KcallResult::Error(e.code.into()),
            }
        },
    }
}

pub unsafe fn recv(
    tid: ThreadIdentifier,
    pid: ProcessIdentifier,
    msg: usize,
) -> Result<(), SleepError> {
    if pid != ProcessIdentifier::INITD {
        trace!("recv(): blocking tid={:?}, pid={:?}", tid, pid);
    }

    match EventManager::wait(tid, pid) {
        Ok(message) => {
            if pid != ProcessIdentifier::INITD {
                trace!(
                    "recv(): unblocked tid={:?}, pid={:?}, msg_type={:?}, status={}",
                    tid,
                    pid,
                    { message.message_type },
                    { message.status }
                );
            }
            pm::copy_to_user(ProcessManager::get_mut(), pid, msg as *mut Message, &message)
                .map_err(SleepError::Generic)
        },
        Err(sleep_error) => Err(sleep_error),
    }
}
