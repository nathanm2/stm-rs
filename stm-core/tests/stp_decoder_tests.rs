use stm_core::stp::{self, StpVersion, Timestamp, TimestampType};
use stm_core::stp_decoder::{Error, ErrorReason::*, Packet, Result, StpDecoder};

// Unsynced:
#[test]
fn basic_unsynced() {
    let mut results = Vec::<Result>::new();
    let mut decoder = StpDecoder::new();
    let stream = [1; 32];

    decoder.decode_bytes(&stream, |r| results.push(r));
    assert_eq!(results.len(), 0);
}

#[test]
fn basic_async() {
    let mut results = Vec::<Result>::new();
    let mut exp = Vec::<Result>::new();
    let mut decoder = StpDecoder::new();
    let mut stream = [0xFF; 11];
    stream[10] = 0x0F;

    decoder.decode_bytes(&stream, |r| results.push(r));

    exp.push(Ok(Packet {
        packet: stp::Packet::Async,
        start: 0,
        span: 22,
    }));

    assert_eq!(results, exp);
}

#[test]
fn invalid_async() {
    let mut results = Vec::<Result>::new();
    let mut exp = Vec::<Result>::new();
    let mut decoder = StpDecoder::new();
    let mut stream = [0xFF; 11];
    stream[10] = 0x1F;

    decoder.decode_bytes(&stream, |r| results.push(r));

    exp.push(Err(Error {
        reason: InvalidAsync { bad_nibble: 1 },
        start: 0,
        span: 22,
    }));

    assert_eq!(results, exp);
}

const ASYNC_NIBBLES: [u8; 22] = [
    0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf,
    0xf, 0xf, 0x0,
];

const INVALID_ASYNC_NIBBLES: [u8; 22] = [
    0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf,
    0xf, 0xf, 0x1,
];

#[test]
fn truncated_by_async() {
    let mut results = Vec::<Result>::new();
    let mut exp = Vec::<Result>::new();
    let mut decoder = StpDecoder::new();
    let mut stream = Vec::<u8>::with_capacity(46);
    stream.push(0x1);
    stream.extend_from_slice(&ASYNC_NIBBLES);
    stream.push(0x0);
    stream.push(0xF);
    stream.push(0x0);
    stream.extend_from_slice(&ASYNC_NIBBLES);

    decoder.decode_nibbles(&stream, |r| results.push(r));

    exp.push(Ok(Packet {
        packet: stp::Packet::Async,
        start: 1,
        span: 22,
    }));

    exp.push(Err(Error {
        reason: TruncatedPacket { opcode: None },
        start: 24,
        span: 2,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Async,
        start: 26,
        span: 22,
    }));
    assert_eq!(results, exp);
}

#[test]
fn truncated_by_invalid_async() {
    let mut results = Vec::<Result>::new();
    let mut exp = Vec::<Result>::new();
    let mut decoder = StpDecoder::new();
    let mut stream = Vec::<u8>::with_capacity(46);
    stream.extend_from_slice(&ASYNC_NIBBLES);
    stream.push(0xF);
    stream.extend_from_slice(&INVALID_ASYNC_NIBBLES);

    decoder.decode_nibbles(&stream, |r| results.push(r));

    exp.push(Ok(Packet {
        packet: stp::Packet::Async,
        start: 0,
        span: 22,
    }));

    exp.push(Err(Error {
        reason: TruncatedPacket { opcode: None },
        start: 22,
        span: 1,
    }));

    exp.push(Err(Error {
        reason: InvalidAsync { bad_nibble: 1 },
        start: 23,
        span: 22,
    }));

    assert_eq!(results, exp);
}

#[test]
fn invalid_opcode() {
    let mut results = Vec::<Result>::new();
    let mut exp = Vec::<Result>::new();
    let mut decoder = StpDecoder::new();
    let mut stream = Vec::<u8>::with_capacity(46);

    stream.extend_from_slice(&ASYNC_NIBBLES);
    stream.push(0xF);
    stream.push(0xF); // <= Invalid op-code
    stream.extend_from_slice(&ASYNC_NIBBLES);

    decoder.decode_nibbles(&stream, |r| results.push(r));

    exp.push(Ok(Packet {
        packet: stp::Packet::Async,
        start: 0,
        span: 22,
    }));

    exp.push(Err(Error {
        reason: InvalidOpCode { value: 0xFF },
        start: 22,
        span: 2,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Async,
        start: 24,
        span: 22,
    }));

    assert_eq!(results, exp);
}

