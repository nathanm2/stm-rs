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
        write!(f, "{} ({})", self.reason, self.offset)
    }
}

pub type Result = result::Result<Option<u8>, Error>;

/// Decode a series of frames.
///
/// # Arguments
///
///  * `frames` - A stream of bytes representing contiguous 16 byte frames.
///  * `stream_id` - The starting stream ID.
pub fn decode_frames<H>(frames: &[u8], stream_id: Option<u8>, mut handler: H) -> Result
where
    H: FnMut(result::Result<(Option<u8>, u8), Error>) -> result::Result<(), Error>,
{
    let mut id = stream_id;
    let mut offset = 0;
    let mut iter = frames.chunks_exact(16);

    for frame in &mut iter {
        id = decode_frame(frame.try_into().unwrap(), id, |mut r| {
            if let Err(ref mut e) = r {
                e.offset += offset;
            }
            handler(r)
        })?;
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
pub fn decode_frame<H>(frame: &[u8; 16], stream_id: Option<u8>, mut handler: H) -> Result
where
    H: FnMut(result::Result<(Option<u8>, u8), Error>) -> result::Result<(), Error>,
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
                handler(Ok((cur_stream, byte | aux_bit)))?;
            }
        } else {
            // Odd byte: Data only.
            handler(Ok((cur_stream, *byte)))?;
            if let Some(_) = next_stream {
                cur_stream = next_stream;
                next_stream = None;
            }
        }
    }

    Ok(cur_stream)
}
