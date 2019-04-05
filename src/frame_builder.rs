use std::result;

#[derive(Debug, PartialEq)]
pub enum FrameBuilderError {
    InvalidOffset(usize),
}

use FrameBuilderError::*;

pub type Result = result::Result<(), FrameBuilderError>;

pub fn set_stream_id(frames: &mut [u8], offset: usize, id: u8, immediate: bool) -> Result {
    if offset % 2 != 0 || offset % 16 == 15 {
        return Err(InvalidOffset(offset));
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
    frames: Vec<u8>
    offset: usize,
}

impl FrameBuilder {
    pub fn new(frame_count: usize) -> FrameBuilder {
        FrameBuilder {
            frames: vec![0; frame_count * 16],
            offset: 0,
        }
    }

    fn check(&mut self) {
        if self.offset == self.frames.len() {
            self.frames.resize(self.frames.len() + 16, 0);
        }
    }

    fn increment_offset(&mut self) {
        self.offset += if self.offset % 16 == 15 { 2 } else { 1 };
    }

    pub fn id(mut self, value: u8, immediate: bool) -> FrameBuilder {
        self.check();
        set_stream_id(&mut self.frames, self.offset, value, immediate).unwrap();
        self.increment_offset();
        self
    }

    pub fn data(mut self, value: u8) -> FrameBuilder {
        self.check();
        set_stream_data(&mut self.frames, self.offset, value).unwrap();
        self.increment_offset();
        self
    }

    pub fn build(self) -> Vec<u8> {
        self.frames
    }
}
