use crate::stp;

pub struct Packet {
    pub offset: usize,        // Starting nibble offset.
    pub span: usize,          // Size of the packet in nibbles.
    pub details: stp::Packet, // Packet details.
}

#[derive(Debug)]
pub struct PacketFragment {
    pub offset: usize,               // Starting nibble offset.
    pub span: usize,                 // Size of the packet in nibbles.
    pub opcode: Option<stp::OpCode>, // Packet's opcode.
    pub payload_sz: u8,              // Current packet's payload size (nibbles).
    pub payload_index: u8,           // Current payload index (nibbles).
    pub payload: u64,                // Current packet's payload.
    pub has_timestamp: bool,         // Current packet has a timestamp.
    pub timestamp: u64,              // Current packet's timestamp.
}

impl PacketFragment {
    pub fn new() -> Self {
        return PacketFragment {
            offset: 0,
            span: 0,
            opcode: None,
            payload_sz: 0,
            payload_index: 0,
            payload: 0,
            has_timestamp: false,
            timestamp: 0,
        };
    }
}

#[derive(Debug, PartialEq)]
pub enum DecoderState {
    Unsynced, // The decoder is looking for a SYNC packet.
}

use self::DecoderState::*;

pub struct StpDecoder {
    offset: usize,          // Offset in nibbles.
    f_count: u8,            // Number of consecutive 0xF nibbles.
    state: DecoderState,    // The state of the decoder.
    packet: PacketFragment, // The current packet fragment.
}

impl StpDecoder {
    pub fn new() -> Self {
        return StpDecoder {
            offset: 0,
            f_count: 0,
            state: Unsynced,
            packet: PacketFragment::new(),
        };
    }
}
