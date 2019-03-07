pub struct FrameDecoder {
    stream: Option<u8>
}

impl FrameDecoder {
    pub fn new() -> Self {
        FrameDecoder { stream: None }
    }

    pub fn decode(&mut self) {
        self.stream = Some(42);
    }
}

#[cfg(test)]
mod tests {

    use super::FrameDecoder;

    #[test]
    fn it_works() {
        let mut decoder = FrameDecoder::new();
        decoder.decode();
        assert_eq!(decoder.stream, Some(42));
    }
}