// STPv2.2, NATDELTA, BE
const VERSION_NIBBLES: [u8; 6] = [0xf, 0x0, 0x0, 0xA, 0x0, 0x1];
const VERSION_V1_NIBBLES: [u8; 4] = [0xf, 0x0, 0x0, 0x0];
const VERSION_V2_1_NIBBLES: [u8; 4] = [0xf, 0x0, 0x0, 0x3];
const VERSION_LE_NIBBLES: [u8; 6] = [0xf, 0x0, 0x0, 0xC, 0x8, 0x1];
const VERSION_INVALID_NIBBLES: [u8; 6] = [0xf, 0x0, 0x0, 0xC, 0x8, 0x2];

#[test]
fn version_test() {
    let mut results = Vec::<Result>::new();
    let mut exp = Vec::<Result>::new();
    let mut decoder = StpDecoder::new();
    let mut stream = Vec::<u8>::with_capacity(46);

    stream.extend_from_slice(&ASYNC_NIBBLES);
    stream.extend_from_slice(&VERSION_NIBBLES);
    stream.extend_from_slice(&VERSION_V1_NIBBLES);
    stream.extend_from_slice(&VERSION_V2_1_NIBBLES);
    stream.extend_from_slice(&VERSION_LE_NIBBLES);
    stream.extend_from_slice(&VERSION_INVALID_NIBBLES);

    decoder.decode_nibbles(&stream, |r| results.push(r));

    exp.push(Ok(Packet {
        packet: stp::Packet::Async,
        start: 0,
        span: 22,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Version {
            version: StpVersion::STPv2_2,
            ts_type: TimestampType::STPv2NATDELTA,
            is_le: false,
        },
        start: 22,
        span: 6,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Version {
            version: StpVersion::STPv1,
            ts_type: TimestampType::STPv1LEGACY,
            is_le: false,
        },
        start: 28,
        span: 4,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Version {
            version: StpVersion::STPv2_1,
            ts_type: TimestampType::STPv2NAT,
            is_le: false,
        },
        start: 32,
        span: 4,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Version {
            version: StpVersion::STPv2_2,
            ts_type: TimestampType::STPv2GRAY,
            is_le: true,
        },
        start: 36,
        span: 6,
    }));

    exp.push(Err(Error {
        reason: InvalidVersion { value: 0x2 },
        start: 42,
        span: 6,
    }));

    assert_eq!(results, exp);
}

const D4_NIBBLES: [u8; 2] = [0xC, 0x1];
const D8_NIBBLES: [u8; 3] = [0x4, 0x1, 0x2];
const D16_NIBBLES: [u8; 5] = [0x5, 0x1, 0x2, 0x3, 0x4];
const D32_NIBBLES: [u8; 9] = [0x6, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8];
const D64_NIBBLES: [u8; 17] = [
    0x7, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8, 0x9, 0xa, 0xb, 0xc, 0xd, 0xe, 0xf, 0x0,
];

const D4M_NIBBLES: [u8; 3] = [0xF, 0xD, 0x1];
const D8M_NIBBLES: [u8; 4] = [0xF, 0x8, 0x1, 0x2];
const D16M_NIBBLES: [u8; 6] = [0xF, 0x9, 0x1, 0x2, 0x3, 0x4];
const D32M_NIBBLES: [u8; 10] = [0xF, 0xA, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8];
const D64M_NIBBLES: [u8; 18] = [
    0xF, 0xB, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8, 0x9, 0xa, 0xb, 0xc, 0xd, 0xe, 0xf, 0x0,
];

fn is_data(r: &Result) -> bool {
    match r {
        Ok(Packet {
            packet: stp::Packet::Data { .. },
            ..
        }) => true,
        _ => false,
    }
}

