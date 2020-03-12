use stm_core::stp::{self, StpVersion, TimestampType};
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
const VERSION_NIBBLES: [u8; 6] = [0xf, 0x0, 0x0, 0xA, 0x0, 0x01];

#[test]
fn version_test() {
    let mut results = Vec::<Result>::new();
    let mut exp = Vec::<Result>::new();
    let mut decoder = StpDecoder::new();
    let mut stream = Vec::<u8>::with_capacity(46);

    stream.extend_from_slice(&ASYNC_NIBBLES);
    stream.extend_from_slice(&VERSION_NIBBLES);

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

    assert_eq!(results, exp);
}
