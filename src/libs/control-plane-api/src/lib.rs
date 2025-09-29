// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! This file contains the messages used in the control-plane API between the control-plane
//! (nanvixd), the system VM (linuxd), and the user VM. It defines a simple wire-format to allow
//! for bi-directional communication. The specification for the wire-format is as follows:
//! - source: u8 -> Source of the control-plane message (Nanvixd, SystemVm, or UserVm).
//! - length: u32 LE -> Length of the message payload.
//! - bytes: [u8; length]
//!

use ::anyhow::Result;
use ::num_enum::{
    IntoPrimitive,
    TryFromPrimitive,
};
use ::std::io::{
    Error,
    ErrorKind,
};
use ::syscomm::{
    SocketError,
    SocketStream,
};

#[repr(u8)]
#[derive(Clone, Copy, Debug, IntoPrimitive, TryFromPrimitive)]
pub enum Command {
    Shutdown,
}

///
/// # Description
///
/// This enum indicates which entity is the source of a given control-plane message.
///
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum Source {
    Nanvixd = 0,
    SystemVm = 1,
    UserVm = 2,
}

impl TryFrom<u8> for Kind {
    type Error = SocketError;
    fn try_from(v: u8) -> Result<Self, SocketError> {
        match v {
            0 => Ok(Source::Nanvixd),
            1 => Ok(Source::SystemVm),
            2 => Ok(Source::UserVm),
            _ => Err(Error::new(ErrorKind::InvalidData, "invalid message source").into()),
        }
    }
}

///
/// # Description
///
/// Wire format for all control-plane messages, irrespective of the source.
///
trait WireCommand: Serialize + for<'de> Deserialize<'de> {
    const SOURCE: Source;
}

///
/// # Description
///
/// Default bincode options to encode/decode control-plane messages.
///
fn bincode_opts() -> impl bincode::Options {
    bincode::DefaultOptions::new().with_fixint_encoding().allow_trailing_bytes()
}

///
/// # Description
///
/// Control-plane messages from Nanvixd to the system VM and the user VM. For the moment we do not
/// differentiate between who is the receipient.
///
#[derive(Debug, Serialize, Deserialize)]
pub enum NanvixdCommand {
    Shutdown,
}

impl WireCommand for NanvixdCommand {
    const SOURCE: Source = Source::Nanvixd;
}

///
/// # Description
///
/// Control-plane messages from the system VM to nanvixd.
///
#[derive(Debug, Serialize, Deserialize)]
pub enum SystemVmCommand {
    Shutdown,
}

impl WireCommand for SystemVmCommand {
    const SOURCE: Source = Source::SystemVm;
}

///
/// # Description
///
/// Send a message following the control-plane protocol.
///
/// # Arguments
///
/// - `stream`: the control-plane stream where to send the message.
/// - `msg`: a message that implements the WireMsg trait.
///
/// # Returns
///
/// In case of success, returns nothing. Otherwise, returns an error.
///
pub fn send_typed<T: WireCommand>(stream: &mut SocketStream, msg: &T) -> Result<(), SocketError> {
    let payload: &[u8] = bincode_opts()
        .serialize(msg)
        .map_err(|e| Error::new(ErrorKind::InvalidData, format!("encode failed: {e}")))?;

    let len: u32 = (payload.len() as u32).to_le_bytes();

    // Send source, then payload length, then payload.
    stream.write_all(&[T::SOURCE as u8])?;
    stream.write_all(&len)?;
    if !payload.is_empty() {
        stream.write_all(&payload)?;
    }

    Ok(())
}

///
/// # Description
///
/// Receive a generic control-plane message following the wire-format.
///
/// # Arguments
///
/// - `stream`: the control-plane stream where to send the message.
///
/// # Returns
///
/// In case of success, a message of the indicated trait. Otherwise, an error.
///
pub fn recv_typed<T: WireCommand>(stream: &mut SocketStream) -> Result<T, SocketError> {
    // Read command source.
    let mut source_bytes: [u8; 1] = [0u8; 1];
    let num_read: usize = stream.try_read_exact(&mut source_bytes)?;
    debug_assert_eq!(num_read, 1);

    let source: Source = Source::try_from(source_bytes[0])?;
    if source as u8 != T::SOURCE as u8 {
        return Err(Error::new(ErrorKind::InvalidData, "unexpected kind for T").into());
    }

    // Read command length.
    let mut length_bytes: [u8; 4] = [0u8; 4];
    let num_read = stream.try_read_exact(&mut length_bytes)?;
    debug_assert_eq!(num_read, 4);
    let len: u32 = u32::from_le_bytes(length_bytes) as usize;

    // optional size guard
    const MAX_MSG: usize = 1 << 20;
    if len > MAX_MSG {
        return Err(Error::new(ErrorKind::InvalidData, "message too large").into());
    }

    // Read payload.
    let mut message_buf: Vec<u8> = vec![0u8; len];
    if len > 0 {
        let num_read: usize = stream.try_read_exact(&mut message_buf)?;
        debug_assert_eq!(num_read, len);
    }

    // Decode message.
    bincode_opts()
        .deserialize::<T>(&message_buf)
        .map_err(|e| Error::new(ErrorKind::InvalidData, format!("failed to decode message (error={e:?})")).into())
}

// TODO: consider these tiny shims
/*
 pub fn send_cp(stream: &mut SocketStream, m: &ControlPlaneMessage) -> Result<(), SocketError> {
    send_typed(stream, m)
}
pub fn send_system(stream: &mut SocketStream, m: &SystemVmMessage) -> Result<(), SocketError> {
    send_typed(stream, m)
}
pub fn send_user(stream: &mut SocketStream, m: &UserVmMessage) -> Result<(), SocketError> {
    send_typed(stream, m)
}
*/

// This function is actually used by linuxd, consuming nanvixd as a library. It is never used from
// the main nanvixd binary, hence why we need to add this annotation to silent clippy.
#[allow(dead_code)]
pub fn try_read_command(stream: &mut SocketStream) -> Result<Command, SocketError> {
    let mut buf: [u8; 1] = [0u8; 1];

    // Try read exact returns the number of bytes read `n` with 0 < n <= buf.len(). In this case
    // it is safe to ignore the return value because n can only ever be 1.
    let num_read = stream.try_read_exact(&mut buf)?;
    debug_assert!(num_read == 1);

    Command::try_from(buf[0]).map_err(|_| {
        Error::new(ErrorKind::InvalidData, "error parsing control-plane command".to_string()).into()
    })
}

#[allow(dead_code)]
pub fn send_command(stream: &mut SocketStream, cmd: Command) -> Result<(), SocketError> {
    let byte: u8 = cmd.into();
    stream.write_all(&[byte])?;
    Ok(())
}
