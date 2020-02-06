use stm_core::frame_decoder_new::{decode_frame, Error, ErrorReason::*};

#[test]
fn error_display() {
    let frame = [0; 16];
    decode_frame(&frame, None);
}
