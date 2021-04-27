//! Parses TWP frames into data values.

use super::types::{Data, Error, ErrorReason::*, Result};
use std::convert::TryInto;

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
