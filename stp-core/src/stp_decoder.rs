use crate::nibble::swap_nibbles;
use crate::stp::{self, OpCode::*, StpVersion::*, TimestampType::*};
use std::result;

#[derive(Debug, PartialEq)]
pub struct Packet {
    pub packet: stp::Packet, // Packet type.
    pub start: usize,        // Packet's starting nibble offset.
    pub span: usize,         // Pacekt's size in nibbles.
}

#[derive(Debug, PartialEq)]
pub enum ErrorReason {
    InvalidAsync { bad_nibble: u8 },
    TruncatedPacket { opcode: Option<stp::OpCode> },
    MissingVersion,
    InvalidOpCode { value: u16 },
    InvalidTimestampType { value: u8 },
    InvalidTimestampSize,
    InvalidVersion { value: u8 },
}

use self::ErrorReason::*;

#[derive(Debug, PartialEq)]
pub struct Error {
    pub reason: ErrorReason,
    pub start: usize,
    pub span: usize,
}

pub type Result = result::Result<Packet, Error>;

// Used internally.
type PartialResult = result::Result<stp::Packet, ErrorReason>;

enum DecoderState {
    Unsynced,          // The decoder is looking for a SYNC packet.
    OpCode,            // Decoding an opcode.
    Version(u8),       // Decoding a Version packet.
    Data(DataDecoder), // Decoding a data packet
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

    /// Decode a slice of bytes.
    pub fn decode_bytes<F>(&mut self, bytes: &[u8], mut handler: F)
    where
        F: FnMut(Result),
    {
        for byte in bytes {
            self.decode_nibble(byte & 0xF, &mut handler);
            self.decode_nibble(byte >> 4, &mut handler);
        }
    }

    /// Decode a slice of nibbles.
    pub fn decode_nibbles<F>(&mut self, nibbles: &[u8], mut handler: F)
    where
        F: FnMut(Result),
    {
        for nibble in nibbles {
            self.decode_nibble(*nibble, &mut handler);
        }
    }

    /// Decode a single nibble.
    pub fn decode_nibble<F>(&mut self, nibble: u8, mut handler: F)
    where
        F: FnMut(Result),
    {
        assert!(nibble & 0xF0 == 0x00);

        // An ASYNC can appear anywhere within the stream, so every 0xf nibble needs to be buffered
        // until either a non-0xf nibble is encountered, or ASYNC_F_COUNT number of 0xf nibbles
        // are encountered.  If the latter occurs, then conceptually a 0xf nibble rolls off the end
        // of the buffer.
        if nibble == 0xf {
            if self.f_count < ASYNC_F_COUNT {
                self.f_count += 1;
            } else {
                self.do_decode_nibble(0xf, &mut handler);
            }
        } else {
            if self.f_count == ASYNC_F_COUNT {
                self.handle_async(nibble, &mut handler);
            } else {
                // Decode any buffered 0xf nibbles first, and then decode the new nibble:
                for _ in 0..self.f_count {
                    self.do_decode_nibble(0xf, &mut handler);
                }
                self.do_decode_nibble(nibble, &mut handler);
            }
            self.f_count = 0;
        }
    }

    fn do_decode_nibble(&mut self, nibble: u8, handler: &mut dyn FnMut(Result)) {
        self.span += 1;

        let result = match self.state {
            Unsynced => None, // Do nothing.
            OpCode => self.decode_opcode(nibble),
            Version(_) => self.decode_version(nibble),
            Data(ref mut data_decoder) => data_decoder.decode(nibble, self.span),
        };

        // Convert a PartialResult into a Result and report it to the caller:
        match result {
            None => {}
            Some(Ok(packet)) => {
                self.report_packet(packet, handler);
                self.set_state(OpCode);
            }
            Some(Err(reason)) => {
                self.report_error(reason, handler);
                self.set_state(Unsynced);
            }
        };
        self.offset += 1;
    }

