use std::collections::HashMap;
use stm::frame_builder::*;
use stm::frame_decoder::{Error::*, FrameConsumer, FrameDecoder};

struct NullConsumer;

impl FrameConsumer for NullConsumer {
    fn stream_byte(&mut self, _stream: Option<u8>, _data: u8) {}
}

// Feed a partial frame into the decoder:
#[test]
fn partial_frame() {
    let mut fd = FrameDecoder::new();
    let mut c = NullConsumer {};
    let frame = [0; 12];

    assert_eq!(fd.decode(&frame, &mut c, 0), Err(PartialFrame(12)));
}

// Put an invalid id within the stream:
#[test]
fn bad_path() {
    let mut fd = FrameDecoder::new();
    let mut c = NullConsumer {};
    let mut frame = [0; 32];

    frame[0] = 0x03;
    frame[2] = 0xFF;
    assert_eq!(fd.decode(&frame, &mut c, 0), Err(InvalidStreamId(2)));
}

type RecordMap = HashMap<Option<u8>, Vec<u8>>;

struct Record {
    streams: RecordMap,
    frame_count: usize,
}

impl Record {
    fn new() -> Record {
        Record {
            streams: RecordMap::new(),
            frame_count: 0,
        }
    }
}

impl FrameConsumer for Record {
    fn stream_byte(&mut self, stream: Option<u8>, data: u8) {
        self.streams
            .entry(stream)
            .and_modify(|e| e.push(data))
            .or_insert(vec![data]);
    }

    fn end_of_frame(&mut self) {
        self.frame_count += 1;
    }
}

// Two frames worth of data with an unknown stream id:
#[test]
fn unknown_id() {
    let mut fd = FrameDecoder::new();
    let mut c = Record::new();
    let frames = FrameBuilder::new(2).data_span(30, 3).build();

    assert_eq!(fd.decode(&frames, &mut c, 0), Ok(()));
    assert_eq!(c.frame_count, 2);

    let mut exp = RecordMap::new();
    exp.insert(None, vec![3; 30]);

    assert_eq!(c.streams, exp);
}

// Test an immediate stream change
#[test]
fn immediate_change() {
    let mut fd = FrameDecoder::new();
    let mut c = Record::new();
    let frames = FrameBuilder::new(1)
        .immediate_id(1)
        .data(1)
        .immediate_id(2)
        .data_span(12, 2)
        .build();

    assert_eq!(fd.decode(&frames, &mut c, 0), Ok(()));
    assert_eq!(c.frame_count, 1);

    let mut exp = RecordMap::new();
    exp.insert(Some(1), vec![1]);
    exp.insert(Some(2), vec![2; 12]);

    assert_eq!(c.streams, exp);
}

// Test a delayed stream change
#[test]
fn delayed_change() {
    let mut fd = FrameDecoder::new();
    let mut c = Record::new();
    let frames = FrameBuilder::new(2)
        .data_span(2, 1)
        .delayed_id(4)
        .data(1)
        .data_span(4, 4)
        .delayed_id(5)
        .data(4)
        .data_span(20, 5)
        .build();

    assert_eq!(fd.decode(&frames, &mut c, 0), Ok(()));
    assert_eq!(c.frame_count, 2);

    let mut exp = RecordMap::new();
    exp.insert(None, vec![1; 3]);
    exp.insert(Some(4), vec![4; 5]);
    exp.insert(Some(5), vec![5; 20]);

    assert_eq!(c.streams, exp);
}

// A delayed switch followed immediately by an immediate switch
#[test]
fn delayed_and_immediate_change() {
    let mut fd = FrameDecoder::new();
    let mut c = Record::new();
    let frames = FrameBuilder::new(1)
        .data_span(2, 1)
        .delayed_id(4)
        .data(1)
        .immediate_id(5)
        .data_span(10, 5)
        .build();

    assert_eq!(fd.decode(&frames, &mut c, 0), Ok(()));
    assert_eq!(c.frame_count, 1);

    let mut exp = RecordMap::new();
    exp.insert(None, vec![1; 3]);
    exp.insert(Some(5), vec![5; 10]);

    assert_eq!(c.streams, exp);
}

// Put a delayed id switch in an invalid position:
#[test]
fn invalid_aux_byte() {
    let mut fd = FrameDecoder::new();
    let mut c = Record::new();
    let mut frames = [0; 32];

    frames[14] = 0x03;
    frames[15] = 0x80;

    assert_eq!(fd.decode(&frames, &mut c, 0), Err(InvalidAuxByte(15)));
}
