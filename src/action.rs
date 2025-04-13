use crate::util::Sign;

pub type Cursor = (u16, u16);
#[derive(Copy, Clone, Debug)]
pub enum Action {
    Command(GameAction),
    Restart(Option<RestartAction>),
    Debug(DebugAction),
}

#[derive(Copy, Clone, Debug)]
pub enum GameAction {
    OpenCell(Cursor),
    FlagCell(Cursor),
    ClearFlag(Cursor),
    Surrender,
}

#[derive(Copy, Clone, Debug)]
pub enum RestartAction {
    ResizeH(Sign),
    ResizeV(Sign),
    IncrementMinesPercent(Sign),
    IncrementMines(Sign),
}

#[derive(Copy, Clone, Debug)]
pub enum DebugAction {
    Undo,
    Redo,
}
