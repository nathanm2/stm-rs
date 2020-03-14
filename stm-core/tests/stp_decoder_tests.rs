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
const D64_NIBBLES: [u8; 17] = [
    0x7, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8, 0x9, 0xa, 0xb, 0xc, 0xd, 0xe, 0xf, 0x0,
];
const D4TS_NIBBLES: [u8; 6] = [0xF, 0xC, 0x1, 0x2, 0x1, 0x2];

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
    stream.extend_from_slice(&D64_NIBBLES);
    stream.extend_from_slice(&D4TS_NIBBLES);

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
            data: 0x1234_5678_9abc_def0,
            timestamp: None,
        },
        start: 30,
        span: 17,
    }));

    exp.push(Ok(Packet {
        packet: stp::Packet::Data {
            opcode: stp::OpCode::D4TS,
            data: 0x1,
            timestamp: Some(Timestamp::STPv2NATDELTA {
                length: 2,
                value: 0x12,
            }),
        },
        start: 47,
        span: 6,
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
    stream.extend_from_slice(&D4TS_NIBBLES);

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
            opcode: stp::OpCode::D4TS,
            data: 0x1,
            timestamp: Some(Timestamp::STPv2GRAY {
                length: 2,
                value: 0x21,
            }),
        },
        start: 47,
        span: 6,
    }));
    assert_eq!(results, exp);
}
