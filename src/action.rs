use crate::util::Unit;

#[derive(Copy, Clone, Debug)]
pub enum Action {
    OpenCell,
    FlagCell,
    ClearFlag,
    Surrender,
    Restart,
    IncrementMinesPercent(Unit),
    Resize(Unit, Unit),
    IncrementMines(Unit),
}
