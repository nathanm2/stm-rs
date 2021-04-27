use super::frame_parser::decode_frame_offset;
use super::types::{Data, Error, ErrorReason::*, Result};

pub struct FrameDecoder {
    frame: [u8; 16],
    frame_idx: usize,
    ff_count: usize,
    aligned: bool,
    stream_id: Option<u8>,
    offset: usize,
}

pub const FSYNC: [u8; 4] = [0xFF, 0xFF, 0xFF, 0x7F];

impl FrameDecoder {
    pub fn new(aligned: bool, stream_id: Option<u8>) -> FrameDecoder {
        FrameDecoder {
            frame: [0; 16],
            frame_idx: 0,
            ff_count: 0,
            aligned,
            stream_id,
            offset: 0,
        }
    }

    pub fn decode<H>(&mut self, data: &[u8], mut handler: H) -> Result<()>
    where
        H: FnMut(Result<Data>) -> Result<()>,
    {
        for d in data {
            if *d == 0xFF && self.ff_count < 3 {
                self.ff_count += 1;
            } else if *d == 0x7F && self.ff_count == 3 {
                self.aligned = true;
                if self.frame_idx > 0 {
                    handler(Err(Error {
                        offset: self.offset,
                        reason: PartialFrame(self.frame_idx),
                    }))?;
                }
                self.offset += 4 + self.frame_idx;
                self.frame_idx = 0;
                self.ff_count = 0;
            } else if self.aligned {
                if *d != 0xFF {
                    for _ in 0..self.ff_count {
                        self.process_byte(0xFF, &mut handler)?;
                    }
                    self.ff_count = 0;
                }
                self.process_byte(*d, &mut handler)?;
            } else {
                self.offset += 1;
            }
        }
        Ok(())
    }

    pub fn finish<H>(&mut self, mut handler: H) -> Result<()>
    where
        H: FnMut(Result<Data>) -> Result<()>,
    {
        // Only take action if the AUX byte is 0xFF.  In all other cases we're dealing with a
        // partial frame or a truncated FSYNC
        if self.ff_count == 1 && self.frame_idx == 15 {
            self.process_byte(0xFF, &mut handler)?;
            self.ff_count = 0;
        }
        Ok(())
    }

    fn process_byte<H>(&mut self, byte: u8, mut handler: H) -> Result<()>
    where
        H: FnMut(Result<Data>) -> Result<()>,
    {
        self.frame[self.frame_idx] = byte;
        self.frame_idx += 1;
        if self.frame_idx == self.frame.len() {
            let offset = self.offset;
            self.offset += self.frame.len();
            self.frame_idx = 0;
            self.stream_id =
                decode_frame_offset(&self.frame, self.stream_id, &mut handler, offset)?;
        }
        Ok(())
    }
}
