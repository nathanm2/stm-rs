use stm_core::stp_decoder::{Error::*, Packet, PacketDetails, Result, StpDecoder};

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
        start: 0,
        span: 22,
        details: PacketDetails::Async,
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

    exp.push(Err(InvalidAsync {
        start: 0,
        value: 0x01,
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
    stream.extend_from_slice(&ASYNC_NIBBLES);
    stream.push(0xF);
    stream.extend_from_slice(&ASYNC_NIBBLES);

    decoder.decode_nibbles(&stream, |r| results.push(r));

    exp.push(Ok(Packet {
        start: 0,
        span: 22,
        details: PacketDetails::Async,
    }));

    exp.push(Err(TruncatedPacket { start: 22, span: 1 }));

    exp.push(Ok(Packet {
        start: 23,
        span: 22,
        details: PacketDetails::Async,
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
        start: 0,
        span: 22,
        details: PacketDetails::Async,
    }));

    exp.push(Err(TruncatedPacket { start: 22, span: 1 }));

    exp.push(Err(InvalidAsync {
        start: 23,
        value: 0x01,
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
        start: 0,
        span: 22,
        details: PacketDetails::Async,
    }));

    exp.push(Err(InvalidOpCode {
        start: 22,
        span: 2,
        opcode: 0xFF,
    }));

    exp.push(Ok(Packet {
        start: 24,
        span: 22,
        details: PacketDetails::Async,
    }));

    assert_eq!(results, exp);
}
