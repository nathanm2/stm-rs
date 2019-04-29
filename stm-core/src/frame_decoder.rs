use std::fmt;
use std::result;

pub struct FrameDecoder {
    stream: Option<u8>,
}

#[derive(Debug, PartialEq)]
pub enum Error {
    InvalidStreamId(usize),
    InvalidAuxByte(usize),
    PartialFrame(usize),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            InvalidStreamId(offset) => write!(f, "invalid stream id at offset {}", offset),
            InvalidAuxByte(offset) => write!(f, "invalid aux byte at offset {}", offset),
            PartialFrame(offset) => write!(f, "partial frame at offset {}", offset),
        }
    }
}

use self::Error::*;

pub type Result = result::Result<(), Error>;

pub trait FrameConsumer {
    fn stream_byte(&mut self, stream: Option<u8>, data: u8);

    fn end_of_frame(&mut self) {}
}

impl FrameDecoder {
    /// Create a new FrameDecoder.
    pub fn new() -> Self {
        FrameDecoder { stream: None }
    }

    /// Decode a stream of frames.
    pub fn decode<C>(&mut self, frames: &[u8], cb: &mut C, offset: usize) -> Result
    where
        C: FrameConsumer,
    {
        for (i, frame) in frames.chunks(16).enumerate() {
            self.decode_frame(frame, cb, offset + i * 16)?;
        }

        Ok(())
    }

    /// Decode a single frame.
    fn decode_frame<C>(&mut self, frame: &[u8], cb: &mut C, offset: usize) -> Result
    where
        C: FrameConsumer,
    {
        if frame.len() < 16 {
            return Err(PartialFrame(frame.len()));
        }

        let aux_byte = frame[15];

        if aux_byte & 0x80 == 0x80 && frame[14] & 0x01 == 0x01 {
            return Err(InvalidAuxByte(offset + 15));
        }

        let mut cur_stream = self.stream;
        let mut next_stream = None;

        for (i, byte) in frame[..15].iter().enumerate() {
            if i % 2 == 0 {
                // Even byte: ID change OR data.
                let aux_bit = (aux_byte >> (i / 2)) & 0x01;
                if byte & 0x01 == 1 {
                    if *byte == 0xFF {
                        return Err(InvalidStreamId(offset + i));
                    }
                    // Id Change.
                    if aux_bit == 1 {
                        // Delayed ID Change.
                        next_stream = Some(byte >> 1);
                    } else {
                        // Immediate ID change.
                        cur_stream = Some(byte >> 1);
                    }
                } else {
                    cb.stream_byte(cur_stream, byte | aux_bit);
                }
            } else {
                // Odd byte: Data only.
                cb.stream_byte(cur_stream, *byte);
                if let Some(_) = next_stream {
                    cur_stream = next_stream;
                    next_stream = None;
                }
            }
        }

        self.stream = cur_stream;
        cb.end_of_frame();

        Ok(())
    }
}
