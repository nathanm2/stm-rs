use std::result;

pub struct FrameDecoder {
    stream: Option<u8>,
}

#[derive(Debug)]
pub enum FrameDecoderError {
    InvalidStreamId(usize),
    InvalidAuxByte(usize),
    PartialFrame(usize),
}

use self::FrameDecoderError::*;

pub type Result<T> = result::Result<T, FrameDecoderError>;

impl FrameDecoder {
    pub fn new() -> Self {
        FrameDecoder { stream: None }
    }

    pub fn decode<H>(&mut self, frames: &[u8], handler: H) -> Result<()>
    where
        H: Fn(Option<u8>, u8),
    {
        let mut cur_stream = self.stream;

        for (count, frame) in frames.chunks(16).enumerate() {
            if frame.len() < 16 {
                return Err(PartialFrame(count * 16));
            }

            let mut next_stream = None;
            let aux_byte = frame[15];
            for (i, byte) in frame[..15].iter().enumerate() {
                if i % 2 == 0 {
                    // Even byte: ID change OR data.
                    let aux_bit = (aux_byte >> (i / 2)) & 0x01;
                    if byte & 0x01 == 1 {
                        if *byte == 0xFF {
                            return Err(InvalidStreamId(count * 16 + i));
                        }
                        // Id Change.
                        if aux_bit == 1 {
                            if i == 14 {
                                return Err(InvalidAuxByte(count * 16 + i));
                            }
                            // Delayed ID Change.
                            next_stream = Some(byte >> 1);
                        } else {
                            // Immediate ID change.
                            cur_stream = Some(byte >> 1);
                        }
                    } else {
                        handler(cur_stream, byte | aux_bit);
                    }
                } else {
                    // Odd byte: Data only.
                    handler(cur_stream, *byte);
                    if let Some(_) = next_stream {
                        cur_stream = next_stream;
                        next_stream = None;
                    }
                }
            }
        }
        self.stream = cur_stream;
        return Ok(());
    }
}