    fn decode_opcode(&mut self, nibble: u8) -> Option<PartialResult> {
        match self.span {
            1 => {
                self.start = self.offset;
                match nibble {
                    0x0 => Some(Ok(stp::Packet::Null { timestamp: None })),
                    0x1 => self.set_data_state(M8, 2, false),
                    0x2 => self.set_data_state(MERR, 2, false),
                    0x3 => self.set_data_state(C8, 2, false),
                    0x4 => self.set_data_state(D8, 2, false),
                    0x5 => self.set_data_state(D16, 4, false),
                    0x6 => self.set_data_state(D32, 8, false),
                    0x7 => self.set_data_state(D64, 16, false),
                    0x8 => self.set_data_state(D8MTS, 2, true),
                    0x9 => self.set_data_state(D16MTS, 4, true),
                    0xA => self.set_data_state(D32MTS, 8, true),
                    0xB => self.set_data_state(D64MTS, 16, true),
                    0xC => self.set_data_state(D4, 1, false),
                    0xD => self.set_data_state(D4MTS, 1, true),
                    0xE => self.set_data_state(FLAG_TS, 0, true),
                    0xF => None, // Continued in next nibble...
                    _ => panic!("Not a nibble: {}", nibble),
                }
            }
            2 => match nibble {
                0x0 => None, // Continued in next nibble...
                0x1 => self.set_data_state(M16, 4, false),
                0x2 => self.set_data_state(GERR, 2, false),
                0x3 => self.set_data_state(C16, 4, false),
                0x4 => self.set_data_state(D8TS, 2, true),
                0x5 => self.set_data_state(D16TS, 4, true),
                0x6 => self.set_data_state(D32TS, 8, true),
                0x7 => self.set_data_state(D64TS, 16, true),
                0x8 => self.set_data_state(D8M, 2, false),
                0x9 => self.set_data_state(D16M, 4, false),
                0xA => self.set_data_state(D32M, 8, false),
                0xB => self.set_data_state(D64M, 16, false),
                0xC => self.set_data_state(D4TS, 1, true),
                0xD => self.set_data_state(D4M, 1, false),
                0xE => Some(Ok(stp::Packet::Flag { timestamp: None })),
                0xF => Some(Err(InvalidOpCode {
                    value: 0xF0 | (nibble as u16),
                })),
                _ => panic!("Not a nibble: {}", nibble),
            },
            3 => match nibble {
                0x0 => self.set_version_state(),
                0x1 => self.set_data_state(NULL_TS, 0, true),
                0x2 => self.set_variable_data_state(USER, false),
                0x3 => self.set_variable_data_state(USER_TS, true),
                0x8 => self.set_data_state(FREQ, 8, false),
                0x9 => self.set_data_state(FREQ_TS, 8, true),
                0xF => None,
                // TODO: Support remaining opcodes!
                _ => Some(Err(InvalidOpCode {
                    value: 0xF00 | (nibble as u16),
                })),
            },
            4 => match nibble {
                0x0 => self.set_data_state(FREQ_40, 10, false),
                0x1 => self.set_data_state(FREQ_40_TS, 10, true),
                _ => Some(Err(InvalidOpCode {
                    value: 0xF0F0 | (nibble as u16),
                })),
            },
            _ => panic!("Unexpected span: {}", self.span),
        }
    }

    fn set_data_state(
        &mut self,
        opcode: stp::OpCode,
        data_sz: usize,
        has_timestamp: bool,
    ) -> Option<PartialResult> {
        if let None = self.ts_type {
            Some(Err(MissingVersion))
        } else {
            self.set_state(Data(DataDecoder::new(
                opcode,
                self.is_le,
                data_sz,
                self.span,
                if has_timestamp { self.ts_type } else { None },
            )));
            None
        }
    }

    fn set_variable_data_state(
        &mut self,
        opcode: stp::OpCode,
        has_timestamp: bool,
    ) -> Option<PartialResult> {
        if let None = self.ts_type {
            Some(Err(MissingVersion))
        } else {
            self.set_state(Data(DataDecoder::new_variable_data(
                opcode,
                self.is_le,
                self.span,
                if has_timestamp { self.ts_type } else { None },
            )));
            None
        }
    }

