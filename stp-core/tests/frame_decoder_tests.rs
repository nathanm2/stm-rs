use std::collections::HashMap;
use std::result;
use stp_core::frame_builder::*;
use stp_core::frame_decoder::{decode_frames, Data, Error, ErrorReason::*, FrameDecoder, FSYNC};

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

    fn record(&mut self, r: result::Result<Data, Error>) -> result::Result<(), Error> {
        match r {
            Ok(d) => {
                self.data
                    .entry(d.id)
                    .and_modify(|v| v.push(d.data))
                    .or_insert(vec![d.data]);
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
        .data_span(16, 100)
        .id(2)
        .data_span(10, 200)
        .build();
    let mut recorder = Recorder::new(false);

    assert_eq!(
        decode_frames(&frames, None, |d| {
            match d {
                Ok(data) if data.id == Some(2) => Err(Error {
                    offset: data.offset,
                    reason: Stop,
                }),
                x => recorder.record(x),
            }
        }),
        Err(Error {
            offset: 19,
            reason: Stop
        })
    );

    let mut exp = HashMap::new();
    exp.insert(Some(1), vec![100; 16]);

    assert_eq!(recorder.data, exp);
}

// Two unsynchronized frames:
#[test]
fn unsynced_frames() {
    let frames = FrameBuilder::new(2).id(1).data_span(14 + 15, 1).build();
    let mut decoder = FrameDecoder::new(false, None);
    let mut recorder = Recorder::new(false);

    assert_eq!(decoder.decode(&frames, |d| recorder.record(d)), Ok(()));
    assert_eq!(decoder.finish(|d| recorder.record(d)), Ok(()));

    assert_eq!(recorder.data.is_empty(), true);
}

// An FSYNC followed by two frames worth of data.
#[test]
fn synced_frames() {
    let mut decoder = FrameDecoder::new(false, None);
    let mut recorder = OffsetRecorder::new(false);
    let mut frames = FSYNC.to_vec();
    frames.extend(FrameBuilder::new(2).id(1).data_span(14 + 15, 1).build());

    assert_eq!(decoder.decode(&frames, |d| recorder.record(d)), Ok(()));
    assert_eq!(decoder.finish(|d| recorder.record(d)), Ok(()));

    let mut exp = HashMap::new();
    exp.insert(Some(1), vec![1; 14 + 15]);
    assert_eq!(recorder.r.data, exp);

    let mut exp_offsets = HashMap::new();
    exp_offsets.insert(Some(1), (5..5 + 14).chain(20..20 + 15).collect());
    assert_eq!(recorder.offsets, exp_offsets);
}

// An ignored frame + FSYNC + decoded frame.
#[test]
fn ignored_frame() {
    let mut decoder = FrameDecoder::new(false, None);
    let mut recorder = OffsetRecorder::new(false);
    let mut frames = FrameBuilder::new(1).id(1).data_span(14, 1).build();

    frames.extend_from_slice(&FSYNC);
    frames.extend(FrameBuilder::new(1).id(2).data_span(14, 2).build());

    assert_eq!(decoder.decode(&frames, |d| recorder.record(d)), Ok(()));
    assert_eq!(decoder.finish(|d| recorder.record(d)), Ok(()));

    let mut exp = HashMap::new();
    exp.insert(Some(2), vec![2; 14]);
    assert_eq!(recorder.r.data, exp);

    let mut exp_offsets = HashMap::new();
    exp_offsets.insert(Some(2), (21..21 + 14).collect());
    assert_eq!(recorder.offsets, exp_offsets);
}

// Decode in 16 byte chunks and split the FSYNC across a chunk boundary.
#[test]
fn split_fsync() {
    let mut decoder = FrameDecoder::new(false, None);
    let mut recorder = OffsetRecorder::new(false);
    let mut frames = vec![0; 14];

    frames.extend_from_slice(&FSYNC);
    frames.extend(FrameBuilder::new(1).id(2).data_span(14, 2).build());

    assert_eq!(frames[14], 0xFF);
    assert_eq!(frames[15], 0xFF);
    assert_eq!(frames[16], 0xFF);
    assert_eq!(frames[17], 0x7F);

    for frame in frames.chunks(16) {
        assert_eq!(decoder.decode(frame, |d| recorder.record(d)), Ok(()));
    }
    assert_eq!(decoder.finish(|d| recorder.record(d)), Ok(()));

    let mut exp = HashMap::new();
    exp.insert(Some(2), vec![2; 14]);
    assert_eq!(recorder.r.data, exp);

    let mut exp_offsets = HashMap::new();
    exp_offsets.insert(Some(2), (19..19 + 14).collect());
    assert_eq!(recorder.offsets, exp_offsets);
}

// Sync'd from the outset:
#[test]
fn initially_synced() {
    let mut decoder = FrameDecoder::new(true, None);
    let mut recorder = OffsetRecorder::new(false);
    let frames = FrameBuilder::new(2).id(2).data_span(14 + 15, 2).build();

    assert_eq!(decoder.decode(&frames, |d| recorder.record(d)), Ok(()));
    assert_eq!(decoder.finish(|d| recorder.record(d)), Ok(()));

    let mut exp = HashMap::new();
    exp.insert(Some(2), vec![2; 14 + 15]);
    assert_eq!(recorder.r.data, exp);

    let mut exp_offsets = HashMap::new();
    exp_offsets.insert(Some(2), (1..15).chain(16..16 + 15).collect());
    assert_eq!(recorder.offsets, exp_offsets);
}

// Truncate a frame with an FSYNC.
#[test]
fn truncated_frame() {
    let mut decoder = FrameDecoder::new(true, None); // Sync'd from the outset.
    let mut recorder = Recorder::new(true);
    let mut frames = FrameBuilder::new(2)
        .id(1)
        .data_span(14, 1)
        .id(2)
        .data_span(14, 2)
        .build();

    frames[12] = FSYNC[0];
    frames[13] = FSYNC[1];
    frames[14] = FSYNC[2];
    frames[15] = FSYNC[3];

    assert_eq!(decoder.decode(&frames, |d| recorder.record(d)), Ok(()));
    assert_eq!(decoder.finish(|d| recorder.record(d)), Ok(()));

    let mut exp = HashMap::new();
    exp.insert(Some(2), vec![2; 14]);

    assert_eq!(recorder.data, exp);

    let errors = vec![Error {
        offset: 0,
        reason: PartialFrame(11),
    }];

    assert_eq!(recorder.errors.unwrap(), errors);
}

// Truncate an FSYNC:
#[test]
fn truncated_fsync() {
    let mut decoder = FrameDecoder::new(true, Some(4)); // Sync'd from the outset.
    let mut recorder = Recorder::new(false);
    let mut frames = FrameBuilder::new(2)
        .data(0)
        .id(1)
        .data_span(13 + 15, 1)
        .build();

    assert_eq!(frames[15], 0xFF);
    assert_eq!(frames[31], 0xFF);

    frames[30] = 0xFF;

    assert_eq!(decoder.decode(&frames, |d| recorder.record(d)), Ok(()));
    assert_eq!(decoder.finish(|d| recorder.record(d)), Ok(()));

    let mut exp = HashMap::new();
    exp.insert(Some(4), vec![0]);
    exp.insert(Some(1), vec![1; 13]);

    assert_eq!(recorder.data, exp);
}

// An AUX byte containing 0xFF, followed by an FSYNC:
#[test]
fn aux_ff() {
    let mut decoder = FrameDecoder::new(true, None);
    let mut recorder = OffsetRecorder::new(false);
    let mut frames = FrameBuilder::new(3).data(2).id(2).data_span(13, 1).build();

    frames.extend_from_slice(&FSYNC);
    frames.extend(FrameBuilder::new(1).id(4).data_span(14, 4).build());

    assert_eq!(frames[15], 0xFF);
    assert_eq!(frames[16], 0xFF);
    assert_eq!(frames[17], 0xFF);
    assert_eq!(frames[18], 0xFF);
    assert_eq!(frames[19], 0x7F);

    assert_eq!(decoder.decode(&frames, |d| recorder.record(d)), Ok(()));
    assert_eq!(decoder.finish(|d| recorder.record(d)), Ok(()));

    let mut exp = HashMap::new();
    exp.insert(Some(2), vec![1; 14]);
    exp.insert(Some(4), vec![4; 14]);
    assert_eq!(recorder.r.data, exp);

    let mut exp_offsets = HashMap::new();
    exp_offsets.insert(Some(2), (1..1 + 14).collect());
    exp_offsets.insert(Some(4), (21..21 + 14).collect());
    assert_eq!(recorder.offsets, exp_offsets);
}

struct OffsetRecorder {
    r: Recorder,
    offsets: HashMap<Option<u8>, Vec<usize>>,
}

impl OffsetRecorder {
    fn new(continue_on_error: bool) -> OffsetRecorder {
        OffsetRecorder {
            r: Recorder::new(continue_on_error),
            offsets: HashMap::new(),
        }
    }

    fn record(&mut self, r: result::Result<Data, Error>) -> result::Result<(), Error> {
        if let Ok(ref d) = r {
            self.offsets
                .entry(d.id)
                .and_modify(|v| v.push(d.offset))
                .or_insert(vec![d.offset]);
        };
        self.r.record(r)
    }
}
