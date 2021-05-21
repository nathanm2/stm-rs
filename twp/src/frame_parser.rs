//! Parses MIPI TWP frames (aka TPIU frames) into data values.

use super::error::{ErrorKind, ErrorKind::*, Result};
use std::convert::TryInto;
use std::option::Option;
use std::result;

/// Stream ID
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum StreamId {
    /// 'null' Stream ID.
    Null,
    /// A standard data Stream ID.
    Data(u8),
    /// A trigger Stream ID.
    Trigger,
}

use self::StreamId::*;

impl From<u8> for StreamId {
    fn from(id: u8) -> Self {
        match id {
            0 => Null,
            0x7D => Trigger,
            _ => Data(id),
        }
    }
}

impl Into<u8> for StreamId {
    fn into(self) -> u8 {
        match self {
            Null => 0,
            Trigger => 0x7D,
            Data(id) => id,
        }
    }
}

/// A byte of frame data associated with a particular Stream ID.
pub struct FrameByte {
    /// The byte value.
    pub data: u8,
    /// The byte's associated Stream ID.
    pub id: Option<StreamId>,
}

/// Parse a single TWP frame.
///
/// Parses `frame` and invokes `handler` for each byte of data or error.
///
/// `stream_id` is the initial stream ID to use when parsing the frame.  The last stream ID
/// encountered will be returned to the caller unless `handler` returns an error, in which case this
/// error will be returned instead.  Typically, the returned stream ID will be used as the
/// `stream_id` parameter of the next parse_frame invocation.
///
/// `handler` is given both a `twp::error::Result<FrameData>` and a `usize` offset as input
/// parameters.  Result will contain either a single byte of data associated with a particular
/// Stream ID *or* a `twp::error::ErrorKind` if a problem was encountered during parsing.  If
/// `handler` returns an `twp::error::Error`, frame parsing will stop immediately regardless of
/// whether an error was encountered.
///
/// All offsets passed to `handler` are relative to the frame.
///
/// Note that in the event of an error, `handler` is supplied an twp::error::ErrorKind but must
/// return a twp::error::Error which is a combination of ErrorKind and the offset.
///
pub fn parse_frame<H>(
    frame: &[u8; 16],
    stream_id: Option<StreamId>,
    mut handler: H,
) -> Result<Option<StreamId>>
where
    H: FnMut(result::Result<FrameByte, ErrorKind>, usize) -> Result<()>,
{
    let mut aux_byte = frame[15];

    // If byte 14 contains a frame id, it's aux bit should be zero per the spec:
    if aux_byte & 0x80 == 0x80 && frame[14] & 0x01 == 0x01 {
        handler(Err(InvalidAuxByte(aux_byte)), 15)?;
        aux_byte = aux_byte & 0x7F;
    }

    let mut cur_stream = stream_id;
    let mut next_stream = None;
    let mut delayed = false;

    for (i, byte) in frame[..15].iter().enumerate() {
        if i % 2 == 0 {
            // Even byte: ID change OR data.
            let aux_bit = (aux_byte >> (i / 2)) & 0x01;
            if byte & 0x01 == 1 {
                if *byte == 0xFF {
                    handler(Err(InvalidStreamId(0x7F)), i)?;
                }
                // Id Change.
                if aux_bit == 1 {
                    // Delayed ID Change.
                    next_stream = Some(StreamId::from(byte >> 1));
                    delayed = true;
                } else {
                    // Immediate ID change.
                    cur_stream = Some(StreamId::from(byte >> 1));
                }
            } else {
                handler(
                    Ok(FrameByte {
                        id: cur_stream,
                        data: byte | aux_bit,
                    }),
                    i,
                )?;
            }
        } else {
            // Odd byte: Data only.
            handler(
                Ok(FrameByte {
                    id: cur_stream,
                    data: *byte,
                }),
                i,
            )?;
            if delayed == true {
                cur_stream = next_stream;
                delayed = false;
            }
        }
    }

    Ok(cur_stream)
}

/// Parse a series of TWP frames.
///
/// Use `LayerParser` if the byte trace stream contains Frame Sync or Halfword Sync packets as
/// `parse_frames` assumes `frames` consists of a contiguous array of frames.
///
/// Parses a series of TWP frames and invokes `handler` for each byte of data or error encountered.
/// See [`parse_frame`] for more details about `handler` and `stream_id`.
///
/// All offsets passed to `handler` are relative to the start of the byte stream.
///
pub fn parse_frames<H>(
    frames: &[u8],
    stream_id: Option<StreamId>,
    mut handler: H,
) -> Result<Option<StreamId>>
where
    H: FnMut(result::Result<FrameByte, ErrorKind>, usize) -> Result<()>,
{
    let mut id = stream_id;
    let mut offset = 0;
    let mut iter = frames.chunks_exact(16);

    for frame in &mut iter {
        id = parse_frame(frame.try_into().unwrap(), id, |r, o| handler(r, o + offset))?;
        offset += 16;
    }

    let remainder = iter.remainder().len();
    if remainder > 0 {
        handler(Err(PartialFrame(remainder)), offset)?;
    }

    Ok(id)
}