    fn set_version_state(&mut self) -> Option<PartialResult> {
        self.set_state(Version(0));
        None
    }

    fn decode_version(&mut self, nibble: u8) -> Option<PartialResult> {
        match self.span {
            4 => {
                let ts_type = match nibble & 0x7 {
                    0 | 1 => STPv1LEGACY,
                    2 => STPv2NATDELTA,
                    3 => STPv2NAT,
                    4 => STPv2GRAY,
                    value => {
                        return Some(Err(InvalidTimestampType { value }));
                    }
                };
                self.ts_type = Some(ts_type);
                if nibble & 0x8 == 0 {
                    Some(Ok(stp::Packet::Version {
                        version: if nibble == 0 { STPv1 } else { STPv2_1 },
                        ts_type,
                        is_le: false,
                    }))
                } else {
                    None
                }
            }
            5 => {
                self.state = Version(nibble);
                None
            }
            6 => {
                if let Version(prior_nibble) = self.state {
                    let payload = prior_nibble << 4 | nibble;
                    self.is_le = if payload & 0x80 == 0x80 { true } else { false };
                    if payload & 0x7F == 0x01 {
                        Some(Ok(stp::Packet::Version {
                            version: STPv2_2,
                            ts_type: self.ts_type.unwrap(),
                            is_le: self.is_le,
                        }))
                    } else {
                        Some(Err(InvalidVersion {
                            value: payload & 0x7F,
                        }))
                    }
                } else {
                    panic!("Unexpected state");
                }
            }
            _ => panic!("Unexpected VERSION span: {}", self.span),
        }
    }

    fn handle_async(&mut self, nibble: u8, handler: &mut dyn FnMut(Result)) {
        let span = ASYNC_F_COUNT as usize + 1;

        // Report truncated packets (if any):
        self.truncated_packet_check(handler);

        if nibble == 0x0 {
            // Report the async packet:
            handler(Ok(Packet {
                packet: stp::Packet::Async,
                start: self.offset,
                span,
            }));

            // Per the spec, an ASYNC must be followed by a VERSION packet, we can use ts_type to
            // tell if this has been violated.
            self.ts_type = None;
            self.is_le = false;

            // Transition to the 'OpCode' state
            self.set_state(OpCode);
        } else {
            // Report the invalid ASYNC packet:
            handler(Err(Error {
                reason: InvalidAsync { bad_nibble: nibble },
                start: self.offset,
                span,
            }));

            // Transition to the 'Unsynced' state
            self.set_state(Unsynced);
        }

        self.offset += span;
    }

    fn truncated_packet_check(&mut self, handler: &mut dyn FnMut(Result)) {
        // If we're not synced OR we're between packets, then nothing has been truncated:
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
}

type TimestampResult = result::Result<stp::Timestamp, ErrorReason>;

struct TimestampDecoder {
    ts: u64,
    ts_span: usize,
    ts_sz: u8,
    ts_type: stp::TimestampType,
    is_le: bool,
}

impl TimestampDecoder {
    fn new(ts_type: stp::TimestampType, is_le: bool) -> TimestampDecoder {
        TimestampDecoder {
            ts: 0,
            ts_span: 0,
            ts_sz: 0,
            ts_type,
            is_le,
        }
    }

    fn decode(&mut self, nibble: u8, span: usize) -> Option<TimestampResult> {
        if span <= self.ts_span {
            self.ts = self.ts << 4 | nibble as u64;
            if span == self.ts_span {
                return Some(Ok(self.finish_timestamp()));
            }
        } else if self.ts_span == 0 {
            if self.ts_type == STPv1LEGACY {
                self.ts_sz = 2;
                self.ts_span = span + 1;
                self.ts = self.ts << 4 | nibble as u64;
            } else {
                // To insure this branch is only called once and panics thereafter...
                self.ts_span = span;

                self.ts_sz = match nibble {
                    0 => return Some(Ok(self.finish_timestamp())),
                    v @ 0x1..=0xC => v,
                    0xD => 14,
                    0xE => 16,
                    _ => return Some(Err(InvalidTimestampSize)),
                };
                self.ts_span = span + self.ts_sz as usize;
            }
        } else {
            panic!("Unexpected timestamp nibble");
        }
        None
    }

