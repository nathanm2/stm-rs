use stm_core::stp_decoder::{Error::*, Result, StpDecoder};

#[test]
fn basic_fsync() {
    let mut results = Vec::<Result>::new();
    let mut decoder = StpDecoder::new();
    let stream = [0; 32];

    decoder.decode_bytes(&stream, |r| results.push(r));
}
