use self::GameCommand::*;
use crate::args::MinesweeperArgs;
use crate::cell::Cell;
use crate::cell_content::CellContent;
use crate::cell_content::CellContent::Empty;
use crate::diff::Diff::{MultiCell, SingleCell};
use crate::diff::*;
use crate::flag::Flag::*;
use crate::minesweeper::{GameState, Minesweeper};
use crate::tile_visibility::TileVisibility;
use crate::tile_visibility::TileVisibility::{Hidden, Show};
use crate::util::{DIRS_8, Sign, i_xy, xy_i};
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
    pub fn apply(&self, game: &GameState, args: &MinesweeperArgs) -> Option<Diff> {
        let branch = (*self, game.win_state);
        let w = args.width;
        let h = args.height;

        match branch {
            (OpenCell(xy), Untouched) => init_random_game(xy, args),
            (OpenCell(xy), Ongoing) => xy_i(xy, w, h).and_then(|i| {
                let cells = &game.cells;

                let cell = cells[i];

                let Hidden(Clear | FlaggedMaybe) = cell.visibility else {
                    return None;
                };

                match cell.content {
                    Empty(0) => expand_cell_diff_result(cells, w, h, i),
                    Empty(_) | Mine => cell.diff_result(i, Show),
                }
            }),

            (FlagCell(xy), Ongoing | Untouched) => xy_i(xy, w, h).and_then(|i| {
                let cell = game.cells[i];
                if let Hidden(flag) = cell.visibility {
                    cell.diff_result(i, Hidden(flag.next()))
                } else {
                    None
                }
            }),
            (ClearFlag(xy), Ongoing | Untouched) => xy_i(xy, w, h).and_then(|i| {
                let cell = game.cells[i];
                if let Show | Hidden(Clear) = cell.visibility {
                    None
                } else {
                    cell.diff_result(i, Hidden(Clear))
                }
            }),
            (Surrender, Ongoing) => {
                let mut ret = vec![];
                ret.reserve_exact(game.cells.len() - game.open_cells as usize);
                for (i, cell) in game.cells.iter().enumerate() {
                    if let Show = cell.visibility {
                        continue;
                    }
                    ret.push(cell.diff(i, Show))
                }
                Some(MultiCell(ret))
            }
            (_, Untouched | Won | Lost) => None,
        }
    }
}

fn init_random_game(cursor: Cursor, args: &MinesweeperArgs) -> Option<Diff> {
    let w = args.width;
    let h = args.height;
    let m = args.mines;
    if m == 0 {
        return None;
    };
    let Some(i) = xy_i(cursor, w, h) else {
        return None;
    };
    let ret = vec![Default::default(); m as usize];

    Some(MultiCell(ret))
}

fn expand_cell_diff_result(cells: &[Cell], w: u16, h: u16, idx: usize) -> Option<Diff> {
    let mut stack = VecDeque::<Cursor>::new();
    let mut ret = vec![];
    ret.push(cells[idx].diff(idx, Show));

    stack.push_back(i_xy(idx, w, h).unwrap());

    while let Some(cursor) = stack.pop_back() {
        for neighbor in neighbors(cursor, w, h) {
            let Some(xy) = neighbor else { continue };
            let Some(i) = xy_i(xy, w, h) else { continue };

            let cell = &cells[i];

            let Empty(n) = cell.content else {
                unreachable!()
            };
            ret.push(cell.diff(i, Show));

            if n == 0 {
                stack.push_back(xy);
            }
        }
    }

    Some(MultiCell(ret))
}

fn neighbors((x, y): Cursor, w: u16, h: u16) -> [Option<Cursor>; 8] {
    DIRS_8
        .map(|(dx, dy)| {
            (
                x.overflowing_add_signed(dx as i16),
                y.overflowing_add_signed(dy as i16),
            )
        })
        .map(
            |((x, ox), (y, oy))| {
                if oy || ox { None } else { Some((x, y)) }
            },
        )
}

impl Cell {
    pub fn diff(&self, i: usize, visibility: TileVisibility) -> SingleCellDiff {
        SingleCellDiff {
            index: i,
            before: *self,
            after: self.with_visibility(visibility),
        }
    }

    pub fn diff_result(&self, i: usize, visibility: TileVisibility) -> Option<Diff> {
        Some(SingleCell(self.diff(i, visibility)))
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
