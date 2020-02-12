use std::collections::HashMap;
use stm_core::frame_decoder_new::{decode_frames, Error, ErrorReason::*};

#[derive(Debug, PartialEq)]
struct DataMap {
    map: HashMap<Option<u8>, Vec<u8>>,
}

impl DataMap {
    fn new() -> DataMap {
        DataMap {
            map: HashMap::new(),
        }
    }

    fn save(&mut self, id: Option<u8>, data: u8) {
        self.map
            .entry(id)
            .and_modify(|v| v.push(data))
            .or_insert(vec![data]);
    }
}

// Empty frame
#[test]
fn empty_frame() {
    let frame = [0; 16];
    let mut map = DataMap::new();

    assert_eq!(
        decode_frames(&frame, Some(1), |id, data| map.save(id, data), |e| Err(e)),
        Ok(Some(1))
    );

    let mut expected = DataMap::new();
    expected.map.insert(Some(1), vec![0; 15]);

    assert_eq!(map, expected);
}
