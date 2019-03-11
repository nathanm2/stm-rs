#[allow(dead_code)]
pub struct FrameDecoder {
    stream: Option<u8>,
}

impl FrameDecoder {
    pub fn new() -> Self {
        FrameDecoder { stream: None }
    }

    pub fn decode(&mut self, data: [u8; 16]) {
        let _aux = data[15];
        for i in 0..15 {
            println!("{}={}", i, data[i]);
        }
    }
}
