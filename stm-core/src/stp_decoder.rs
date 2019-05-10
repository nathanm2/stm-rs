pub struct StpPacket {
    pub nibble_offset: u64,
}

pub struct StpDecoder {
    nibble_offset: u64,
}

impl StpDecoder {
    /// Create a new StpDecoder.
    pub fn new() -> Self {
        StpDecoder { nibble_offset: 0 }
    }

    pub fn decode<F>(&mut self, bytes: &[u8], packet_handler: F)
    where
        F: Fn(StpPacket),
    {
        for byte in bytes {
            self.decode_nibble(byte & 0xF, packet_handler);
            self.decode_nibble(byte >> 4, packet_handler);
        }
    }

    fn decode_nibble<F>(&mut self, nibble: u8, packet_handler: F)
    where
        F: Fn(StpPacket),
    {

    }
}
