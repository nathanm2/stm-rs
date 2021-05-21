use super::error::{Error, ErrorKind, Result};

pub enum Layer<'a> {
    /// Layer T1: Padding packet.  Emitted when the trace port is idle.  Can be found within a frame
    /// or between frames and is always aligned on a 16-bit boundary.
    Padding { offset: usize },

    /// Layer T2: Frame synchronization packet.  Emitted periodically between frames.  Used to
    /// determine frame alignment within a stream of bytes.
    FrameSync { offset: usize },

    /// Layer T3: Data frame.  Sixteen byte data frame.  Since padding packets can appear within a
    /// frame, a frame's bytes are not always contiguous.
    Frame {
        frame: &'a [u8; 16],
        offsets: &'a [usize; 16],
    },
}

pub struct LayerParser {
    frame: [u8; 16],
    frame_offsets: [usize; 16],
    frame_idx: usize,
    ff_count: usize,
    syncd: bool, // synchronized
    offset: usize,
    padding: bool,
}

impl LayerParser {
    pub fn new(syncd: bool, padding: bool, offset: usize) -> LayerParser {
        LayerParser {
            frame: [0; 16],
            frame_offsets: [0; 16],
            frame_idx: 0,
            ff_count: 0,
            syncd,
            offset,
            padding,
        }
    }

    pub fn parse<H>(&mut self, data: &[u8], mut handler: H) -> Result<()>
    where
        H: FnMut(Result<Layer>) -> Result<()>,
    {
        for d in data {
            let result = self.process_byte(*d, &mut handler);
            self.offset += 1;
            result?;
        }

        Ok(())
    }

    pub fn finish<H>(&mut self, mut handler: H) -> Result<()>
    where
        H: FnMut(Result<Layer>) -> Result<()>,
    {
        let ff_count = self.ff_count;
        self.ff_count = 0;

        for i in 0..ff_count {
            self.frame_byte(0xFF, self.offset - (ff_count - i), &mut handler)?;
        }
        Ok(())
    }

    fn process_byte<H>(&mut self, byte: u8, mut handler: H) -> Result<()>
    where
        H: FnMut(Result<Layer>) -> Result<()>,
    {
        if byte == 0xFF {
            if self.ff_count < 3 {
                self.ff_count += 1;
            } else {
                if self.ff_count == 2 {
                    self.frame_byte(byte, self.offset - 2, &mut handler)?;
                }
                self.frame_byte(byte, self.offset - 3, &mut handler)?;
            }
        } else if byte == 0x7F && self.ff_count == 3 {
            self.syncd = true;
            self.ff_count = 0;

            // We got a frame sync in the middle of a frame.  This could indicate that we lost
            // synchronization since the prior FrameSync and some number of the preceding frames are
            // invalid.
            if self.frame_idx > 0 {
                self.frame_idx = 0;
                handler(Err(Error {
                    kind: ErrorKind::InvalidFrames,
                    offset: self.offset,
                }))?
            }
            handler(Ok(Layer::FrameSync {
                offset: self.offset - 3,
            }))?;

            // Padding packets are aligned on a 16-bit boundary with respect to a frame.
        } else if self.padding
            && byte == 0x7F
            && self.syncd
            && self.ff_count > 0
            && ((self.frame_idx + self.ff_count + 1) % 2) == 0
        {
            let extra_ff = self.ff_count == 2;
            self.ff_count = 0;
            if extra_ff {
                self.frame_byte(byte, self.offset - 2, &mut handler)?;
            }
            handler(Ok(Layer::Padding {
                offset: self.offset - 1,
            }))?;
        } else if self.syncd {
            let ff_count = self.ff_count;
            self.ff_count = 0;

            for i in 0..ff_count {
                self.frame_byte(0xFF, self.offset - (ff_count - i), &mut handler)?;
            }
            self.frame_byte(byte, self.offset, &mut handler)?;
        }

        Ok(())
    }

    fn frame_byte<H>(&mut self, byte: u8, offset: usize, mut handler: H) -> Result<()>
    where
        H: FnMut(Result<Layer>) -> Result<()>,
    {
        self.frame[self.frame_idx] = byte;
        self.frame_offsets[self.frame_idx] = offset;
        self.frame_idx += 1;
        if self.frame_idx == self.frame.len() {
            self.frame_idx = 0;
            handler(Ok(Layer::Frame {
                frame: &self.frame,
                offsets: &self.frame_offsets,
            }))?;
        }
        Ok(())
    }
}
