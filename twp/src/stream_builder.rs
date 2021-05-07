pub use super::frame_parser::StreamId;
use std::io::{self, Write};
use std::result::Result;

#[derive(Debug, PartialEq)]
pub enum Error {
    InvalidStreamId(u8, usize),
    IOError(io::ErrorKind),
}

impl std::convert::From<io::Error> for Error {
    fn from(error: io::Error) -> Error {
        IOError(error.kind())
    }
}

use Error::*;

#[derive(Copy, Clone)]
enum ByteType {
    Data(u8),
    Id(u8),
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
        self.pad_frame()?;
        Ok(self.total)
    }

    pub fn pad_frame(&mut self) -> Result<&mut Self, Error> {
        if self.frame_offset == 14 {
            self.push_byte(Id(0))?;
        } else if self.frame_offset > 0 {
            self.id_data(StreamId::Null, &vec![0; 16 - self.frame_offset - 1])?;
        }
        Ok(self)
    }

    pub fn id(&mut self, id: StreamId) -> Result<&mut Self, Error> {
        let id_value = id.into();
        if id_value >= 0x7F {
            return Err(InvalidStreamId(id_value, self.total));
        }

        self.next_id = Some(id_value);
        Ok(self)
    }

    pub fn data(&mut self, data: &[u8]) -> Result<&mut Self, Error> {
        // Easy case:
        if data.len() == 0 {
            return Ok(self);
        }

        if let Some(id) = self.next_id {
            self.push_byte(Id(id))?;
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
        // Even bytes are buffered unless it's last even byte of the frame.  Buffering is needed to
        // support 'deferred' ID changes in the stream.
        if self.frame_offset % 2 == 0 {
            if self.frame_offset < 14 {
                self.even_byte = byte;
                self.frame_offset += 1;
            } else {
                let even = self.even_byte(byte, true);
                self.total += self.out.write(&[even, self.aux])?;
                self.frame_offset = 0;
                self.aux = 0;
            }
        } else {
            match byte {
                Id(i) => {
                    let even = self.even_byte_id(i, false);
                    match self.even_byte {
                        Data(d) => self.out.write(&[even, d])?,
                        Id(_) => panic!(),
                    };
                }
                Data(d) => {
                    let even = self.even_byte(self.even_byte, true);
                    self.out.write(&[even, d])?;
                }
            }

            self.total += 2;
            self.frame_offset += 1;
        }
        Ok(())
    }

    fn even_byte(&mut self, byte: ByteType, immediate: bool) -> u8 {
        match byte {
            Data(d) => self.even_byte_data(d),
            Id(i) => self.even_byte_id(i, immediate),
        }
    }

    fn even_byte_id(&mut self, id: u8, immediate: bool) -> u8 {
        let mask = 0x01 << self.frame_offset / 2;
        if immediate {
            self.aux &= !mask;
        } else {
            self.aux |= mask;
        }
        id << 1 | 0x01
    }

    fn even_byte_data(&mut self, data: u8) -> u8 {
        let mask = 0x01 << self.frame_offset / 2;
        if data & 0x01 == 0 {
            self.aux &= !mask;
        } else {
            self.aux |= mask;
        }
        data & 0xFE
    }
}
