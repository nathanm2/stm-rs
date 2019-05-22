use std::fmt;
use std::result;

#[derive(Debug, PartialEq)]
pub enum Error {
    InvalidAsync { nibble_offset: u64, terminal: u8 },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            InvalidAsync { nibble_offset, .. } => {
                write!(f, "invalid async packet at nibble offset {}", nibble_offset)
            }
        }
    }
}

use self::Error::*;

pub struct Packet {
    pub offset: u64,
}

pub type Result = result::Result<Packet, Error>;

enum DecoderState {
    Unsynced,
    OpCode,
    Payload,
}

use self::DecoderState::*;

const ASYNC_F_COUNT: u8 = 21;

pub struct StpDecoder {
    nibble_offset: u64, // Offset in nibbles.
    f_count: u8,        // Number of consecutive 0xF nibbles.
    state: DecoderState,
    packet_start: u64,
}

impl StpDecoder {
    /// Create a new StpDecoder.
    pub fn new() -> Self {
        StpDecoder {
            nibble_offset: 0,
            f_count: 0,
            state: Unsynced,
            packet_start: 0,
        }
    }

    pub fn decode_bytes<F>(&mut self, bytes: &[u8], mut handler: F)
    where
        F: FnMut(Result),
    {
        for byte in bytes {
            self.decode_nibble(byte & 0xF, &mut handler);
            self.decode_nibble(byte >> 4, &mut handler);
        }
    }

    pub fn decode_nibbles<F>(&mut self, nibbles: &[u8], mut handler: F)
    where
        F: FnMut(Result),
    {
        for nibble in nibbles {
            self.decode_nibble(*nibble, &mut handler);
        }
    }

    fn decode_nibble<F>(&mut self, nibble: u8, mut handler: F)
    where
        F: FnMut(Result),
    {
        // An ASYNC can appear anywhere within the stream, so we need to check for it first:
        self.async_filter(nibble, &mut handler);
        self.nibble_offset += 1;
    }

    fn async_filter<F>(&mut self, nibble: u8, mut handler: F)
    where
        F: FnMut(Result),
    {
        if nibble == 0xf {
            if self.f_count < ASYNC_F_COUNT {
                self.f_count += 1;
            } else {
                self.process(0xf, &mut handler);
            }
        } else {
            if nibble == 0 && self.f_count == ASYNC_F_COUNT {
                self.process_async(&mut handler);
            } else {
                for _ in 0..self.f_count {
                    self.process(0xf, &mut handler);
                }
                self.process(nibble, &mut handler);
            }
            self.f_count = 0;
        }
    }

    fn process_async<F>(&mut self, mut _handler: F)
    where
        F: FnMut(Result),
    {
        self.state = OpCode;
    }

    fn process<F>(&mut self, _nibble: u8, mut _handler: F)
    where
        F: FnMut(Result),
    {
        match self.state {
            Unsynced => (),
            OpCode => (),
            Payload => (),
        }
    }

    fn to_opcode(&mut self) {
        self.state = OpCode;
        self.packet_start = self.nibble_offset;
    }
}
