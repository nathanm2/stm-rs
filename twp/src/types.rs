//! TWP error type.

use std::fmt;
use std::result;

#[derive(Debug, PartialEq)]
pub enum ErrorReason {
    InvalidStreamId(u8),
    InvalidAuxByte(u8),
    PartialFrame(usize),
    Stop,
}

use self::ErrorReason::*;

impl fmt::Display for ErrorReason {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            InvalidStreamId(id) => write!(f, "invalid stream id: {:#x}", id),
            InvalidAuxByte(byte) => write!(f, "invalid aux byte: {:#x}", byte),
            PartialFrame(size) => write!(f, "truncated frame: {} bytes", size),
            Stop => write!(f, "stopped"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Error {
    pub offset: usize,
    pub reason: ErrorReason,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}, offset: {:#x}", self.reason, self.offset)
    }
}

pub type Result<S> = result::Result<S, Error>;

pub struct Data {
    pub id: Option<u8>,
    pub data: u8,
    pub offset: usize,
}
