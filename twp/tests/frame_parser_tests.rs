use std::collections::HashMap;
use twp::error::*;
use twp::frame_parser::*;

type DataMap = HashMap<Option<StreamId>, Vec<u8>>;

fn record_parse_frame(
    frames: &[u8],
    data_map: &mut DataMap,
    mut errors: Option<&mut Vec<Error>>,
) -> Result<Option<StreamId>> {
    parse_frames(&frames, None, |r, o| match r {
        Ok(FrameByte { data, id }) => {
            data_map
                .entry(id.into())
                .and_modify(|v| v.push(data))
                .or_insert(vec![data]);
            Ok(())
        }
        Err(kind) => {
            let e = Error {
                kind: kind,
                offset: o,
            };
            match &mut errors {
                Some(v) => {
                    v.push(e);
                    Ok(())
                }
                None => Err(e),
            }
        }
    })
}

// Feed a partial frame into the decoder:
#[test]
fn partial_frame() {
    let frames = [0; 31];
    let mut data_map = DataMap::new();
    let result = record_parse_frame(&frames, &mut data_map, None);
    let mut expected_data = DataMap::new();

    assert_eq!(
        result,
        Err(Error {
            kind: ErrorKind::PartialFrame(15),
            offset: 16,
        })
    );

    expected_data.insert(None, vec![0; 15]);
    assert_eq!(data_map, expected_data);
}
