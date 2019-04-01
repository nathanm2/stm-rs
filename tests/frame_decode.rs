fn display(stream: Option<u8>, byte: u8) {
    match stream {
        Some(id) => println!("{} = {}", id, byte),
        None => println!("<Unknown> = {}", byte),
    }
}

#[test]
fn good_path_single() {
    let mut fd = stm::FrameDecoder::new();
    let mut frame = [0; 32];

    frame[0] = 0x03;
    frame[2] = 0x05;
    d.decode(&frame, display).expect("Not Ok");
}

#[test]
fn bad_path() {
    let mut d = stm::FrameDecoder::new();
    let mut frame = [0; 32];

    frame[0] = 0x03;
    frame[2] = 0xFF;
    let r = d.decode(&frame, display);
    match r {
        Ok(_) => println!("Everything works"),
        Err(code) => println!("Failure encountered: {:?}", code),
    }
}
