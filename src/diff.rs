use crate::cell::Cell;
#[derive(Debug, Clone)]
pub enum Diff {
    SingleCell(SingleCellDiff),
    MultiCell(Vec<SingleCellDiff>),
}

#[derive(Debug, Default, Copy, Clone)]
pub struct SingleCellDiff {
    pub index: usize,
    pub before: Cell,
    pub after: Cell,
}
