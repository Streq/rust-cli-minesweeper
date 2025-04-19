use self::GameCommand::*;
use crate::args::MinesweeperArgs;
use crate::cell::Cell;
use crate::cell_content::CellContent;
use crate::cell_content::CellContent::Empty;
use crate::diff::Diff;
use crate::diff::Diff::{MultiCell, SingleCell};
use crate::diff::*;
use crate::flag::Flag::*;
use crate::minesweeper::GameState;
use crate::tile_visibility::TileVisibility;
use crate::tile_visibility::TileVisibility::{Hidden, Show};
use crate::util::{DIRS_8, Sign, i_xy, valid_neighbors, xy_i};
use crate::win_state::WinState::*;
use CellContent::Mine;
use std::collections::VecDeque;

pub type Cursor = (u16, u16);
#[derive(Copy, Clone, Debug)]
pub enum Action {
    Command(GameCommand),
    Restart(Option<RestartAction>),
    Debug(DebugAction),
}

#[derive(Copy, Clone, Debug)]
pub enum GameCommand {
    OpenCell(Cursor),
    FlagCell(Cursor),
    ClearFlag(Cursor),
    Surrender,
}

impl GameCommand {
    pub fn apply(&self, game: &mut GameState, args: &MinesweeperArgs) -> Option<Diff> {
        let branch = *self;
        let w = args.width;
        let h = args.height;

        let Ongoing = game.win_state else { return None };
        let cells = &mut game.cells;

        match branch {
            OpenCell(xy) => xy_i(xy, w, h).and_then(|i| {
                let cell = &mut cells[i];

                let Hidden(Clear | FlaggedMaybe) = cell.visibility else {
                    return None;
                };
                match cell.content {
                    Empty(0) => Some(MultiCell(expand_cell_diff_result(cells, w, h, i))),
                    Empty(_) => Some(cell.diff_result(i, Show)),
                    Mine => Some(cell.diff_result(i, Show)),
                }
            }),
            FlagCell(xy) => xy_i(xy, w, h).and_then(|i| {
                let cell = &mut cells[i];
                if let Hidden(flag) = cell.visibility {
                    Some(cell.diff_result(i, Hidden(flag.next())))
                } else {
                    None
                }
            }),
            ClearFlag(xy) => xy_i(xy, w, h).and_then(|i| {
                let cell = &mut cells[i];
                if let Show | Hidden(Clear) = cell.visibility {
                    None
                } else {
                    Some(cell.diff_result(i, Hidden(Clear)))
                }
            }),
            Surrender => {
                let mut ret = vec![];
                ret.reserve_exact(cells.len());
                for (i, cell) in game.cells.iter_mut().enumerate() {
                    if let Show = cell.visibility {
                        continue;
                    }
                    ret.push(cell.diff(i, Show))
                }
                Some(MultiCell(ret))
            }
        }
    }
}
fn expand_cell_diff_result(cells: &mut [Cell], w: u16, h: u16, idx: usize) -> Vec<SingleCellDiff> {
    let mut ret = vec![];

    let mut stack = VecDeque::<Cursor>::new();
    ret.push(cells[idx].diff(idx, Show));

    stack.push_back(i_xy(idx, w, h).unwrap());

    while let Some(c) = stack.pop_back() {
        for xy in valid_neighbors(&DIRS_8, c, w, h) {
            let Some(i) = xy_i(xy, w, h) else {
                unreachable!()
            };
            let cell = &mut cells[i];
            let Hidden(_) = cell.visibility else { continue };
            let Empty(n) = cell.content else {
                unreachable!()
            };
            ret.push(cell.diff(i, Show));

            if n == 0 {
                stack.push_back(xy);
            }
        }
    }
    ret
}

impl Cell {
    pub fn diff(&mut self, i: usize, visibility: TileVisibility) -> SingleCellDiff {
        let before = *self;
        self.visibility = visibility;
        SingleCellDiff {
            index: i,
            before,
            after: *self,
        }
    }

    pub fn diff_result(&mut self, i: usize, visibility: TileVisibility) -> Diff {
        SingleCell(self.diff(i, visibility))
    }
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