#[test]
fn data_be_test() {
    let mut results = Vec::<Result>::new();
    let mut exp = Vec::<Result>::new();
    let mut decoder = StpDecoder::new();
    let mut stream = Vec::<u8>::with_capacity(46);

    stream.extend_from_slice(&ASYNC_NIBBLES);
    stream.extend_from_slice(&VERSION_NIBBLES);
    stream.extend_from_slice(&D4_NIBBLES);
    stream.extend_from_slice(&D8_NIBBLES);
    stream.extend_from_slice(&D16_NIBBLES);
    stream.extend_from_slice(&D32_NIBBLES);
    stream.extend_from_slice(&D64_NIBBLES);
    stream.extend_from_slice(&D4M_NIBBLES);
    stream.extend_from_slice(&D8M_NIBBLES);
    stream.extend_from_slice(&D16M_NIBBLES);
    stream.extend_from_slice(&D32M_NIBBLES);
    stream.extend_from_slice(&D64M_NIBBLES);

    decoder.decode_nibbles(&stream, |r| {
        if is_data(&r) {
            results.push(r);
        }
    });

    exp.push(Ok(Packet {
        packet: stp::Packet::Data {
            opcode: stp::OpCode::D4,
            data: 0x1,
            timestamp: None,
        },
        start: 28,
        span: 2,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Data {
            opcode: stp::OpCode::D8,
            data: 0x12,
            timestamp: None,
        },
        start: 30,
        span: 3,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Data {
            opcode: stp::OpCode::D16,
            data: 0x1234,
            timestamp: None,
        },
        start: 33,
        span: 5,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Data {
            opcode: stp::OpCode::D32,
            data: 0x12345678,
            timestamp: None,
        },
        start: 38,
        span: 9,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Data {
            opcode: stp::OpCode::D64,
            data: 0x1234_5678_9abc_def0,
            timestamp: None,
        },
        start: 47,
        span: 17,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Data {
            opcode: stp::OpCode::D4M,
            data: 0x1,
            timestamp: None,
        },
        start: 64,
        span: 3,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Data {
            opcode: stp::OpCode::D8M,
            data: 0x12,
            timestamp: None,
        },
        start: 67,
        span: 4,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Data {
            opcode: stp::OpCode::D16M,
            data: 0x1234,
            timestamp: None,
        },
        start: 71,
        span: 6,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Data {
            opcode: stp::OpCode::D32M,
            data: 0x12345678,
            timestamp: None,
        },
        start: 77,
        span: 10,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Data {
            opcode: stp::OpCode::D64M,
            data: 0x1234_5678_9abc_def0,
            timestamp: None,
        },
        start: 87,
        span: 18,
    }));

    assert_eq!(results, exp);
}

const D4TS_NIBBLES: [u8; 5] = [0xF, 0xC, 0x1, 0x1, 0x1];
const D8TS_NIBBLES: [u8; 7] = [0xF, 0x4, 0x1, 0x2, 0x2, 0x1, 0x2];
const D16TS_NIBBLES: [u8; 10] = [0xF, 0x5, 0x1, 0x2, 0x3, 0x4, 0x3, 0x1, 0x2, 0x3];
const D32TS_NIBBLES: [u8; 15] = [
    0xF, 0x6, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8, 0x4, 0x1, 0x2, 0x3, 0x4,
];
const D64TS_NIBBLES: [u8; 24] = [
    0xF, 0x7, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8, 0x9, 0xa, 0xb, 0xc, 0xd, 0xe, 0xf, 0x0, 0x5,
    0x1, 0x2, 0x3, 0x4, 0x5,
];

