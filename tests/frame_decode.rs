use std::collections::HashMap;
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
    let frame = [0; 32];

    assert_eq!(fd.decode(&frame, &mut c, 0), Ok(()));
    assert_eq!(c.frame_count, 2);

    let mut exp = RecordMap::new();
    exp.insert(None, vec![0; 30]);

    assert_eq!(c.streams, exp);
}

fn set_stream_id(frames: &mut [u8], offset: usize, id: u8, immediate: bool) {
    assert!(offset % 2 == 0);
    let aux_offset = offset - (offset % 16) + 15;
    frames[offset] = id << 1 | 0x01;

    let mask = 0x01 << ((offset % 16) / 2);
    if immediate {
        frames[aux_offset] &= !mask;
    } else {
        frames[aux_offset] |= mask;
    }
}

fn set_stream_data(frames: &mut [u8], offset: usize, data: u8) {
    if offset % 2 == 0 {
        let aux_offset = offset - (offset % 16) + 15;
        frames[offset] = data & 0xFE;

        let mask = 0x01 << ((offset % 16) / 2);
        if data & 0x01 == 0 {
            frames[aux_offset] &= !mask;
        } else {
            frames[aux_offset] |= mask;
        }
    } else {
        frames[offset] = data;
    }
}

// Test an immediate stream change
#[test]
fn stream_change() {
    let mut fd = FrameDecoder::new();
    let mut c = Record::new();
    let mut frames = [0; 32];

    set_stream_data(&mut frames, 0, 1);
    set_stream_data(&mut frames, 1, 2);
    set_stream_id(&mut frames, 2, 3, true);
    set_stream_id(&mut frames, 6, 4, false);

    assert_eq!(fd.decode(&frames, &mut c, 0), Ok(()));
    assert_eq!(c.frame_count, 2);

    let mut exp = RecordMap::new();
    exp.insert(None, vec![1, 2]);
    exp.insert(Some(3), vec![0; 4]);
    exp.insert(Some(4), vec![0; 22]);

    assert_eq!(c.streams, exp);
}
