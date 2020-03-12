use crate::nibble::swap_nibbles;
use crate::stp::{self, OpCode::*, StpVersion::*, TimestampType::*};
use std::result;

#[derive(Debug, PartialEq)]
pub enum ErrorReason {
    InvalidAsync { bad_nibble: u8 },
    TruncatedPacket { opcode: Option<stp::OpCode> },
    MissingVersion,
    InvalidOpCode { value: u16 },
    InvalidTimestampType { value: u8 },
    InvalidVersion { value: u8 },
}

use self::ErrorReason::*;

#[derive(Debug, PartialEq)]
pub struct Error {
    pub reason: ErrorReason,
    pub start: usize,
    pub span: usize,
}

#[derive(Debug, PartialEq)]
pub struct Packet {
    pub packet: stp::Packet, // Packet type.
    pub start: usize,        // Packet's starting nibble offset.
    pub span: usize,         // Pacekt's size in nibbles.
}

pub type Result = result::Result<Packet, Error>;

struct DataFragment {
    data_sz_span: usize,
    data_sz: u8,
    data_span: usize,
    data: u64,
    has_timestamp: bool,
    timestamp_sz_span: usize,
    timestamp_sz: u8,
    timestamp_span: usize,
    timestamp: u64,
}

impl DataFragment {
    pub fn new(span: usize, data_sz: u8, has_timestamp: bool) -> Self {
        DataFragment {
            data_sz_span: 0,
            data_sz,
            data_span: span + data_sz as usize,
            data: 0,
            has_timestamp,
            timestamp_sz_span: 0,
            timestamp_sz: 0,
            timestamp_span: 0,
            timestamp: 0,
        }
    }
}

enum DecoderState {
    Unsynced, // The decoder is looking for a SYNC packet.
    OpCode,   // Processing an opcode.
    Version(u8),
    Data(DataFragment),
}

use self::DecoderState::*;

const ASYNC_F_COUNT: u8 = 21;

pub struct StpDecoder {
    state: DecoderState,                 // The state of the decoder.
    offset: usize,                       // Offset in nibbles.
    f_count: u8,                         // Number of consecutive 0xF nibbles.
    start: usize,                        // Current packet starting offset.
    span: usize,                         // Current packet span.
    opcode: Option<stp::OpCode>,         // Current opcode.
    ts_type: Option<stp::TimestampType>, // Timestamp type.
    is_le: bool,                         // Are data payloads little endian?
}

