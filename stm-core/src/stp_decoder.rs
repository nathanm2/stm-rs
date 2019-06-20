use std::fmt;
use std::result;

#[derive(Debug, PartialEq)]
pub enum Error {
    InvalidAsync {
        nibble_offset: u64,
        invalid_nibble: u8,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            InvalidAsync {
                nibble_offset,
                invalid_nibble,
            } => write!(
                f,
                "Invalid async packet. Nibble offset: {}, Nibble: {}",
                nibble_offset, invalid_nibble
            ),
        }
    }
}

use self::Error::*;

pub struct Packet {
    pub offset: u64,
    pub lenth: u64,
    pub ptype: PacketType,
}

pub enum PacketType {
    Async,
}

pub type Result = result::Result<Packet, Error>;

enum DecoderState {
    Unsynced,
    OpCode,
    Payload,
}

use self::DecoderState::*;

pub struct StpDecoder {
    nibble_offset: u64,  // Offset in nibbles.
    f_count: u8,         // Number of consecutive 0xF nibbles.
    state: DecoderState, // The state of the decoder.
    packet_start: u64,
}

const ASYNC_F_COUNT: u8 = 21;

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

    pub fn decode_nibbles<F>(&mut self, nibbles: &[u8], mut handler: F)
    where
        F: FnMut(Result),
    {
        for nibble in nibbles {
            self.decode_nibble(*nibble, &mut handler);
        }
    }

    fn decode_nibble(&mut self, nibble: u8, handler: &mut dyn FnMut(Result)) {
        // An ASYNC can appear anywhere within the stream, so every 0xf nibble needs to be buffered
        // just in case it's part of one.
        if nibble == 0xf {
            if self.f_count < ASYNC_F_COUNT {
                self.f_count += 1;
            } else {
                self.process(0xf, handler);
            }
        } else {
            if self.f_count == ASYNC_F_COUNT {
                if nibble == 0 {
                    self.process_async(handler);
                } else {
                    self.process_invalid_async(nibble, handler);
                }
            } else {
                for _ in 0..self.f_count {
                    self.process(0xf, handler);
                }
                self.process(nibble, handler);
            }
            self.f_count = 0;
        }
        self.nibble_offset += 1;
    }

    fn process_async(&mut self, _handler: &mut dyn FnMut(Result)) {
        self.state = OpCode;
    }

    fn process_invalid_async(&mut self, nibble: u8, handler: &mut dyn FnMut(Result)) {
        self.state = Unsynced;
        handler(Err(InvalidAsync {
            nibble_offset: self.nibble_offset - ASYNC_F_COUNT as u64,
            invalid_nibble: nibble,
        }));
    }

    fn process(&mut self, _nibble: u8, _handler: &mut dyn FnMut(Result)) {
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
