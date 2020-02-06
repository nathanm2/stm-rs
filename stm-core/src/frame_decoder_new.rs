use std::fmt;
use std::result;

#[derive(Debug, PartialEq)]
pub enum ErrorReason {
    InvalidStreamId(u8),
    InvalidAuxByte(u8),
    PartialFrame(usize),
}

use self::ErrorReason::*;

#[derive(Debug, PartialEq)]
pub struct Error {
    pub offset: usize,
    pub reason: ErrorReason,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.reason {
            InvalidStreamId(id) => write!(f, "invalid stream id: {:#x} ({})", id, self.offset),
            InvalidAuxByte(byte) => write!(f, "invalid aux byte: {:#x} ({})", byte, self.offset),
            PartialFrame(size) => write!(f, "partial frame: {} bytes ({})", size, self.offset),
        }
    }
}

pub type Result = result::Result<(), Error>;

/// Decode a single frame of data.
///
/// # Arguments
///
///  * `frame` - The frame of data to be decoded.
///  * `stream` - The starting stream ID.
///
pub fn decode_frame(frame: &[u8; 16], stream: Option<u8>) -> Result {
    Ok(())
}