const D4MTS_NIBBLES: [u8; 4] = [0xD, 0x1, 0x1, 0x1];
const D8MTS_NIBBLES: [u8; 6] = [0x8, 0x1, 0x2, 0x2, 0x1, 0x2];
const D16MTS_NIBBLES: [u8; 9] = [0x9, 0x1, 0x2, 0x3, 0x4, 0x3, 0x1, 0x2, 0x3];
const D32MTS_NIBBLES: [u8; 14] = [
    0xA, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8, 0x4, 0x1, 0x2, 0x3, 0x4,
];
const D64MTS_NIBBLES: [u8; 23] = [
    0xB, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8, 0x9, 0xa, 0xb, 0xc, 0xd, 0xe, 0xf, 0x0, 0x5, 0x1,
    0x2, 0x3, 0x4, 0x5,
];
#[test]
fn data_be_ts_test() {
    let mut results = Vec::<Result>::new();
    let mut exp = Vec::<Result>::new();
    let mut decoder = StpDecoder::new();
    let mut stream = Vec::<u8>::with_capacity(46);

    stream.extend_from_slice(&ASYNC_NIBBLES);
    stream.extend_from_slice(&VERSION_NIBBLES);
    stream.extend_from_slice(&D4TS_NIBBLES);
    stream.extend_from_slice(&D8TS_NIBBLES);
    stream.extend_from_slice(&D16TS_NIBBLES);
    stream.extend_from_slice(&D32TS_NIBBLES);
    stream.extend_from_slice(&D64TS_NIBBLES);
    stream.extend_from_slice(&D4MTS_NIBBLES);
    stream.extend_from_slice(&D8MTS_NIBBLES);
    stream.extend_from_slice(&D16MTS_NIBBLES);
    stream.extend_from_slice(&D32MTS_NIBBLES);
    stream.extend_from_slice(&D64MTS_NIBBLES);

    decoder.decode_nibbles(&stream, |r| {
        if is_data(&r) {
            results.push(r);
        }
    });

    exp.push(Ok(Packet {
        packet: stp::Packet::Data {
            opcode: stp::OpCode::D4TS,
            data: 0x1,
            timestamp: Some(Timestamp::STPv2NATDELTA {
                length: 1,
                value: 0x1,
            }),
        },
        start: 28,
        span: 5,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Data {
            opcode: stp::OpCode::D8TS,
            data: 0x12,
            timestamp: Some(Timestamp::STPv2NATDELTA {
                length: 2,
                value: 0x12,
            }),
        },
        start: 33,
        span: 7,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Data {
            opcode: stp::OpCode::D16TS,
            data: 0x1234,
            timestamp: Some(Timestamp::STPv2NATDELTA {
                length: 3,
                value: 0x123,
            }),
        },
        start: 40,
        span: 10,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Data {
            opcode: stp::OpCode::D32TS,
            data: 0x1234_5678,
            timestamp: Some(Timestamp::STPv2NATDELTA {
                length: 4,
                value: 0x1234,
            }),
        },
        start: 50,
        span: 15,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Data {
            opcode: stp::OpCode::D64TS,
            data: 0x1234_5678_9abc_def0,
            timestamp: Some(Timestamp::STPv2NATDELTA {
                length: 5,
                value: 0x12345,
            }),
        },
        start: 65,
        span: 24,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Data {
            opcode: stp::OpCode::D4MTS,
            data: 0x1,
            timestamp: Some(Timestamp::STPv2NATDELTA {
                length: 1,
                value: 0x1,
            }),
        },
        start: 89,
        span: 4,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Data {
            opcode: stp::OpCode::D8MTS,
            data: 0x12,
            timestamp: Some(Timestamp::STPv2NATDELTA {
                length: 2,
                value: 0x12,
            }),
        },
        start: 93,
        span: 6,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Data {
            opcode: stp::OpCode::D16MTS,
            data: 0x1234,
            timestamp: Some(Timestamp::STPv2NATDELTA {
                length: 3,
                value: 0x123,
            }),
        },
        start: 99,
        span: 9,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Data {
            opcode: stp::OpCode::D32MTS,
            data: 0x1234_5678,
            timestamp: Some(Timestamp::STPv2NATDELTA {
                length: 4,
                value: 0x1234,
            }),
        },
        start: 108,
        span: 14,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Data {
            opcode: stp::OpCode::D64MTS,
            data: 0x1234_5678_9abc_def0,
            timestamp: Some(Timestamp::STPv2NATDELTA {
                length: 5,
                value: 0x12345,
            }),
        },
        start: 122,
        span: 23,
    }));

    assert_eq!(results, exp);
}

