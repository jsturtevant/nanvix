// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::std::{
    io::{
        Error,
        ErrorKind,
    },
    mem,
};
use ::sys::ipc::Message;
use ::syscomm::{
    SocketError,
    SocketStream,
};

//==================================================================================================
// Structure
//==================================================================================================

pub struct Gateway {
    stream: SocketStream,
}

impl Gateway {
    ///
    /// # Description
    ///
    /// Creates a new gateway.
    ///
    /// # Parameters
    ///
    /// - `stream`: Socket stream to be used by the gateway.
    ///
    /// # Returns
    ///
    /// A new gateway instance.
    ///
    pub fn new(stream: SocketStream) -> Self {
        Gateway { stream }
    }

    ///
    /// # Description
    ///
    /// Attempts to send a message to the gateway.
    ///
    /// # Parameters
    ///
    /// - `message`: Message to be sent.
    ///
    /// # Returns
    ///
    /// If the message was successfully sent, `Ok(())` is returned. Otherwise, an error is returned.
    ///
    pub fn try_send(&mut self, message: Message) -> Result<(), SocketError> {
        let bytes: [u8; mem::size_of::<Message>()] = message.to_bytes();
        match self.stream.write_all(&bytes) {
            Ok(_) => Ok(()),
            Err(e) => {
                // Print error messages only if it is not a WouldBlock error to avoid spamming the logs.
                if e.kind() != ErrorKind::WouldBlock {
                    let reason: String = format!("failed to send message ({e:?}");
                    error!("send(): {reason}");
                }

                Err(e)
            },
        }
    }

    ///
    /// # Description
    ///
    /// Attempts to receive a message from the gateway.
    ///
    /// # Returns
    ///
    /// Upon success, the received message is returned. Otherwise, an error is returned.
    ///
    pub fn try_receive(&mut self) -> Result<Message, SocketError> {
        let mut bytes: [u8; mem::size_of::<Message>()] = [0; mem::size_of::<Message>()];
        match self.stream.read_exact(&mut bytes) {
            Ok(_) => {
                let mut message: Message = match Message::try_from_bytes(bytes) {
                    Ok(message) => message,
                    Err(e) => {
                        let reason: String = format!("failed to parse message ({e:?})");
                        error!("receive(): {reason}");
                        return Err(SocketError::new(Error::new(ErrorKind::InvalidData, reason)));
                    },
                };

                // TODO: delete me
                // Add timestamp
                trace!("adding timestamp");
                let this_step: usize = 2;
                // Offset is for the LinuxDaemonHeader + count field in ReadMessage
                let this_offset: usize = 6 + 2 * this_step;
                let now = std::time::SystemTime::now();
                let duration = now.duration_since(std::time::UNIX_EPOCH).expect("Time went backwards");
                let timestamp_micros = duration.as_micros();
                let timestamp_u16 = (timestamp_micros & 0xFFFF) as u16;
                let timestamp_bytes = timestamp_u16.to_be_bytes();
                message.payload[this_offset..this_offset + 2].copy_from_slice(&timestamp_bytes);

                Ok(message)
            },
            Err(e) => {
                // Print error messages only if it is not a WouldBlock error to avoid spamming the logs.
                if e.kind() != ErrorKind::WouldBlock {
                    let reason: String = format!("failed to receive message ({e:?})");
                    error!("receive(): {reason}");
                }

                Err(SocketError::new(e))
            },
        }
    }
}
