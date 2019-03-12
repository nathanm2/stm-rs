use std::result;

pub struct FrameDecoder {
    stream: Option<u8>,
}

#[derive(Debug)]
pub enum FrameDecoderError {
    InvalidStreamId,
    InvalidAuxByte,
}

use self::FrameDecoderError::*;

pub type Result<T> = result::Result<T, FrameDecoderError>;

impl FrameDecoder {
    pub fn new() -> Self {
        FrameDecoder { stream: None }
    }

    pub fn decode_frame<B>(&mut self, frame: &[u8; 16], out: B)
    where
        B: Fn(Option<u8>, u8),
    {
        let mut cur_stream = self.stream;
        let mut next_stream = None;
        let aux_byte = frame[15];

        for (i, byte) in frame[..15].iter().enumerate() {
            // Even byte: ID change OR data.
            if i % 2 == 0 {
                let aux_bit = (aux_byte >> (i / 2)) & 0x01;
                // Id Change.
                if byte & 0x01 == 1 {
                    if aux_bit == 1 {
                        // Delayed ID Change.
                        next_stream = Some(byte >> 1);
                    } else {
                        // Immediate ID change.
                        cur_stream = Some(byte >> 1);
                    }
                } else {
                    out(cur_stream, byte | aux_bit);
                }
            } else {
                // Odd byte: Data only.
                out(cur_stream, *byte);
                if let Some(_) = next_stream {
                    cur_stream = next_stream;
                    next_stream = None;
                }
            }
        }
        self.stream = cur_stream;
    }

    pub fn decode_frame_safe<B>(&mut self, frame: &[u8; 16], out: B) -> Result<()>
    where
        B: Fn(Option<u8>, u8),
    {
        let mut cur_stream = self.stream;
        let mut next_stream = None;
        let aux_byte = frame[15];

        for (i, byte) in frame[..15].iter().enumerate() {
            if i % 2 == 0 {
                // Even byte: ID change OR data.
                let aux_bit = (aux_byte >> (i / 2)) & 0x01;
                if byte & 0x01 == 1 {
                    if *byte == 0xFF {
                        return Err(InvalidStreamId);
                    }
                    // Id Change.
                    if aux_bit == 1 {
                        if i == 14 {
                            return Err(InvalidAuxByte);
                        }
                        // Delayed ID Change.
                        next_stream = Some(byte >> 1);
                    } else {
                        // Immediate ID change.
                        cur_stream = Some(byte >> 1);
                    }
                } else {
                    out(cur_stream, byte | aux_bit);
                }
            } else {
                // Odd byte: Data only.
                out(cur_stream, *byte);
                if let Some(_) = next_stream {
                    cur_stream = next_stream;
                    next_stream = None;
                }
            }
        }
        self.stream = cur_stream;
        Ok(())
    }
}
