use std::fmt;
use std::result;

#[derive(Debug)]
pub enum Error {
    InvalidAsync { start: u64, value: u8 },
    TruncatedPacket { start: u64, span: u64 },
}

use self::Error::*;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            InvalidAsync { start, value } => write!(
                f,
                "Invalid async packet. Start: {}, Value: {}",
                start, value
            ),
            TruncatedPacket { start, span } => {
                write!(f, "Truncated packet. Start: {}, Span: {}", start, span)
            }
        }
    }
}

pub enum OpCode {
    NULLOpCode = 0x0,
    M8 = 0x1,
    MERR = 0x2,
    C8 = 0x3,
}

use self::OpCode::*;

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
    span: u64,           // Current packet span in nibbles.
}

const ASYNC_F_COUNT: u8 = 21;

impl StpDecoder {
    /// Create a new StpDecoder.
    pub fn new() -> Self {
        StpDecoder {
            offset: 0,
            f_count: 0,
            state: Unsynced,
            span: 0,
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

    fn to_state(&mut self, new_state: DecoderState) {
        match new_state {
            Unsynced | OpCode => self.span = 0,
            Payload => (),
        }
        self.state = new_state;
    }

    fn report_truncation(&mut self, handler: &mut dyn FnMut(Result)) {
        if self.span > 0 {
            handler(Err(TruncatedPacket {
                start: self.offset - self.span,
                span: self.span,
            }));
        }
    }

    fn process_async(&mut self, handler: &mut dyn FnMut(Result)) {
        self.report_truncation(handler);
        handler(Ok(Packet {
            start: self.offset - ASYNC_F_COUNT as u64,
            span: (ASYNC_F_COUNT + 1) as u64,
            details: Async,
        }));
        self.to_state(OpCode);
    }

    fn process_invalid_async(&mut self, value: u8, handler: &mut dyn FnMut(Result)) {
        self.report_truncation(handler);
        handler(Err(InvalidAsync {
            start: self.offset - ASYNC_F_COUNT as u64,
            value: value,
        }));
        self.to_state(Unsynced);
    }

    fn process(&mut self, nibble: u8, handler: &mut dyn FnMut(Result)) {
        match self.state {
            Unsynced => (),
            OpCode => self.process_opcode(nibble, handler),
            Payload => (),
        }
    }

    fn process_opcode(&mut self, nibble: u8, handler: &mut dyn FnMut(Result)) {
        match self.span {
            0 => match nibble {
                0 => return;  // NULL packet.
                1 => self.packet_setup(M8, 2, false);
                2 => self.packet_setup(MERR, 2, false);
                3 => self.packet_setup(C8, 2, false);
                }
            }
        }
    }
