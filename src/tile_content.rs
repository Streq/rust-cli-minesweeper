#[derive(Copy, Clone, Debug)]
pub enum TileContent {
    Empty(u8),
    Mine,
}

impl Default for TileContent {
    fn default() -> Self {
        Self::Empty(0)
    }
}
