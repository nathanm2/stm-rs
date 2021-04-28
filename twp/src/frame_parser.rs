//! Parses MIPI TWP frames (aka TPIU) into data values.

use super::error::{ErrorKind, ErrorKind::*, Result};
use std::convert::TryInto;
use std::result;

/// A single byte of frame data.
pub struct FrameByte {
    /// The byte value.
    pub data: u8,
    /// The byte's associated Stream ID.
    pub id: u8,
}

/// Parse a single TWP frame.
///
/// Parses `frame` and invokes `handler` for each byte of data or error encountered.
///
/// `stream_id` specifies the initial Stream ID. The final stream ID encountered while parsing the
/// frame is returned to the caller, which could be `stream_id` if the stream ID was not modified
/// within the frame.
///
/// `handler` takes a `twp::error::Result<FrameData>` as input parameter.  This will contain either
/// a single byte of data associated with a particular Stream ID *or* a `twp::error::Error` if a
/// problem was encountered during parsing.  If `handler` returns an error, frame parsing will
/// stop immediately regardless of whether an error was encountered.
///
/// All offsets passed `handler` are relative to the frame.
///
pub fn parse_frame<H>(frame: &[u8; 16], stream_id: u8, mut handler: H) -> Result<u8>
where
    H: FnMut(result::Result<FrameByte, ErrorKind>, usize) -> Result<()>,
{
    let mut aux_byte = frame[15];

    // If byte 14 contains a frame id, it's aux bit should be zero per the spec:
    if aux_byte & 0x80 == 0x80 && frame[14] & 0x01 == 0x01 {
        handler(Err(Error {
            kind: InvalidAuxByte(aux_byte),
            offset: 15,
        }))?;
        aux_byte = aux_byte & 0x7F;
    }

    let mut cur_stream = stream_id;
    let mut next_stream = 0;
    let mut delayed = false;

    for (i, byte) in frame[..15].iter().enumerate() {
        if i % 2 == 0 {
            // Even byte: ID change OR data.
            let aux_bit = (aux_byte >> (i / 2)) & 0x01;
            if byte & 0x01 == 1 {
                if *byte == 0xFF {
                    handler(Err(Error {
                        kind: InvalidStreamId(0x7F),
                        offset: i,
                    }))?;
                }
                // Id Change.
                if aux_bit == 1 {
                    // Delayed ID Change.
                    next_stream = byte >> 1;
                    delayed = true;
                } else {
                    // Immediate ID change.
                    cur_stream = byte >> 1;
                }
            } else {
                handler(Ok(FrameByte {
                    id: cur_stream,
                    data: byte | aux_bit,
                    offset: i,
                }))?;
            }
        } else {
            // Odd byte: Data only.
            handler(Ok(FrameByte {
                id: cur_stream,
                data: *byte,
                offset: i,
            }))?;
            if delayed == true {
                cur_stream = next_stream;
                delayed = false;
            }
        }
    }

    Ok(cur_stream)
}

/// Decode a series of frames.
///
/// # Arguments
///
///  * `frames` - A stream of bytes representing contiguous 16 byte frames.
///  * `stream_id` - The starting stream ID.
pub fn decode_frames<H>(frames: &[u8], stream_id: u8, mut handler: H) -> Result<u8>
where
    H: FnMut(Result<Data>) -> Result<()>,
{
    let mut id = stream_id;
    let mut offset = 0;
    let mut iter = frames.chunks_exact(16);

    for frame in &mut iter {
        id = decode_frame_offset(frame.try_into().unwrap(), id, |r, o| handler(r, o + offset))?;
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

pub fn decode_frame_offset<H>(
    frame: &[u8; 16],
    stream_id: Option<u8>,
    mut handler: H,
    offset: usize,
) -> Result<Option<u8>>
where
    H: FnMut(Result<Data>) -> Result<()>,
{
    decode_frame(frame, stream_id, |r, o| handler(r, o + offset))
}
