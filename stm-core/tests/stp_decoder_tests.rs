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
