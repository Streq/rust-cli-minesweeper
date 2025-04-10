use crate::util::Unit;

#[derive(Copy, Clone, Debug)]
pub enum Action {
    ShowTile,
    FlagTile,
    ClearFlag,
    Surrender,
    Restart,
    Next,
    Previous,
    Resize(Unit, Unit),
    IncrementMines(Unit),
}
