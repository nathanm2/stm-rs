pub struct FrameDecoder {
    stream: Option<u8>,
}

impl FrameDecoder {
    pub fn new() -> Self {
        FrameDecoder { stream: None }
    }

    pub fn decode_frame(&mut self, frame: &[u8; 16]) {
        let mut cur_stream = self.stream;
        let mut next_stream = None;
        let aux_byte = frame[15];

        for (i, byte) in frame[..15].iter().enumerate() {
            if i % 2 == 0 {  // Even: data or ID change.
                let aux_bit = (aux_byte >> (i/2)) & 0x01;
                if byte & 0x01 == 1 { // Id Change.
                    if aux_bit == 1 { // Delayed ID Change.
                        next_stream = Some(byte >> 1);
                    } else { // Immediate ID change.
                        cur_stream = Some(byte >> 1);
                    }
                } else { // data
                    println!("{}={}", i, byte | aux_bit);
                }
            } else { // Odd : data only.
                println!("{}={}", i, byte);
                if let Some(_) = next_stream {
                    cur_stream = next_stream;
                    next_stream = None;
                }
            }
        }
        self.stream = cur_stream;
    }
}
