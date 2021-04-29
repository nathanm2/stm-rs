//! TWP error type.

use std::fmt;
use std::result;

#[derive(Debug, PartialEq)]
pub struct Error {
    /// The type of error.
    pub kind: ErrorKind,
    /// The stream offset where the error was encountered.
    pub offset: usize,
}

#[derive(Debug, PartialEq)]
pub enum ErrorKind {
    /// An invalid Stream ID was encountered.
    InvalidStreamId(u8),
    /// The frame's AUX byte is invalid.
    InvalidAuxByte(u8),
    /// The frame is less than sixteen bytes.
    PartialFrame(usize),
    Stop,
}

pub type Result<S> = result::Result<S, Error>;

use self::ErrorKind::*;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}, offset: {:#x}", self.kind, self.offset)
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            InvalidStreamId(id) => write!(f, "invalid stream id: {:#x}", id),
            InvalidAuxByte(byte) => write!(f, "invalid aux byte: {:#x}", byte),
            PartialFrame(size) => write!(f, "truncated frame: {} bytes", size),
            Stop => write!(f, "stopped"),
        }
    }
}
