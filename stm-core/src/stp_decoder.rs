use std::fmt;
use std::result;

#[derive(Debug)]
pub enum Error {
    InvalidAsync { offset: u64, value: u8 },
}

use self::Error::*;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            InvalidAsync { offset, value } => write!(
                f,
                "Invalid async packet. Offset: {}, Value: {}",
                offset, value
            ),
        }
    }
}

pub struct Packet {
    pub start: u64, // Starting offset in nibbles.
    pub span: u64,
    pub details: PacketDetails,
}

pub enum PacketDetails {
    Async,
}

use self::PacketDetails::*;

pub type Result = result::Result<Packet, Error>;

enum DecoderState {
    Unsynced,
    OpCode,
    Payload,
}

use self::DecoderState::*;

pub struct StpDecoder {
    offset: u64,         // Offset in nibbles.
    f_count: u8,         // Number of consecutive 0xF nibbles.
    state: DecoderState, // The state of the decoder.
    packet_len: u64,
}

const ASYNC_F_COUNT: u8 = 21;

impl StpDecoder {
    /// Create a new StpDecoder.
    pub fn new() -> Self {
        StpDecoder {
            offset: 0,
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
        self.offset += 1;
    }

    fn enter_state(&mut self, state: DecoderState) {
        match state {
            OpCode
    }
    fn process_async(&mut self, handler: &mut dyn FnMut(Result)) {
        self.state = OpCode;
        
        handler(Ok(Packet {
            start: self.offset - ASYNC_F_COUNT as u64,
            span: (ASYNC_F_COUNT + 1) as u64,
            details: Async,
        }));
    }

    fn process_invalid_async(&mut self, value: u8, handler: &mut dyn FnMut(Result)) {
        self.state = Unsynced;
        handler(Err(InvalidAsync {
            offset: self.offset,
            value: value,
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
        self.packet_start = self.offset;
    }
}
