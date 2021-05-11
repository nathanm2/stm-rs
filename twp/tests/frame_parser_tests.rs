use std::collections::HashMap;
use std::result;
use twp::stream_builder::{self, StreamBuilder};
use twp::*;

type DataMap = HashMap<Option<StreamId>, Vec<u8>>;

// Parses a series of TWP frames and populates `data_map` and `errors`.
fn record_parse_frames(
    frames: &[u8],
    data_map: &mut DataMap,
    errors: Option<&mut Vec<Error>>,
) -> Result<Option<StreamId>> {
    let mut tmp_errors = errors;
    parse_frames(frames, None, |r, o| match r {
        Ok(FrameByte { data, id }) => {
            data_map
                .entry(id)
                .and_modify(|v| v.push(data))
                .or_insert(vec![data]);
            Ok(())
        }
        Err(kind) => {
            let e = Error {
                kind: kind,
                offset: o,
            };
            match tmp_errors.as_mut() {
                Some(v) => {
                    v.push(e);
                    Ok(())
                }
                None => Err(e),
            }
        }
    })
}

// Pass one complete frame and one partial frame to the parser:
#[test]
fn partial_frame() {
    let frames = [0; 30];
    let mut data_map = DataMap::new();
    let result = record_parse_frames(&frames, &mut data_map, None);

    assert_eq!(
        result,
        Err(Error {
            kind: ErrorKind::PartialFrame(14),
            offset: 16,
        })
    );

    let mut expected_data = DataMap::new();
    expected_data.insert(None, vec![0; 15]);
    assert_eq!(data_map, expected_data);
}

// Parse a frame with an invalid ID:
#[test]
fn bad_id() {
    let mut frames = [0; 32];

    frames[0] = 0x03;
    frames[16] = 0xFF;

    let mut data_map = DataMap::new();
    let result = record_parse_frames(&frames, &mut data_map, None);

    assert_eq!(
        result,
        Err(Error {
            offset: 16,
            kind: ErrorKind::InvalidStreamId(0x7F)
        })
    );

    let mut expected = HashMap::new();
    expected.insert(Some(StreamId::from(1)), vec![0; 14]);

    assert_eq!(data_map, expected);
}

// Parse a frame with an invalid ID: (continue on)
#[test]
fn bad_id_continue() {
    let mut frames = [0; 32];

    frames[0] = 0x03;
    frames[16] = 0xFF;

    let mut data_map = DataMap::new();
    let mut errors = Vec::new();
    let result = record_parse_frames(&frames, &mut data_map, Some(&mut errors));
    assert_eq!(result, Ok(Some(StreamId::from(127))));

    let mut expected = HashMap::new();
    expected.insert(Some(StreamId::from(1)), vec![0; 14]);
    expected.insert(Some(StreamId::from(0x7F)), vec![0; 14]);

    assert_eq!(data_map, expected);

    let expected_errors = vec![Error {
        offset: 16,
        kind: ErrorKind::InvalidStreamId(0x7F),
    }];
    assert_eq!(errors, expected_errors);
}

// Two frames of data with an unknown stream id:
#[test]
fn unknown_id() {
    let frames = [0; 32];

    let mut data_map = DataMap::new();
    let result = record_parse_frames(&frames, &mut data_map, None);
    assert_eq!(result, Ok(None));

    let mut expected = HashMap::new();
    expected.insert(None, vec![0; 30]);

    assert_eq!(data_map, expected);
}

// Test immediate and deferred stream change:
#[test]
fn id_change() -> result::Result<(), stream_builder::Error> {
    let mut frames = Vec::with_capacity(16);
    let _ = StreamBuilder::new(&mut frames)
        .id_data(StreamId::Data(1), &[1; 2])?
        .id_data(StreamId::Data(2), &[2, 2])?
        .finish();
    let mut data_map = DataMap::new();
    let result = record_parse_frames(&frames, &mut data_map, None);
    assert_eq!(result, Ok(Some(StreamId::Null)));

    let mut expected = HashMap::new();
    expected.insert(Some(StreamId::Data(1)), vec![1; 2]);
    expected.insert(Some(StreamId::Data(2)), vec![2; 2]);
    expected.insert(Some(StreamId::Null), vec![0; 8]);

    assert_eq!(data_map, expected);
    Ok(())
}

// Put a stream change at the end of a frame:
#[test]
fn id_change_end_of_frame() -> result::Result<(), stream_builder::Error> {
    let mut frames = Vec::with_capacity(16);
    let r = StreamBuilder::new(&mut frames)
        .id_data(StreamId::Data(1), &[1; 13])?
        .id_data(StreamId::Data(2), &[2; 15])?
        .finish();

    print!("{:?}\n", frames);

    assert_eq!(r, Ok(32));
    let mut data_map = DataMap::new();
    let result = record_parse_frames(&frames, &mut data_map, None);
    assert_eq!(result, Ok(Some(StreamId::Data(2))));

    let mut expected = HashMap::new();
    expected.insert(Some(StreamId::Data(1)), vec![1; 13]);
    expected.insert(Some(StreamId::Data(2)), vec![2; 15]);

    assert_eq!(data_map, expected);
    Ok(())
}
