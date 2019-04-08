use std::result;

#[derive(Debug, PartialEq)]
pub enum FrameBuilderError {
    InvalidOffset(usize),
    InvalidStreamId(u8),
}

use FrameBuilderError::*;

pub type Result = result::Result<(), FrameBuilderError>;

pub fn set_stream_id(frames: &mut [u8], offset: usize, id: u8, immediate: bool) -> Result {
    if offset % 2 != 0 || offset % 16 == 15 {
        return Err(InvalidOffset(offset));
    }

    if id >= 0xfe {
        return Err(InvalidStreamId(id));
    }

    let aux_offset = (offset / 16) + 15;
    frames[offset] = id << 1 | 0x01;

    let mask = 0x01 << ((offset % 16) / 2);
    if immediate {
        frames[aux_offset] &= !mask;
    } else {
        frames[aux_offset] |= mask;
    }

    Ok(())
}

pub fn set_stream_data(frames: &mut [u8], offset: usize, data: u8) -> Result {
    if offset % 16 == 15 {
        return Err(InvalidOffset(offset));
    }

    if offset % 2 == 0 {
        let aux_offset = offset - (offset % 16) + 15;
        frames[offset] = data & 0xFE;

        let mask = 0x01 << ((offset % 16) / 2);
        if data & 0x01 == 0 {
            frames[aux_offset] &= !mask;
        } else {
            frames[aux_offset] |= mask;
        }
    } else {
        frames[offset] = data;
    }

    Ok(())
}

pub struct FrameBuilder {
    frames: Vec<u8>,
    offset: usize,
    last_data: Option<u8>, // Last byte of data written.
}

impl FrameBuilder {
    pub fn new(capacity: usize) -> FrameBuilder {
        FrameBuilder {
            frames: Vec<u8>::with_capacity(capacity),
            offset: 0,
            last_data: None
        }
    }

    // See if we need to add a new frame:
    fn check(&mut self) {
        if self.offset == self.frames.len() {
            self.frames.resize(self.frames.len() + 16, 0);
        }
    }

    // Skip over the aux byte:
    fn increment_offset(&mut self) {
        self.offset += if self.offset % 16 == 15 { 2 } else { 1 };
    }

    fn set_id_direct(&mut self, value: u8, immediate: bool) -> Result {
        self.check();
        set_stream_id(&mut self.frames, self.offset, value, immediate)?;
        self.increment_offset();
        Ok(())
    }

    fn set_id(&mut self, value: u8) -> Result {
        if self.offset % 2 == 1 {
            self.offset -= 1;
            match self.last_data {
                None => self.set_id_full(value, true)?,
                Some(byte) => {self.set_id_full(value, false)?;
                    self.set_data(byte)?; }
            }
        } else {
            self.set_id_full(value, true)?;
            self.last_data = None;
        }
        Ok(())
    }

    pub fn immediate_id(mut self, value: u8) -> FrameBuilder {
        self.set_id_full(value, true).unwrap();
        self
    }

    pub fn delayed_id(mut self, value: u8) -> FrameBuilder {
        self.set_id_full(value, false).unwrap();
        self
    }

    fn set_data(&mut self, value: u8) {
        self.check();
        set_stream_data(&mut self.frames, self.offset, value).unwrap();
        self.increment_offset();
        self.last_data = Some(value);

    }

    pub fn data_span<F>(mut self, span: usize, mut f: F) -> FrameBuilder
    where
        F: FnMut() -> u8,
    {
        for _ in 0..span {
            self.set_data(f());
        }
        self
    }

    pub fn data(mut self, value: u8) -> FrameBuilder {
        self.set_data(value);
        self
    }

    pub fn build(self) -> Vec<u8> {
        self.frames
    }
}
