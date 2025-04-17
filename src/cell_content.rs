#[derive(Copy, Clone, Debug, PartialEq)]
pub enum CellContent {
    Empty(u8),
    Mine,
}

impl Default for CellContent {
    fn default() -> Self {
        Self::Empty(0)
    }
}
