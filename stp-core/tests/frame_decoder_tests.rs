use std::collections::HashMap;
use std::result;
use stp_core::frame_builder::*;
use stp_core::frame_decoder::{decode_frames, Error, ErrorReason::*, FrameDecoder};

struct Recorder {
    data: HashMap<Option<u8>, Vec<u8>>,
    errors: Option<Vec<Error>>,
}

impl Recorder {
    fn new(continue_on_error: bool) -> Recorder {
        Recorder {
            data: HashMap::new(),
            errors: if continue_on_error {
                Some(Vec::new())
            } else {
                None
            },
        }
    }

    fn record(&mut self, r: result::Result<(Option<u8>, u8), Error>) -> result::Result<(), Error> {
        match r {
            Ok((id, data)) => {
                self.data
                    .entry(id)
                    .and_modify(|v| v.push(data))
                    .or_insert(vec![data]);
                Ok(())
            }
            Err(e) => match &mut self.errors {
                Some(v) => {
                    v.push(e);
                    Ok(())
                }
                None => Err(e),
            },
        }
    }
}

// Feed a partial frame into the decoder:
#[test]
fn partial_frame() {
    let frames = [0; 31];
    let mut recorder = Recorder::new(false);

    assert_eq!(
        decode_frames(&frames, None, |d| recorder.record(d)),
        Err(Error {
            offset: 16,
            reason: PartialFrame(15)
        })
    );

    let mut expected = HashMap::new();
    expected.insert(None, vec![0; 15]);

    assert_eq!(recorder.data, expected);
}

// Put an invalid id within the stream:
#[test]
fn bad_id() {
    let mut frames = [0; 32];
    let mut recorder = Recorder::new(false);

    frames[0] = 0x03;
    frames[16] = 0xFF;
    assert_eq!(
        decode_frames(&frames, None, |d| recorder.record(d)),
        Err(Error {
            offset: 16,
            reason: InvalidStreamId(0x7F)
        })
    );

    let mut expected = HashMap::new();
    expected.insert(Some(1), vec![0; 14]);

    assert_eq!(recorder.data, expected);
}

// Put an invalid id within the stream (continue on)
#[test]
fn bad_id_continue() {
    let mut frames = [0; 32];
    let mut recorder = Recorder::new(true);

    frames[0] = 0x03;
    frames[16] = 0xFF;
    assert_eq!(
        decode_frames(&frames, None, |d| recorder.record(d)),
        Ok(Some(127))
    );

    let mut expected = HashMap::new();
    expected.insert(Some(1), vec![0; 14]);
    expected.insert(Some(0x7F), vec![0; 14]);

    assert_eq!(recorder.data, expected);

    let errors = vec![Error {
        offset: 16,
        reason: InvalidStreamId(0x7F),
    }];
    assert_eq!(recorder.errors.unwrap(), errors);
}

// Two frames of data with an unknown stream id:
#[test]
fn unknown_id() {
    let frames = [0; 32];
    let mut recorder = Recorder::new(false);

    assert_eq!(
        decode_frames(&frames, None, |d| recorder.record(d)),
        Ok(None)
    );

    let mut expected = HashMap::new();
    expected.insert(None, vec![0; 30]);

    assert_eq!(recorder.data, expected);
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
    let mut recorder = Recorder::new(false);

    assert_eq!(
        decode_frames(&frames, None, |d| recorder.record(d)),
        Ok(Some(2))
    );

    let mut expected = HashMap::new();
    expected.insert(Some(1), vec![1]);
    expected.insert(Some(2), vec![2; 12]);

    assert_eq!(recorder.data, expected);
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
    let mut recorder = Recorder::new(false);

    assert_eq!(
        decode_frames(&frames, None, |d| recorder.record(d)),
        Ok(Some(5))
    );

    let mut exp = HashMap::new();
    exp.insert(None, vec![1; 3]);
    exp.insert(Some(4), vec![4; 5]);
    exp.insert(Some(5), vec![5; 20]);

    assert_eq!(recorder.data, exp);
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
    let mut recorder = Recorder::new(false);

    assert_eq!(
        decode_frames(&frames, None, |d| recorder.record(d)),
        Ok(Some(5))
    );

    let mut exp = HashMap::new();
    exp.insert(None, vec![1; 3]);
    exp.insert(Some(5), vec![5; 10]);

    assert_eq!(recorder.data, exp);
}

// Put a delayed id switch in an invalid position:
#[test]
fn invalid_aux_byte() {
    let mut frames = [0; 32];
    let mut recorder = Recorder::new(false);

    frames[30] = 0x03;
    frames[31] = 0x80;

    assert_eq!(
        decode_frames(&frames, Some(1), |d| recorder.record(d)),
        Err(Error {
            offset: 31,
            reason: InvalidAuxByte(0x80)
        })
    );

    let mut exp = HashMap::new();
    exp.insert(Some(1), vec![0; 15]);

    assert_eq!(recorder.data, exp);
}

// Put a delayed id switch in an invalid position:
#[test]
fn invalid_aux_byte_continue() {
    let mut frames = [0; 32];
    let mut recorder = Recorder::new(true);

    frames[30] = 0x05;
    frames[31] = 0x80;

    assert_eq!(
        decode_frames(&frames, Some(1), |d| recorder.record(d)),
        Ok(Some(2))
    );

    let mut exp = HashMap::new();
    exp.insert(Some(1), vec![0; 29]);

    assert_eq!(recorder.data, exp);
}

// Stop when stream id 2 is seen:
#[test]
fn stop_test() {
    let frames = FrameBuilder::new(1)
        .id(1)
        .data_span(10, 100)
        .id(2)
        .data_span(10, 200)
        .build();
    let mut recorder = Recorder::new(false);

    assert_eq!(
        decode_frames(&frames, None, |d| {
            match d {
                Ok((Some(id), _)) if id == 2 => Err(Error {
                    offset: 0,
                    reason: Stop,
                }),
                x => recorder.record(x),
            }
        }),
        Err(Error {
            offset: 0,
            reason: Stop
        })
    );

    let mut exp = HashMap::new();
    exp.insert(Some(1), vec![100; 10]);

    assert_eq!(recorder.data, exp);
}

// Two unsynchronized frames:
#[test]
fn unsynced_frames() {
    let frames = FrameBuilder::new(2).id(1).data_span(14 + 15, 1).build();
    let mut decoder = FrameDecoder::new(false, None);
    let mut recorder = Recorder::new(true);

    assert_eq!(decoder.decode(&frames, |d| recorder.record(d)), Ok(()));

    assert_eq!(recorder.data.is_empty(), true);
}

// An FSYNC followed by two frames worth of data.
#[test]
fn synced_frames() {
    let mut frames = FrameBuilder::new(2).id(1).data_span(14 + 15, 1).build();
    let mut decoder = FrameDecoder::new(false, None);
    let mut recorder = Recorder::new(true);

    insert_fsync(&mut frames, 0).unwrap();

    assert_eq!(decoder.decode(&frames, |d| recorder.record(d)), Ok(()));

    let mut exp = HashMap::new();
    exp.insert(Some(1), vec![1; 14 + 15]);

    assert_eq!(recorder.data, exp);
}
