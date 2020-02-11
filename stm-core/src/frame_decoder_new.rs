use std::convert::TryInto;
use std::fmt;
use std::result;

#[derive(Debug, PartialEq)]
pub enum ErrorReason {
    InvalidStreamId(u8),
    InvalidAuxByte(u8),
    PartialFrame(usize),
}

use self::ErrorReason::*;

#[derive(Debug, PartialEq)]
pub struct Error {
    pub offset: usize,
    pub reason: ErrorReason,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.reason {
            InvalidStreamId(id) => write!(f, "invalid stream id: {:#x} ({})", id, self.offset),
            InvalidAuxByte(byte) => write!(f, "invalid aux byte: {:#x} ({})", byte, self.offset),
            PartialFrame(size) => write!(f, "partial frame: {} bytes ({})", size, self.offset),
        }
    }
}

pub type Result = result::Result<Option<u8>, Error>;

/// Decode a series of frames.
///
/// # Arguments
///
///  * `frames` - A stream of bytes representing contiguous 16 byte frames.
///  * `stream` - The starting stream ID.
///  * `data` - Data handler closure. This closure takes two arguments: a stream ID and a
///             single byte of data associated with that stream ID.
///  * `error` - Error handler.  This is closure that takes an Error and returns a Result.  If the
///              Result is an Error, decoding will be halted, otherwise decoding will continue.
pub fn decode_frames<D, E>(
    frames: &[u8],
    stream_id: Option<u8>,
    mut data: D,
    mut error: E,
) -> Result
where
    D: FnMut(Option<u8>, u8),
    E: FnMut(Error) -> result::Result<(), Error>,
{
    let mut id = stream_id;
    let mut offset = 0;
    let mut iter = frames.chunks_exact(16);

    for frame in &mut iter {
        id = decode_frame(frame.try_into().unwrap(), id, &mut data, &mut error)?;
        offset += 16;
    }

    let remainder = iter.remainder().len();
    if remainder > 0 {
        error(Error {
            offset: offset,
            reason: PartialFrame(remainder),
        })?;
    }

    Ok(id)
}

/// Decode a single frame of data.
///
/// # Arguments
///
///  * `frame` - The frame of data to be decoded.
///  * `stream` - The starting stream ID.
///  * `data` - Stream data handler.  This is a closure that takes two arguments: a stream ID and a
///             single byte of data.
///  * `error` - Error handler.  This is closure that takes an Error.  If it returns an Error
///             decoding will be halted.  Otherwise decoding will continue.
///
pub fn decode_frame<D, E>(
    frame: &[u8; 16],
    stream_id: Option<u8>,
    mut data: D,
    mut error: E,
) -> Result
where
    D: FnMut(Option<u8>, u8),
    E: FnMut(Error) -> result::Result<(), Error>,
{
    let aux_byte = frame[15];

    if aux_byte & 0x80 == 0x80 && frame[14] & 0x01 == 0x01 {
        error(Error {
            offset: 0x15,
            reason: InvalidAuxByte(aux_byte),
        })?;
    }

    let mut cur_stream = stream_id;
    let mut next_stream = None;

    for (i, byte) in frame[..15].iter().enumerate() {
        if i % 2 == 0 {
            // Even byte: ID change OR data.
            let aux_bit = (aux_byte >> (i / 2)) & 0x01;
            if byte & 0x01 == 1 {
                if *byte == 0xFF {
                    error(Error {
                        offset: i,
                        reason: InvalidStreamId(0x7F),
                    })?;
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
                data(cur_stream, byte | aux_bit);
            }
        } else {
            // Odd byte: Data only.
            data(cur_stream, *byte);
            if let Some(_) = next_stream {
                cur_stream = next_stream;
                next_stream = None;
            }
        }
    }

    match next_stream {
        Some(_) => Ok(next_stream),
        None => Ok(cur_stream),
    }
}
