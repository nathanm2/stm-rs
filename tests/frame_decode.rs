use stm::frame_decoder::FrameDecoderError::*;
use stm::frame_decoder::{FrameConsumer, FrameDecoder};

struct NullConsumer;

impl FrameConsumer for NullConsumer {
    fn stream_byte(&mut self, _stream: Option<u8>, _data: u8) {}
}

#[test]
fn partial_frame() {
    let mut fd = FrameDecoder::new();
    let mut c = NullConsumer {};
    let frame = [0; 12];

    assert_eq!(fd.decode(&frame, &mut c, 0), Err(PartialFrame(12)));
}

#[test]
fn bad_path() {
    let mut fd = FrameDecoder::new();
    let mut c = NullConsumer {};
    let mut frame = [0; 32];

    frame[0] = 0x03;
    frame[2] = 0xFF;
    assert_eq!(fd.decode(&frame, &mut c, 0), Err(InvalidStreamId(2)));
}
