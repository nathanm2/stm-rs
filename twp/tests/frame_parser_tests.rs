use std::collections::HashMap;
use twp::error::*;
use twp::frame_parser::*;

type DataMap = HashMap<Option<StreamId>, Vec<u8>>;

// Utility function that populates a map from FrameBytes and optionally records the errors
// encountered.
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
