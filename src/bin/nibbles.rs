use std::error::Error;
use std::path::PathBuf;
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

fn main() -> Result<(), Box<Error>> {
    let files: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();

    if files.is_empty() {
        Err("usage: nibbles FILE {FILE ...}")?;
    }

    Ok(())
}
