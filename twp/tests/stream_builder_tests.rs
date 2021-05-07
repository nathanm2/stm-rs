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

// Padding tests:
#[test]
fn padding() -> Result<(), Error> {
    let mut out = Vec::with_capacity(32);
    Ok(())
}
#[test]
fn bad_stream_id() {
    let mut out = Vec::with_capacity(32);
    let mut builder = StreamBuilder::new(&mut out);
    let result = builder.id(StreamId::Data(0xFF));
    assert_eq!(result.err(), Some(Error::InvalidStreamId(0xFF, 0)));
}