#[test]
fn data_le_test() {
    let mut results = Vec::<Result>::new();
    let mut exp = Vec::<Result>::new();
    let mut decoder = StpDecoder::new();
    let mut stream = Vec::<u8>::with_capacity(46);

    stream.extend_from_slice(&ASYNC_NIBBLES);
    stream.extend_from_slice(&VERSION_LE_NIBBLES);
    stream.extend_from_slice(&D4_NIBBLES);
    stream.extend_from_slice(&D64_NIBBLES);
    stream.extend_from_slice(&D8TS_NIBBLES);

    decoder.decode_nibbles(&stream, |r| {
        if is_data(&r) {
            results.push(r);
        }
    });

    exp.push(Ok(Packet {
        packet: stp::Packet::Data {
            opcode: stp::OpCode::D4,
            data: 0x1,
            timestamp: None,
        },
        start: 28,
        span: 2,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Data {
            opcode: stp::OpCode::D64,
            data: 0x0fed_cba9_8765_4321,
            timestamp: None,
        },
        start: 30,
        span: 17,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Data {
            opcode: stp::OpCode::D8TS,
            data: 0x21,
            timestamp: Some(Timestamp::STPv2GRAY {
                length: 2,
                value: 0x21,
            }),
        },
        start: 47,
        span: 7,
    }));
    assert_eq!(results, exp);
}

const M8_NIBBLES: [u8; 3] = [0x1, 0x1, 0x2];
const M16_NIBBLES: [u8; 6] = [0xf, 0x1, 0x1, 0x2, 0x3, 0x4];

fn is_master(r: &Result) -> bool {
    match r {
        Ok(Packet {
            packet: stp::Packet::Master { .. },
            ..
        }) => true,
        _ => false,
    }
}
#[test]
fn master_test() {
    let mut results = Vec::<Result>::new();
    let mut exp = Vec::<Result>::new();
    let mut decoder = StpDecoder::new();
    let mut stream = Vec::<u8>::with_capacity(46);

    stream.extend_from_slice(&ASYNC_NIBBLES);
    stream.extend_from_slice(&VERSION_NIBBLES);
    stream.extend_from_slice(&M8_NIBBLES);
    stream.extend_from_slice(&M16_NIBBLES);
    stream.extend_from_slice(&VERSION_LE_NIBBLES);
    stream.extend_from_slice(&M8_NIBBLES);
    stream.extend_from_slice(&M16_NIBBLES);

    decoder.decode_nibbles(&stream, |r| {
        if is_master(&r) {
            results.push(r);
        }
    });

    exp.push(Ok(Packet {
        packet: stp::Packet::Master {
            opcode: stp::OpCode::M8,
            master: 0x12,
        },
        start: 28,
        span: 3,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Master {
            opcode: stp::OpCode::M16,
            master: 0x1234,
        },
        start: 31,
        span: 6,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Master {
            opcode: stp::OpCode::M8,
            master: 0x21,
        },
        start: 43,
        span: 3,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Master {
            opcode: stp::OpCode::M16,
            master: 0x4321,
        },
        start: 46,
        span: 6,
    }));

    assert_eq!(results, exp);
}

const C8_NIBBLES: [u8; 3] = [0x3, 0x1, 0x2];
const C16_NIBBLES: [u8; 6] = [0xf, 0x3, 0x1, 0x2, 0x3, 0x4];

