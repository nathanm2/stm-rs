use std::collections::HashMap;
use stm::frame_builder::*;
use stm::frame_decoder::FrameDecoderError::*;
use stm::frame_decoder::{FrameConsumer, FrameDecoder};

struct NullConsumer;

impl FrameConsumer for NullConsumer {
    fn stream_byte(&mut self, _stream: Option<u8>, _data: u8) {}
}

#[test]
fn partial_frame() {
    let mut fd = FrameDecoder::new();
    let mut c = NullConsumer {};
    let frame = [0; 12];

    assert_eq!(fd.decode(&frame, &mut c, 0), Err(PartialFrame(12)));
}

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

// Two frames worth of data with an unknown stream destination:
#[test]
fn unknown_stream() {
    let mut fd = FrameDecoder::new();
    let mut c = Record::new();
    let frame = FrameBuilder::new(2).build();

    assert_eq!(fd.decode(&frame, &mut c, 0), Ok(()));
    assert_eq!(c.frame_count, 2);

    let mut exp = RecordMap::new();
    exp.insert(None, vec![0; 30]);

    assert_eq!(c.streams, exp);
}

// Test an immediate stream change
#[test]
fn immediate_change() {
    let mut fd = FrameDecoder::new();
    let mut c = Record::new();
    let frames = FrameBuilder::new(1)
        .data(1)
        .data(2)
        .immediate_id(3)
        .data_span(5, || 3)
        .immediate_id(4)
        .data_span(6, || 4)
        .build();

    assert_eq!(fd.decode(&frames, &mut c, 0), Ok(()));
    assert_eq!(c.frame_count, 1);

    let mut exp = RecordMap::new();
    exp.insert(None, vec![1, 2]);
    exp.insert(Some(3), vec![3; 5]);
    exp.insert(Some(4), vec![4; 6]);

    assert_eq!(c.streams, exp);
}

// Test a delayed stream change
#[test]
fn delayed_change() {
    let mut fd = FrameDecoder::new();
    let mut c = Record::new();
    let frames = FrameBuilder::new(1)
        .data(1)
        .data(2)
        .delayed_id(3)
        .data_span(5, || 7)
        .delayed_id(4)
        .data_span(6, || 7)
        .build();

    assert_eq!(fd.decode(&frames, &mut c, 0), Ok(()));
    assert_eq!(c.frame_count, 1);

    let mut exp = RecordMap::new();
    exp.insert(None, vec![1, 2, 7]);
    exp.insert(Some(3), vec![7; 5]);
    exp.insert(Some(4), vec![7; 5]);

    assert_eq!(c.streams, exp);
}