    fn finish_timestamp(&self) -> stp::Timestamp {
        let value = if self.ts_sz > 1 && self.is_le {
            swap_nibbles(self.ts, self.ts_sz as usize)
        } else {
            self.ts
        };
        match self.ts_type {
            STPv1LEGACY => stp::Timestamp::STPv1 { value: value as u8 },
            STPv2NATDELTA => stp::Timestamp::STPv2NATDELTA {
                length: self.ts_sz,
                value,
            },
            STPv2NAT => stp::Timestamp::STPv2NAT {
                length: self.ts_sz,
                value,
            },
            STPv2GRAY => stp::Timestamp::STPv2GRAY {
                length: self.ts_sz,
                value,
            },
        }
    }
}

struct DataDecoder {
    data: u64,
    data_sz: usize,
    data_span: usize,
    opcode: stp::OpCode,
    is_le: bool,
    ts_decoder: Option<TimestampDecoder>,
}

impl DataDecoder {
    fn new(
        opcode: stp::OpCode,
        is_le: bool,
        data_sz: usize,
        span: usize,
        ts_type: Option<stp::TimestampType>,
    ) -> DataDecoder {
        let ts_decoder = match ts_type {
            Some(tt) => Some(TimestampDecoder::new(tt, is_le)),
            None => None,
        };
        DataDecoder {
            data: 0,
            data_sz,
            data_span: span + data_sz,
            opcode,
            is_le,
            ts_decoder,
        }
    }

    fn new_variable_data(
        opcode: stp::OpCode,
        is_le: bool,
        _span: usize,
        ts_type: Option<stp::TimestampType>,
    ) -> DataDecoder {
        DataDecoder::new(opcode, is_le, 0, 0, ts_type)
    }

    fn decode(&mut self, nibble: u8, span: usize) -> Option<PartialResult> {
        if span <= self.data_span {
            self.data = self.data << 4 | nibble as u64;
            if span == self.data_span && self.ts_decoder.is_none() {
                Some(Ok(self.finish(None)))
            } else {
                None
            }
        } else if self.data_span == 0 {
            self.data_sz = (nibble as usize) + 1;
            self.data_span = span + self.data_sz;
            None
        } else {
            match self.ts_decoder.as_mut().unwrap().decode(nibble, span) {
                None => None,
                Some(Err(error)) => Some(Err(error)),
                Some(Ok(ts)) => Some(Ok(self.finish(Some(ts)))),
            }
        }
    }

    fn finish(&mut self, timestamp: Option<stp::Timestamp>) -> stp::Packet {
        let opcode = self.opcode;
        let data = if self.data_sz > 1 && self.is_le {
            swap_nibbles(self.data, self.data_sz)
        } else {
            self.data
        };

        match opcode {
            M8 | M16 => stp::Packet::Master {
                opcode,
                master: data as u16,
            },
            MERR | GERR => stp::Packet::Error {
                opcode,
                data: data as u8,
            },
            C8 | C16 => stp::Packet::Channel {
                opcode,
                channel: data as u16,
            },
            D4 | D4M | D4TS | D4MTS | D8 | D8M | D8TS | D8MTS | D16 | D16M | D16TS | D16MTS
            | D32 | D32M | D32TS | D32MTS | D64 | D64M | D64TS | D64MTS => stp::Packet::Data {
                opcode,
                data,
                timestamp,
            },
            FLAG_TS => stp::Packet::Flag { timestamp },
            FREQ | FREQ_TS | FREQ_40 | FREQ_40_TS => stp::Packet::Frequency {
                opcode,
                frequency: data,
                timestamp,
            },
            NULL_TS => stp::Packet::Null { timestamp },
            USER | USER_TS => stp::Packet::User {
                length: self.data_sz as u8,
                payload: data,
                timestamp,
            },

            _ => panic!("Unexpected data opcode: {:?}", opcode),
        }
    }
}
