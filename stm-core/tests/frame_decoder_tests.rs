use std::collections::HashMap;
use stm_core::frame_builder::*;
use stm_core::frame_decoder::{decode_frames, Error, ErrorReason::*};

trait DataMap {
    fn save(&mut self, id: Option<u8>, data: u8);
}

impl DataMap for HashMap<Option<u8>, Vec<u8>> {
    fn save(&mut self, id: Option<u8>, data: u8) {
        self.entry(id)
            .and_modify(|v| v.push(data))
            .or_insert(vec![data]);
    }
}

// Feed a partial frame into the decoder:
#[test]
fn partial_frame() {
    let frames = [0; 31];
    let mut map = HashMap::new();

    assert_eq!(
        decode_frames(&frames, None, |id, data| map.save(id, data), |e| Err(e)),
        Err(Error {
            offset: 16,
            reason: PartialFrame(15)
        })
    );

    let mut expected = HashMap::new();
    expected.insert(None, vec![0; 15]);

    assert_eq!(map, expected);
}

// Put an invalid id within the stream:
#[test]
fn bad_id() {
    let mut frames = [0; 32];
    let mut map = HashMap::new();

    frames[0] = 0x03;
    frames[16] = 0xFF;
    assert_eq!(
        decode_frames(&frames, None, |id, data| map.save(id, data), |e| Err(e)),
        Err(Error {
            offset: 16,
            reason: InvalidStreamId(0x7F)
        })
    );

    let mut expected = HashMap::new();
    expected.insert(Some(1), vec![0; 14]);

    assert_eq!(map, expected);
}

// Put an invalid id within the stream (continue on)
#[test]
fn bad_id_continue() {
    let mut frames = [0; 32];
    let mut map = HashMap::new();
    let mut errors = Vec::new();

    frames[0] = 0x03;
    frames[16] = 0xFF;
    assert_eq!(
        decode_frames(
            &frames,
            None,
            |id, data| map.save(id, data),
            |e| {
                errors.push(e);
                Ok(())
            }
        ),
        Ok(Some(127))
    );

    let mut expected = HashMap::new();
    expected.insert(Some(1), vec![0; 14]);
    expected.insert(Some(0x7F), vec![0; 14]);

    assert_eq!(map, expected);
}

// Two frames of data with an unknown stream id:
#[test]
fn unknown_id() {
    let frame = [0; 32];
    let mut map = HashMap::new();

    assert_eq!(
        decode_frames(&frame, None, |id, data| map.save(id, data), |e| Err(e)),
        Ok(None)
    );

    let mut expected = HashMap::new();
    expected.insert(None, vec![0; 30]);

    assert_eq!(map, expected);
}

// Test an immediate stream change
#[test]
fn immediate_change() {
    let frames = FrameBuilder::new(1)
        .immediate_id(1)
        .data(1)
        .immediate_id(2)
        .data_span(12, 2)
        .build();
    let mut map = HashMap::new();

    assert_eq!(
        decode_frames(&frames, None, |id, data| map.save(id, data), |e| Err(e)),
        Ok(Some(2))
    );

    let mut expected = HashMap::new();
    expected.insert(Some(1), vec![1]);
    expected.insert(Some(2), vec![2; 12]);

    assert_eq!(map, expected);
}

// Test a delayed stream change
#[test]
fn delayed_change() {
    let frames = FrameBuilder::new(2)
        .data_span(2, 1)
        .delayed_id(4)
        .data(1)
        .data_span(4, 4)
        .delayed_id(5)
        .data(4)
        .data_span(20, 5)
        .build();
    let mut map = HashMap::new();

    assert_eq!(
        decode_frames(&frames, None, |id, data| map.save(id, data), |e| Err(e)),
        Ok(Some(5))
    );

    let mut exp = HashMap::new();
    exp.insert(None, vec![1; 3]);
    exp.insert(Some(4), vec![4; 5]);
    exp.insert(Some(5), vec![5; 20]);

    assert_eq!(map, exp);
}

// A delayed switch followed immediately by an immediate switch
#[test]
fn delayed_and_immediate_change() {
    let frames = FrameBuilder::new(1)
        .data_span(2, 1)
        .delayed_id(4)
        .data(1)
        .immediate_id(5)
        .data_span(10, 5)
        .build();
    let mut map = HashMap::new();

    assert_eq!(
        decode_frames(&frames, None, |id, data| map.save(id, data), |e| Err(e)),
        Ok(Some(5))
    );

    let mut exp = HashMap::new();
    exp.insert(None, vec![1; 3]);
    exp.insert(Some(5), vec![5; 10]);

    assert_eq!(map, exp);
}

// Put a delayed id switch in an invalid position:
#[test]
fn invalid_aux_byte() {
    let mut frames = [0; 32];
    let mut map = HashMap::new();

    frames[30] = 0x03;
    frames[31] = 0x80;

    assert_eq!(
        decode_frames(&frames, Some(1), |id, data| map.save(id, data), |e| Err(e)),
        Err(Error {
            offset: 31,
            reason: InvalidAuxByte(0x80)
        })
    );

    let mut exp = HashMap::new();
    exp.insert(Some(1), vec![0; 15]);

    assert_eq!(map, exp);
}

// Put a delayed id switch in an invalid position:
#[test]
fn invalid_aux_byte_continue() {
    let mut frames = [0; 32];
    let mut map = HashMap::new();
    let mut errors = Vec::new();

    frames[30] = 0x05;
    frames[31] = 0x80;

    assert_eq!(
        decode_frames(
            &frames,
            Some(1),
            |id, data| map.save(id, data),
            |e| {
                errors.push(e);
                Ok(())
            }
        ),
        Ok(Some(2))
    );

    let mut exp = HashMap::new();
    exp.insert(Some(1), vec![0; 29]);

    assert_eq!(map, exp);
}
