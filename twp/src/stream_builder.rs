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

enum ByteType {
    Data(u8),
    Id(u8, bool),
}

use ByteType::*;

pub struct StreamBuilder<'a, W>
where
    W: Write,
{
    out: &'a mut W,
    aux: u8,
    next_id: Option<u8>,
    even_byte: ByteType,
    frame_offset: usize,
    total: usize,
}

impl<'a, W> StreamBuilder<'a, W>
where
    W: Write,
{
    pub fn new(out: &'a mut W) -> Self {
        StreamBuilder {
            out: out,
            aux: 0,
            next_id: None,
            even_byte: Data(0),
            frame_offset: 0,
            total: 0,
        }
    }

    pub fn finish(&mut self) -> Result<usize, Error> {
        Ok(42)
    }

    pub fn id(&mut self, id: StreamId) -> Result<&mut Self, Error> {
        self.next_id = Some(id.into());
        Ok(self)
    }

    pub fn data(&mut self, data: &[u8]) -> Result<&mut Self, Error> {
        if data.len() == 0 {
            return Ok(self);
        }

        if let Some(id) = self.next_id {
            self.push_byte(Id(id, false))?;
            self.next_id = None;
        }

        for d in data {
            self.push_byte(Data(*d))?;
        }

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

    fn push_byte(&mut self, byte: ByteType) -> Result<(), Error> {
        if self.frame_offset % 2 == 0 {
            if self.frame_offset < 14 {
                self.even_byte = byte;
            } else {
                let halfword = [self.set_even_byte(byte), self.aux];
                self.total += self.out.write(&halfword)?;
                self.frame_offset = 0;
                self.aux = 0;
            }
        } else {
            match byte {
                Id(id, _) => {
                    let tmp = self.set_even_byte(Id(id, true));
                    self.out.write(&[tmp])?;
                    match self.even_byte {
                        Data(d) => self.out.write(&[d])?,
                        Id(_, _) => panic!(),
                    };
                }
                Data(d) => {
                    let even_byte = self.even_byte;
                    let tmp = self.set_even_byte(even_byte);
                    self.out.write(&[tmp])?;
                    self.out.write(&[d])?;
                }
            }

            self.total += 2;
            self.frame_offset += 2;
        }
        Ok(())
    }

    fn set_even_byte(&mut self, byte: ByteType) -> u8 {
        let mask = 0x01 << self.frame_offset / 2;
        match byte {
            Data(d) => {
                if d & 0x01 == 0 {
                    self.aux &= !mask;
                } else {
                    self.aux |= mask;
                }
                d & 0xFE
            }
            Id(id, deferred) => id,
        }
    }
}
