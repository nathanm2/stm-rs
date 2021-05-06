use super::frame_parser::StreamId;
use std::io::{self, Write};
use std::result::Result;

#[derive(Debug)]
pub enum Error {
    InvalidOffset(usize),
    InvalidStreamId(usize, u8),
    InvalidDelayedId(usize, u8),
    MissingData(usize),
    IOError(io::Error),
}

impl std::convert::From<io::Error> for Error {
    fn from(error: io::Error) -> Error {
        IOError(error)
    }
}

use Error::*;

enum EvenByte {
    Data(u8),
    Id(u8, bool),
}

use EvenByte::*;

pub struct StreamBuilder<'a, W>
where
    W: Write,
{
    out: &'a mut W,
    aux: u8,
    even_byte: Option<EvenByte>,
    frame_offset: usize,
}

impl<'a, W> StreamBuilder<'a, W>
where
    W: Write,
{
    pub fn new(out: &'a mut W) -> Self {
        StreamBuilder {
            out: out,
            aux: 0,
            even_byte: None,
            frame_offset: 0,
        }
    }

    pub fn done(&mut self) -> Result<usize, Error> {
        Ok(42)
    }

    pub fn id(&mut self, id: StreamId) -> Result<&mut Self, Error> {
        let id = [id.into()];
        self.out.write(&id)?;
        Ok(self)
    }

    pub fn data(&mut self, data: &[u8]) -> Result<&mut Self, Error> {
        Ok(self)
    }

    pub fn id_data(&mut self, id: StreamId, data: &[u8]) -> Result<&mut Self, Error> {
        Ok(self.id(id)?.data(data)?)
    }

    pub fn frame_sync(&mut self) -> Result<&mut Self, Error> {
        Ok(self)
    }

    pub fn halfword_sync(&mut self) -> Result<&mut Self, Error> {
        Ok(self)
    }

    fn set_even_byte(&mut self, even_byte: EvenByte) -> u8 {
        let mask = 0x01 << self.frame_offset / 2;
        match even_byte {
            Data(d) => {
                if d & 0x01 == 0 {
                    self.aux &= !mask;
                } else {
                    self.aux |= mask;
                }
                d & 0xFE
            }
            Id(id, deferred) => {

            ImmediateId(id) => {}
            DeferredId(id) => {}
        }
    }
}
