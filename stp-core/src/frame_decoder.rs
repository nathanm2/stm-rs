use std::convert::TryInto;
use std::fmt;
use std::result;

#[derive(Debug, PartialEq)]
pub enum ErrorReason {
    InvalidStreamId(u8),
    InvalidAuxByte(u8),
    PartialFrame(usize),
    Stop,
}

use self::ErrorReason::*;

impl fmt::Display for ErrorReason {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            InvalidStreamId(id) => write!(f, "invalid stream id: {:#x}", id),
            InvalidAuxByte(byte) => write!(f, "invalid aux byte: {:#x}", byte),
            PartialFrame(size) => write!(f, "truncated frame: {} bytes", size),
            Stop => write!(f, "stopped"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Error {
    pub offset: usize,
    pub reason: ErrorReason,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}, offset: {:#x}", self.reason, self.offset)
    }
}

pub struct Data {
    pub id: Option<u8>,
    pub data: u8,
    pub offset: usize,
}

pub type Result<S> = result::Result<S, Error>;

/// Decode a series of frames.
///
/// # Arguments
///
///  * `frames` - A stream of bytes representing contiguous 16 byte frames.
///  * `stream_id` - The starting stream ID.
pub fn decode_frames<H>(frames: &[u8], stream_id: Option<u8>, mut handler: H) -> Result<Option<u8>>
where
    H: FnMut(Result<Data>) -> Result<()>,
{
    let mut id = stream_id;
    let mut offset = 0;
    let mut iter = frames.chunks_exact(16);

    for frame in &mut iter {
        id = decode_frame_offset(frame.try_into().unwrap(), id, &mut handler, offset)?;
        offset += 16;
    }

    let remainder = iter.remainder().len();
    if remainder > 0 {
        handler(Err(Error {
            offset: offset,
            reason: PartialFrame(remainder),
        }))?;
    }

    Ok(id)
}

/// Decode a single frame of data.
///
/// # Arguments
///
///  * `frame` - The frame of data to be decoded.
///  * `stream` - The starting stream ID.
pub fn decode_frame<H>(
    frame: &[u8; 16],
    stream_id: Option<u8>,
    mut handler: H,
) -> Result<Option<u8>>
where
    H: FnMut(Result<Data>) -> Result<()>,
{
    let mut aux_byte = frame[15];

    // If byte 14 contains a frame id, it's aux bit should be zero per the spec:
    if aux_byte & 0x80 == 0x80 && frame[14] & 0x01 == 0x01 {
        handler(Err(Error {
            offset: 15,
            reason: InvalidAuxByte(aux_byte),
        }))?;
        aux_byte = aux_byte & 0x7F;
    }

    let mut cur_stream = stream_id;
    let mut next_stream = None;

    for (i, byte) in frame[..15].iter().enumerate() {
        if i % 2 == 0 {
            // Even byte: ID change OR data.
            let aux_bit = (aux_byte >> (i / 2)) & 0x01;
            if byte & 0x01 == 1 {
                if *byte == 0xFF {
                    handler(Err(Error {
                        offset: i,
                        reason: InvalidStreamId(0x7F),
                    }))?;
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
                handler(Ok(Data {
                    id: cur_stream,
                    data: byte | aux_bit,
                    offset: i,
                }))?;
            }
        } else {
            // Odd byte: Data only.
            handler(Ok(Data {
                id: cur_stream,
                data: *byte,
                offset: i,
            }))?;
            if let Some(_) = next_stream {
                cur_stream = next_stream;
                next_stream = None;
            }
        }
    }

    Ok(cur_stream)
}

pub fn decode_frame_offset<H>(
    frame: &[u8; 16],
    stream_id: Option<u8>,
    mut handler: H,
    offset: usize,
) -> Result<Option<u8>>
where
    H: FnMut(Result<Data>) -> Result<()>,
{
    decode_frame(frame, stream_id, |mut r| {
        match r {
            Err(ref mut e) => e.offset += offset,
            Ok(ref mut d) => d.offset += offset,
        }
        handler(r)
    })
}

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