fn is_channel(r: &Result) -> bool {
    match r {
        Ok(Packet {
            packet: stp::Packet::Channel { .. },
            ..
        }) => true,
        _ => false,
    }
}
#[test]
fn channel_test() {
    let mut results = Vec::<Result>::new();
    let mut exp = Vec::<Result>::new();
    let mut decoder = StpDecoder::new();
    let mut stream = Vec::<u8>::with_capacity(46);

    stream.extend_from_slice(&ASYNC_NIBBLES);
    stream.extend_from_slice(&VERSION_NIBBLES);
    stream.extend_from_slice(&C8_NIBBLES);
    stream.extend_from_slice(&C16_NIBBLES);
    stream.extend_from_slice(&VERSION_LE_NIBBLES);
    stream.extend_from_slice(&C8_NIBBLES);
    stream.extend_from_slice(&C16_NIBBLES);

    decoder.decode_nibbles(&stream, |r| {
        if is_channel(&r) {
            results.push(r);
        }
    });

    exp.push(Ok(Packet {
        packet: stp::Packet::Channel {
            opcode: stp::OpCode::C8,
            channel: 0x12,
        },
        start: 28,
        span: 3,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Channel {
            opcode: stp::OpCode::C16,
            channel: 0x1234,
        },
        start: 31,
        span: 6,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Channel {
            opcode: stp::OpCode::C8,
            channel: 0x21,
        },
        start: 43,
        span: 3,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Channel {
            opcode: stp::OpCode::C16,
            channel: 0x4321,
        },
        start: 46,
        span: 6,
    }));

    assert_eq!(results, exp);
}

const MER_NIBBLES: [u8; 3] = [0x2, 0x1, 0x2];
const GER_NIBBLES: [u8; 4] = [0xf, 0x2, 0x1, 0x2];

fn is_error(r: &Result) -> bool {
    match r {
        Ok(Packet {
            packet: stp::Packet::Error { .. },
            ..
        }) => true,
        _ => false,
    }
}
#[test]
fn error_test() {
    let mut results = Vec::<Result>::new();
    let mut exp = Vec::<Result>::new();
    let mut decoder = StpDecoder::new();
    let mut stream = Vec::<u8>::with_capacity(46);

    stream.extend_from_slice(&ASYNC_NIBBLES);
    stream.extend_from_slice(&VERSION_NIBBLES);
    stream.extend_from_slice(&MER_NIBBLES);
    stream.extend_from_slice(&GER_NIBBLES);
    stream.extend_from_slice(&VERSION_LE_NIBBLES);
    stream.extend_from_slice(&MER_NIBBLES);
    stream.extend_from_slice(&GER_NIBBLES);

    decoder.decode_nibbles(&stream, |r| {
        if is_error(&r) {
            results.push(r);
        }
    });

    exp.push(Ok(Packet {
        packet: stp::Packet::Error {
            opcode: stp::OpCode::MERR,
            data: 0x12,
        },
        start: 28,
        span: 3,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Error {
            opcode: stp::OpCode::GERR,
            data: 0x12,
        },
        start: 31,
        span: 4,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Error {
            opcode: stp::OpCode::MERR,
            data: 0x21,
        },
        start: 41,
        span: 3,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Error {
            opcode: stp::OpCode::GERR,
            data: 0x21,
        },
        start: 44,
        span: 4,
    }));

    assert_eq!(results, exp);
}

const FLAG_NIBBLES: [u8; 2] = [0xF, 0xE];
const FLAG_TS_NIBBLES: [u8; 4] = [0xE, 0x2, 0x1, 0x2];

fn is_flag(r: &Result) -> bool {
    match r {
        Ok(Packet {
            packet: stp::Packet::Flag { .. },
            ..
        }) => true,
        _ => false,
    }
}
#[test]
fn flag_test() {
    let mut results = Vec::<Result>::new();
    let mut exp = Vec::<Result>::new();
    let mut decoder = StpDecoder::new();
    let mut stream = Vec::<u8>::with_capacity(46);

    stream.extend_from_slice(&ASYNC_NIBBLES);
    stream.extend_from_slice(&VERSION_NIBBLES);
    stream.extend_from_slice(&FLAG_NIBBLES);
    stream.extend_from_slice(&FLAG_TS_NIBBLES);

    decoder.decode_nibbles(&stream, |r| {
        if is_flag(&r) {
            results.push(r);
        }
    });

    exp.push(Ok(Packet {
        packet: stp::Packet::Flag { timestamp: None },
        start: 28,
        span: 2,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Flag {
            timestamp: Some(Timestamp::STPv2NATDELTA {
                length: 2,
                value: 0x12,
            }),
        },
        start: 30,
        span: 4,
    }));

    assert_eq!(results, exp);
}
