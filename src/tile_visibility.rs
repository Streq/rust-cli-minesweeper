use crate::flag::Flag;

#[derive(Copy, Clone, Debug)]
pub enum TileVisibility {
    Hidden(Flag),
    Show,
}

impl Default for TileVisibility {
    fn default() -> Self {
        Self::Hidden(Flag::None)
    }
}
