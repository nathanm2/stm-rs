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
    D16MTS = 0x9,
    D32MTS = 0xA,
    D64MTS = 0xB,
    D4 = 0xC,
    D4MTS = 0xD,
    FLAG_TS = 0xE,
    M16 = 0xF1,
    GERR = 0xF2,
    C16 = 0xF3,
    D8TS = 0xF4,
    D16TS = 0xF5,
    D32TS = 0xF6,
    D64TS = 0xF7,
    D8M = 0xF8,
    D16M = 0xF9,
    D32M = 0xFA,
    D64M = 0xFB,
    D4TS = 0xFC,
    D4M = 0xFD,
    FLAG = 0xFE,
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

#[derive(PartialEq)]
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
        // until a either a non-0xf nibble is encountered, or ASYNC_F_COUNT number of 0xf nibbles
        // are encountered.  In the latter case we can process the overflow.
        if nibble == 0xf {
            if self.f_count < ASYNC_F_COUNT {
                self.f_count += 1;
            } else {
                self.process_nibble(0xf, handler);
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
                    self.process_nibble(0xf, handler);
                }
                self.process_nibble(nibble, handler);
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
        if self.state != Unsynced && self.span > 0 {
            handler(Err(TruncatedPacket {
                start: self.offset - self.span as u64,
                span: self.span,
            }));
        }
    }

    fn process_nibble(&mut self, nibble: u8, handler: &mut dyn FnMut(Result)) {
        self.span += 1;
        match self.state {
            Unsynced => return,
            OpCode => self.process_opcode(nibble, handler),
            Payload => self.process_payload(nibble, handler),
            Timestamp => return,
        }
    }

    fn process_opcode(&mut self, nibble: u8, _handler: &mut dyn FnMut(Result)) {
        match self.span {
            0 => panic!("Unexpected span"),
            1 => match nibble {
                0x0 => self.to_state(OpCode), // NULL packet.
                0x1 => self.data_setup(M8, 2, false),
                0x2 => self.data_setup(MERR, 2, false),
                0x3 => self.data_setup(C8, 2, false),
                0x4 => self.data_setup(D8, 2, false),
                0x5 => self.data_setup(D16, 4, false),
                0x6 => self.data_setup(D32, 8, false),
                0x7 => self.data_setup(D64, 16, false),
                0x8 => self.data_setup(D8MTS, 2, true),
                0x9 => self.data_setup(D16MTS, 4, true),
                0xA => self.data_setup(D32MTS, 8, true),
                0xB => self.data_setup(D64MTS, 8, true),
                0xC => self.data_setup(D4, 1, false),
                0xD => self.data_setup(D4MTS, 1, true),
                0xE => self.data_setup(FLAG_TS, 0, true),
                0xF => return,
                _ => panic!("Invalid nibble value: {}", nibble),
            },
            2 => match nibble {
                0x0 => return,
                0x1 => self.data_setup(M16, 4, false),
                0x2 => self.data_setup(GERR, 2, false),
                0x3 => self.data_setup(C16, 4, false),
                0x4 => self.data_setup(D8TS, 2, true),
                0x5 => self.data_setup(D16TS, 4, true),
                0x6 => self.data_setup(D32TS, 8, true),
                0x7 => self.data_setup(D64TS, 16, true),
                0x8 => self.data_setup(D8M, 2, false),
                0x9 => self.data_setup(D16M, 4, false),
                0xA => self.data_setup(D32M, 8, false),
                0xB => self.data_setup(D64M, 8, false),
                0xC => self.data_setup(D4TS, 1, true),
                0xD => self.data_setup(D4M, 1, false),
                0xE => self.report_packet(FLAG),
                _ => panic!("Invalid nibble value: {}", nibble),

            _ => return,
        }
    }

    fn data_setup(&mut self, op_code: OpCode, payload_sz: u8, has_timestamp: bool) {
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
