use crate::cell::Cell;
use crate::win_state::WinState;

#[derive(Debug)]
pub struct Diff {
    pub win_state: WinState,
    pub diff: CellDiff,
}
#[derive(Debug)]
pub enum CellDiff {
    SingleCell(SingleCellDiff),
    MultiCell(Vec<SingleCellDiff>),
}

#[derive(Debug, Default, Copy, Clone)]
pub struct SingleCellDiff {
    pub index: usize,
    pub before: Cell,
    pub after: Cell,
}
