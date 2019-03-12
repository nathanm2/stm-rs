use std::error;

fn display(stream: Option<u8>, byte: u8) {
    match stream {
        Some(id) => println!("{} = {}", id, byte),
        None => println!("<Unknown> = {}", byte),
    }
}

#[test]
fn good_path() {
    let mut d = stm::FrameDecoder::new();
    let mut frame = [0; 16];

    frame[0] = 0x03;
    frame[2] = 0x05;
    d.decode_frame(&frame, display)
}

#[test]
fn bad_path() {
    let mut d = stm::FrameDecoder::new();
    let mut frame = [0; 16];

    frame[0] = 0x03;
    frame[2] = 0xFF;
    let r = d.decode_frame_safe(&frame, display);
    match r {
        Ok(_) => println!("Everything works"),
        Err(code) => println!("Failure encountered: {:?}", code),
    }
}
