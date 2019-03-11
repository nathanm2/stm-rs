#[test]
fn it_works() {
    let mut d = stm::FrameDecoder::new();
    let frame = [0; 16];
    d.decode_frame(&frame)
}
