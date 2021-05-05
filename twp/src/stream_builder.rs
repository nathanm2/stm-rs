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

pub struct StreamBuilder<'a, W>
where
    W: Write,
{
    out: &'a mut W,
    id: u8,
    aux: u8,
    last: Option<u8>,
    offset: usize,
    total: usize,
}

impl<'a, W> StreamBuilder<'a, W>
where
    W: Write,
{
    pub fn new(out: &'a mut W) -> Self {
        StreamBuilder {
            out: out,
            id: 0,
            aux: 0,
            last: None,
            offset: 0,
            total: 0,
        }
    }

    pub fn finish(&mut self) -> Result<usize, Error> {
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

    fn write_data_byte(&mut self, byte: u8) -> Result<usize, Error> {
        Ok(0)
    }
}