impl StpDecoder {
    /// Create a new StpDecoder.
    pub fn new() -> Self {
        return StpDecoder {
            state: Unsynced,
            offset: 0,
            f_count: 0,
            start: 0,
            span: 0,
            opcode: None,
            ts_type: None,
            is_le: false,
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

        // An ASYNC can appear anywhere within the stream, so every 0xf nibble needs to be buffered
        // until either a non-0xf nibble is encountered, or ASYNC_F_COUNT number of 0xf nibbles
        // are encountered.
        if nibble == 0xf {
            if self.f_count < ASYNC_F_COUNT {
                self.f_count += 1;
            } else {
                self.do_decode_nibble(0xf, &mut handler);
            }
        } else {
            if self.f_count == ASYNC_F_COUNT {
                if nibble == 0 {
                    self.handle_async(&mut handler);
                } else {
                    self.handle_invalid_async(nibble, &mut handler);
                }
            } else {
                for _ in 0..self.f_count {
                    self.do_decode_nibble(0xf, &mut handler);
                }
                self.do_decode_nibble(nibble, &mut handler);
            }
            self.f_count = 0;
        }
    }

    fn handle_async(&mut self, handler: &mut dyn FnMut(Result)) {
        let span = ASYNC_F_COUNT as usize + 1;

        // Report truncated packets (if any):
        self.truncated_packet_check(handler);

        // Report the async packet:
        handler(Ok(Packet {
            packet: stp::Packet::Async,
            start: self.offset,
            span,
        }));

        self.offset += span;

        // Per the spec, an ASYNC must be followed by a VERSION packet, we can use ts_type to tell
        // if this has been violated.
        self.ts_type = None;
        self.is_le = false;

        // Transition to the 'OpCode' state
        self.set_state(OpCode);
    }

    fn handle_invalid_async(&mut self, bad_nibble: u8, handler: &mut dyn FnMut(Result)) {
        let span = ASYNC_F_COUNT as usize + 1;

        // Report truncated packets (if any):
        self.truncated_packet_check(handler);

        // Report the error:
        handler(Err(Error {
            reason: InvalidAsync { bad_nibble },
            start: self.offset,
            span,
        }));

        self.offset += span;

        // Transition to the 'Unsynced' state
        self.set_state(Unsynced);
    }

    fn truncated_packet_check(&mut self, handler: &mut dyn FnMut(Result)) {
        if let Unsynced = self.state {
            return;
        } else if self.span == 0 {
            return;
        }

        self.report_error(
            TruncatedPacket {
                opcode: self.opcode,
            },
            handler,
        );
    }

    fn report_error(&mut self, reason: ErrorReason, handler: &mut dyn FnMut(Result)) {
        handler(Err(Error {
            reason,
            start: self.start,
            span: self.span,
        }));
    }

    fn report_packet(&mut self, packet: stp::Packet, handler: &mut dyn FnMut(Result)) {
        handler(Ok(Packet {
            packet,
            start: self.start,
            span: self.span,
        }));
    }

    fn set_state(&mut self, new_state: DecoderState) {
        if let Unsynced | OpCode = new_state {
            self.span = 0;
            self.opcode = None;
        }
        self.state = new_state;
    }

    fn do_decode_nibble(&mut self, nibble: u8, handler: &mut dyn FnMut(Result)) {
        self.span += 1;
        match self.state {
            Unsynced => {} // Do nothing.
            OpCode => self.decode_opcode(nibble, handler),
            Version(_) => self.decode_version(nibble, handler),
            Data(_) => self.decode_data(nibble, handler),
        }
        self.offset += 1;
    }

    fn decode_opcode(&mut self, nibble: u8, handler: &mut dyn FnMut(Result)) {
        match self.span {
            1 => {
                self.start = self.offset;
                match nibble {
                    0x0 => self.span = 0, // NULL packet.
                    0x1 => self.set_data_state(M8, 2, false, handler),
                    0x2 => self.set_data_state(MERR, 2, false, handler),
                    0x3 => self.set_data_state(C8, 2, false, handler),
                    0x4 => self.set_data_state(D8, 2, false, handler),
                    0x5 => self.set_data_state(D16, 4, false, handler),
                    0x6 => self.set_data_state(D32, 8, false, handler),
                    0x7 => self.set_data_state(D64, 16, false, handler),
                    0x8 => self.set_data_state(D8MTS, 2, true, handler),
                    0x9 => self.set_data_state(D16MTS, 4, true, handler),
                    0xA => self.set_data_state(D32MTS, 8, true, handler),
                    0xB => self.set_data_state(D64MTS, 8, true, handler),
                    0xC => self.set_data_state(D4, 1, false, handler),
                    0xD => self.set_data_state(D4MTS, 1, true, handler),
                    0xE => self.set_data_state(FLAG_TS, 0, true, handler),
                    0xF => {}
                    _ => panic!("Not a nibble: {}", nibble),
                }
            }
            2 => match nibble {
                0x0 => {}
                0x1 => self.set_data_state(M16, 4, false, handler),
                0x2 => self.set_data_state(GERR, 2, false, handler),
                0x3 => self.set_data_state(C16, 4, false, handler),
                0x4 => self.set_data_state(D8TS, 2, true, handler),
                0x5 => self.set_data_state(D16TS, 4, true, handler),
                0x6 => self.set_data_state(D32TS, 8, true, handler),
                0x7 => self.set_data_state(D64TS, 16, true, handler),
                0x8 => self.set_data_state(D8M, 2, false, handler),
                0x9 => self.set_data_state(D16M, 4, false, handler),
                0xA => self.set_data_state(D32M, 8, false, handler),
                0xB => self.set_data_state(D64M, 8, false, handler),
                0xC => self.set_data_state(D4TS, 1, true, handler),
                0xD => self.set_data_state(D4M, 1, false, handler),
                0xE => self.handle_flag_packet(handler),
                0xF => self.handle_invalid_opcode(0xFF, handler),
                _ => panic!("Not a nibble: {}", nibble),
            },
            3 => match nibble {
                0x0 => self.set_state(Version(0)),
                // TODO: Support remaining opcodes!
                _ => self.handle_invalid_opcode(0xF00 & nibble as u16, handler),
            },
            _ => panic!("Unexpected span: {}", self.span),
        }
    }

    fn set_data_state(
        &mut self,
        opcode: stp::OpCode,
        data_sz: u8,
        has_timestamp: bool,
        handler: &mut dyn FnMut(Result),
    ) {
        if self.valid_ts_type(handler) {
            self.opcode = Some(opcode);
            self.set_state(Data(DataFragment::new(self.span, data_sz, has_timestamp)));
        }
    }

    fn valid_ts_type(&mut self, handler: &mut dyn FnMut(Result)) -> bool {
        if let None = self.ts_type {
            self.report_error(MissingVersion, handler);
            self.set_state(Unsynced);
            false
        } else {
            true
        }
    }

    fn handle_invalid_opcode(&mut self, bad_opcode: u16, handler: &mut dyn FnMut(Result)) {
        self.report_error(InvalidOpCode { value: bad_opcode }, handler);
        self.set_state(Unsynced);
    }

    fn handle_flag_packet(&mut self, handler: &mut dyn FnMut(Result)) {
        self.report_packet(stp::Packet::Flag, handler);
        self.set_state(OpCode);
    }

    fn decode_version(&mut self, nibble: u8, handler: &mut dyn FnMut(Result)) {
        match self.span {
            4 => {
                let ts_type = match nibble & 0x7 {
                    0 | 1 => STPv1LEGACY,
                    2 => STPv2NATDELTA,
                    3 => STPv2NAT,
                    4 => STPv2GRAY,
                    value => {
                        self.report_error(InvalidTimestampType { value }, handler);
                        self.set_state(Unsynced);
                        return;
                    }
                };
                self.ts_type = Some(ts_type);
                if nibble & 0x8 == 0 {
                    self.report_packet(
                        stp::Packet::Version {
                            version: if nibble == 0 { STPv1 } else { STPv2_1 },
                            ts_type,
                            is_le: false,
                        },
                        handler,
                    );
                    self.set_state(OpCode);
                }
            }
            5 => self.state = Version(nibble),
            6 => {
                if let Version(prior_nibble) = self.state {
                    let payload = prior_nibble << 4 | nibble;
                    self.is_le = if payload & 0x80 == 0x80 { true } else { false };
                    if payload & 0x7F == 0x01 {
                        self.report_packet(
                            stp::Packet::Version {
                                version: STPv2_2,
                                ts_type: self.ts_type.unwrap(),
                                is_le: self.is_le,
                            },
                            handler,
                        );
                        self.set_state(OpCode);
                    } else {
                        self.report_error(
                            InvalidVersion {
                                value: payload & 0x7F,
                            },
                            handler,
                        );
                        self.set_state(Unsynced);
                    }
                }
            }
            _ => panic!("Unexpected VERSION span: {}", self.span),
        }
    }

    fn finish_timestamp(&mut self, ts: u64, length: u8) -> stp::Timestamp {
        let value = if length > 1 {
            swap_nibbles(ts, length as usize)
        } else {
            ts
        };
        match self.ts_type {
            Some(STPv1LEGACY) => stp::Timestamp::STPv1 { value: value as u8 },
            Some(STPv2NATDELTA) => stp::Timestamp::STPv2NATDELTA { length, value },
            Some(STPv2NAT) => stp::Timestamp::STPv2NAT { length, value },
            Some(STPv2GRAY) => stp::Timestamp::STPv2GRAY { length, value },
            None => panic!("Expected timestamp type"),
        }
    }

    fn finish_data(&mut self, handler: &mut dyn FnMut(Result)) {
        if let Data(ref tmp) = self.state {
            let data = if tmp.data_sz > 1 {
                swap_nibbles(tmp.data, tmp.data_sz as usize)
            } else {
                tmp.data
            };
            let packet = match self.opcode {
                Some(M8) | Some(M16) => stp::Packet::Master {
                    opcode: self.opcode.unwrap(),
                    master: data as u16,
                },
                Some(MERR) | Some(GERR) => stp::Packet::Error {
                    opcode: self.opcode.unwrap(),
                    data: data as u8,
                },
                Some(C8) | Some(C16) => stp::Packet::Channel {
                    opcode: self.opcode.unwrap(),
                    channel: data as u16,
                },
                Some(D4) | Some(D4M) | Some(D8) | Some(D8M) | Some(D16) | Some(D16M)
                | Some(D32) | Some(D32M) | Some(D64) | Some(D64M) => stp::Packet::Data {
                    opcode: self.opcode.unwrap(),
                    data,
                    timestamp: None,
                },
                Some(D4TS) | Some(D4MTS) | Some(D8TS) | Some(D8MTS) | Some(D16TS)
                | Some(D16MTS) | Some(D32TS) | Some(D32MTS) | Some(D64TS) | Some(D64MTS) => {
                    let ts = tmp.timestamp;
                    let ts_sz = tmp.timestamp_sz;
                    stp::Packet::Data {
                        opcode: self.opcode.unwrap(),
                        data,
                        timestamp: Some(self.finish_timestamp(ts, ts_sz)),
                    }
                }
                Some(v) => panic!("Unexpected data opcode: {:?}", v),
                None => panic!("Unexpected data opcode: None"),
            };
            self.report_packet(packet, handler);
            self.set_state(OpCode);
        }
    }

    fn decode_data(&mut self, nibble: u8, handler: &mut dyn FnMut(Result)) {
        if let Data(ref mut tmp) = self.state {
            match self.span {
                s if s < tmp.data_span => tmp.data = tmp.data << 4 | nibble as u64,
                s if s < tmp.timestamp_span => tmp.timestamp = tmp.timestamp << 4 | nibble as u64,
                s if s == tmp.data_span => {
                    if tmp.has_timestamp {
                        if self.ts_type == Some(STPv1LEGACY) {
                            tmp.timestamp_sz = 2;
                            tmp.timestamp_span = self.span + 2;
                        } else {
                            tmp.timestamp_sz_span = self.span + 1;
                        }
                    } else {
                        self.finish_data(handler);
                    }
                }
                s if s == tmp.timestamp_span => self.finish_data(handler),
                s if s == tmp.timestamp_sz_span => {
                    tmp.timestamp_sz = nibble;
                    tmp.timestamp_span = self.span + nibble as usize;
                }
                s => panic!("Unexpected decode data nibble: {}", s),
            }
        }
    }
}
