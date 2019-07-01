use std::fmt;
use std::result;

#[derive(Debug, PartialEq)]
pub enum Error {
    InvalidAsync { start: u64, value: u8 },
    TruncatedPacket { start: u64, span: u8 },
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

#[allow(non_camel_case_types)]
pub enum OpCode {
    NULL = 0x0,
    M8 = 0x1,
    MERR = 0x2,
    C8 = 0x3,
    D8 = 0x4,
    D16 = 0x5,
    D32 = 0x6,
    D64 = 0x7,
    D8MTS = 0x8,
    D16MTS = 0x09,
    D32MTS = 0x0A,
    D64MTS = 0x0B,
    D4 = 0x0C,
    D4MTS = 0x0D,
    FLAG_TS = 0x0E,
    VERSION = 0xF00,
}

use self::OpCode::*;

#[derive(Debug, PartialEq)]
pub struct Packet {
    pub start: u64, // Starting offset in nibbles.
    pub span: u8,
    pub details: PacketDetails,
}

#[derive(Debug, PartialEq)]
pub enum PacketDetails {
    Async,
}

use self::PacketDetails::*;

pub type Result = result::Result<Packet, Error>;

enum DecoderState {
    Unsynced,
    OpCode,
    Payload,
    Timestamp,
}

use self::DecoderState::*;

pub struct StpDecoder {
    offset: u64,         // Offset in nibbles.
    f_count: u8,         // Number of consecutive 0xF nibbles.
    state: DecoderState, // The state of the decoder.

    /** Current Packet: **/
    span: u8, // Current packet span in nibbles.
    op_code: Option<OpCode>, // Current packet OpCode.
    payload_sz: u8,          // Current packet payload size (nibbles).
    has_timestamp: bool,     // Current packet has a timestamp.
    payload: u64,            // Current packet payload.
    timestamp: u64,          // Current packet timestamp.
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
            op_code: None,
            payload_sz: 0,
            payload: 0,
            has_timestamp: false,
            timestamp: 0,
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

    fn process_async(&mut self, handler: &mut dyn FnMut(Result)) {
        self.report_truncation(handler);
        handler(Ok(Packet {
            start: self.offset - ASYNC_F_COUNT as u64,
            span: ASYNC_F_COUNT + 1,
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

    fn to_state(&mut self, new_state: DecoderState) {
        match new_state {
            Unsynced | OpCode => self.span = 0,
            Payload => self.payload = 0,
            Timestamp => self.timestamp = 0,
        }
        self.state = new_state;
    }

    fn report_truncation(&mut self, handler: &mut dyn FnMut(Result)) {
        if self.span > 0 {
            handler(Err(TruncatedPacket {
                start: self.offset - self.span as u64,
                span: self.span,
            }));
        }
    }

    fn process(&mut self, nibble: u8, handler: &mut dyn FnMut(Result)) {
        match self.state {
            Unsynced => return,
            OpCode => self.process_opcode(nibble, handler),
            Payload => self.process_payload(nibble, handler),
            Timestamp => return,
        }
    }

    fn process_opcode(&mut self, nibble: u8, _handler: &mut dyn FnMut(Result)) {
        match self.span {
            0 => match nibble {
                0 => return, // NULL packet.
                1 => self.packet_setup(M8, 2, false),
                2 => self.packet_setup(MERR, 2, false),
                3 => self.packet_setup(C8, 2, false),
                _ => return,
            },
            _ => return,
        }

        self.span += 1;
    }

    fn packet_setup(&mut self, op_code: OpCode, payload_sz: u8, has_timestamp: bool) {
        self.op_code = Some(op_code);
        self.payload_sz = payload_sz;
        self.has_timestamp = has_timestamp;

        if payload_sz > 0 {
            self.to_state(Payload);
        } else if has_timestamp == true {
            self.to_state(Timestamp);
        } else {
            panic!("Invalid packet_setup");
        }
    }

    fn process_payload(&mut self, _nibble: u8, _handler: &mut dyn FnMut(Result)) {}
}
