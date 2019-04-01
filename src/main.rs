use stm::frame_decoder::{FrameConsumer, FrameDecoder};

struct NibbleDumper {}

impl FrameConsumer for NibbleDumper {
    fn stream_byte(&mut self, stream: Option<u8>, data: u8) {
        match stream {
            Some(id) => println!("{} = {}", id, data),
            None => println!("<Unknown> = {}", data),
        }
    }
}

fn main() {
    let mut fd = FrameDecoder::new();
    let mut dumper = NibbleDumper {};
    let mut frame = [0; 32];

    frame[0] = 0x03;
    frame[2] = 0x05;

    fd.decode(&frame, &mut dumper, 0)
        .expect("Error encountered");
}
