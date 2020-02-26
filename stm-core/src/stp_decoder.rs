use crate::stp;
use std::result;

#[derive(Debug, PartialEq)]
pub enum ErrorReason {
    InvalidAsync,
    TruncatedPacket { opcode: Option<stp::OpCode> },
    InvalidOpCode { value: u16 },
}

#[derive(Debug, PartialEq)]
pub struct Error {
    pub reason: ErrorReason,
    pub start: usize,
    pub span: usize,
}

pub struct Packet {
    pub packet: stp::Packet, // Packet type.
    pub start: usize,        // Starting nibble offset.
    pub span: usize,         // Size of the packet in nibbles.
}

pub type Result = result::Result<Packet, Error>;

struct DataFragment {
    data_sz: u8,
    has_timestamp: bool,
    data: u64,
    timestamp_sz: u8,
    timestamp: u64,
}

impl DataFragment {
    pub fn new(data_sz: u8, has_timestamp: bool) -> Self {
        return DataFragment {
            data_sz,
            has_timestamp,
            data: 0,
            timestamp_sz: 0,
            timestamp: 0,
        };
    }
}

enum DecoderState {
    Unsynced, // The decoder is looking for a SYNC packet.
    OpCode,   // Processing an opcode.
    Version,
    Data(DataFragment),
}

use self::DecoderState::*;

pub struct StpDecoder {
    state: DecoderState,         // The state of the decoder.
    offset: usize,               // Offset in nibbles.
    f_count: u8,                 // Number of consecutive 0xF nibbles.
    span: usize,                 // Current packet span.
    opcode: Option<stp::OpCode>, // Current opcode.
}

impl StpDecoder {
    /// Create a new StpDecoder.
    pub fn new() -> Self {
        return StpDecoder {
            state: Unsynced,
            offset: 0,
            f_count: 0,
            span: 0,
            opcode: None,
        };
    }

    /// Decode a stream of bytes.
    pub fn decode_bytes<F>(&mut self, bytes: &[u8], mut handler: F)
    where
        F: FnMut(Result),
    {
        for byte in bytes {
            self.decode_nibble(byte & 0xF, &mut handler);
            self.decode_nibble(byte >> 4, &mut handler);
        }
    }

    /// Decode a stream of nibbles.
    pub fn decode_nibbles<F>(&mut self, nibbles: &[u8], mut handler: F)
    where
        F: FnMut(Result),
    {
        for nibble in nibbles {
            self.decode_nibble(*nibble, &mut handler);
        }
    }

    /// Decode a nibble.
    pub fn decode_nibble<F>(&mut self, nibble: u8, mut handler: F)
    where
        F: FnMut(Result),
    {
        assert!(nibble & 0xF0 == 0x00);
    }
}
