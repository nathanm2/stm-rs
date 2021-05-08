use twp::stream_builder::*;

// Build a single frame:
#[test]
fn basic_frame() -> Result<(), Error> {
    let mut out = Vec::with_capacity(32);
    let len = StreamBuilder::new(&mut out)
        .id_data(StreamId::Data(2), &[3; 14])?
        .finish()?;
    let expected = [5, 3, 2, 3, 2, 3, 2, 3, 2, 3, 2, 3, 2, 3, 2, 0xFE];
    assert_eq!(out, expected);
    assert_eq!(16, out.len());
    assert_eq!(len, out.len());
    Ok(())
}

// ID appearing on the odd byte should result in a 'delayed' ID:
#[test]
fn delayed_id() -> Result<(), Error> {
    let mut out = Vec::with_capacity(32);
    let len = StreamBuilder::new(&mut out)
        .data(&[1])?
        .id_data(StreamId::Data(2), &[3; 13])?
        .finish()?;
    let expected = [5, 1, 2, 3, 2, 3, 2, 3, 2, 3, 2, 3, 2, 3, 2, 0xFF];
    assert_eq!(out, expected);
    assert_eq!(16, out.len());
    assert_eq!(len, out.len());
    Ok(())
}

// Padding test
// Build a series of frames:
#[test]
fn basic_frames() -> Result<(), Error> {
    let mut out = Vec::with_capacity(32);
    let len = StreamBuilder::new(&mut out)
        .id_data(StreamId::from(2), &[0, 1, 2, 3, 4, 5, 6, 7])?
        .id_data(StreamId::from(2), &[8, 9, 10, 11, 12, 13])?
        .id_data(StreamId::from(3), &[2, 2, 2])?
        .finish()?;
    let expected = [
        5, 0, 0, 2, 2, 4, 4, 6, 5, 7, 8, 9, 10, 11, 12, 0x1E, 7, 13, 2, 2, 1, 2, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0x5,
    ];
    assert_eq!(out, expected);
    assert_eq!(32, out.len());
    assert_eq!(len, out.len());
    Ok(())
}

// Redundant IDs:
#[test]
fn redundant_ids() -> Result<(), Error> {
    let mut out = Vec::with_capacity(32);
    let len = StreamBuilder::new(&mut out)
        .id(StreamId::Data(42))?
        .id(StreamId::Null)?
        .id(StreamId::Data(2))?
        .data(&[3; 14])?
        .finish()?;
    let expected = [5, 3, 2, 3, 2, 3, 2, 3, 2, 3, 2, 3, 2, 3, 2, 0xFE];
    assert_eq!(out, expected);
    assert_eq!(16, out.len());
    assert_eq!(len, out.len());
    Ok(())
}

// An empty data segment.
#[test]
fn zero_data() -> Result<(), Error> {
    let mut out = Vec::with_capacity(32);
    let len = StreamBuilder::new(&mut out)
        .id_data(StreamId::Data(2), &[])? // No-op
        .id_data(StreamId::Data(3), &[2, 2, 2])?
        .finish()?;
    let expected = [7, 2, 2, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x00];
    assert_eq!(out, expected);
    assert_eq!(16, out.len());
    assert_eq!(len, out.len());
    Ok(())
}

// No frames
#[test]
fn no_frames() -> Result<(), Error> {
    let mut out = Vec::with_capacity(32);
    let len = StreamBuilder::new(&mut out)
        .id_data(StreamId::Data(2), &[])? // No-op
        .id(StreamId::Data(4))?
        .data(&[])? // No-op
        .finish()?;
    assert_eq!(0, out.len());
    assert_eq!(len, out.len());
    Ok(())
}

#[test]
fn padding() -> Result<(), Error> {
    let mut out = Vec::with_capacity(32);
    let len = StreamBuilder::new(&mut out)
        .id_data(StreamId::Data(3), &[3; 13])?
        .pad_frame()? // Pad a 14 byte frame.
        .pad_frame()? // No-op
        .data(&[4; 3])? // Restores Stream ID to 3
        .finish()?; // Pad a 5 byte frame.
    let expected = [
        7, 3, 2, 3, 2, 3, 2, 3, 2, 3, 2, 3, 2, 3, 1, 0x7E, 7, 4, 4, 4, 1, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0x00,
    ];
    assert_eq!(out, expected);
    assert_eq!(32, out.len());
    assert_eq!(len, out.len());
    Ok(())
}

#[test]
fn padding_null() -> Result<(), Error> {
    let mut out = Vec::with_capacity(32);
    let len = StreamBuilder::new(&mut out)
        .id_data(StreamId::Null, &[3; 13])?
        .pad_frame()? // Pad a 14 byte frame.
        .pad_frame()? // No-op
        .data(&[4; 4])? // Null Stream ID continues to be used.
        .finish()?; // Pad a 4 byte frame.
    let expected = [
        1, 3, 2, 3, 2, 3, 2, 3, 2, 3, 2, 3, 2, 3, 1, 0x7E, 4, 4, 4, 4, 1, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0x00,
    ];
    assert_eq!(out, expected);
    assert_eq!(32, out.len());
    assert_eq!(len, out.len());
    Ok(())
}

#[test]
fn padding_id() -> Result<(), Error> {
    let mut out = Vec::with_capacity(32);
    let len = StreamBuilder::new(&mut out)
        .id_data(StreamId::Data(3), &[3; 13])?
        .id(StreamId::Data(4))? // ID change but no data.
        .pad_frame()? // Pad a 14 byte frame.
        .data(&[4; 5])? // Changes to ID 4, with four bytes of data.
        .finish()?; // Pad a 5 byte frame.
    let expected = [
        7, 3, 2, 3, 2, 3, 2, 3, 2, 3, 2, 3, 2, 3, 1, 0x7E, 9, 4, 4, 4, 4, 4, 1, 0, 0, 0, 0, 0, 0,
        0, 0, 0x00,
    ];
    assert_eq!(out, expected);
    assert_eq!(32, out.len());
    assert_eq!(len, out.len());
    Ok(())
}

#[test]
fn bad_stream_id() {
    let mut out = Vec::with_capacity(32);
    let mut builder = StreamBuilder::new(&mut out);
    let result = builder.id(StreamId::Data(0xFF));
    assert_eq!(result.err(), Some(Error::InvalidStreamId(0xFF, 0)));
}
