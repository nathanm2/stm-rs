
pub struct StpParser {
    synced: bool,
}

impl StpParser {
    /// Create a new StpParser.
    pub fn new() -> Self {
        StpParser {
            synced = false,
        }
    }
}
