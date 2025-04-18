use self::GameCommand::*;
use crate::args::MinesweeperArgs;
use crate::cell::Cell;
use crate::cell_content::CellContent;
use crate::cell_content::CellContent::Empty;
use crate::diff::CellDiff::{MultiCell, SingleCell};
use crate::diff::Diff;
use crate::diff::*;
use crate::flag::Flag::*;
use crate::minesweeper::GameState;
use crate::tile_visibility::TileVisibility;
use crate::tile_visibility::TileVisibility::{Hidden, Show};
use crate::util::{DIRS_8, Sign, i_xy, valid_neighbors, xy_i};
use crate::win_state::WinState;
use crate::win_state::WinState::*;
use CellContent::Mine;
use std::collections::VecDeque;
use std::hint::unreachable_unchecked;

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

        match branch {
            OpenCell(xy) => xy_i(xy, w, h).and_then(|i| {
                let cells = &mut game.cells;

                let cell = &mut cells[i];

                let Hidden(Clear | FlaggedMaybe) = cell.visibility else {
                    return None;
                };
                let size = w as u32 * h as u32;
                let mines = args.mines;
                match cell.content {
                    Empty(0) => {
                        let v = expand_cell_diff_result(cells, w, h, i);
                        game.open_cells += v.len() as u32;
                        let win = if game.open_cells == size - mines {
                            Won
                        } else {
                            Ongoing
                        };
                        Some(Diff {
                            win_state: win,
                            diff: MultiCell(v),
                        })
                    }
                    Empty(_) => {
                        let diff = cell.diff_result(i, Show);
                        game.open_cells += 1;
                        let win_state = if game.open_cells == size - mines {
                            Won
                        } else {
                            Ongoing
                        };

                        Some(Diff { diff, win_state })
                    }
                    Mine => Some(Diff {
                        diff: cell.diff_result(i, Show),
                        win_state: Lost,
                    }),
                }
            }),
            FlagCell(xy) => xy_i(xy, w, h).and_then(|i| {
                let cell = &mut game.cells[i];
                if let Hidden(flag) = cell.visibility {
                    if let Hidden(Flagged) = cell.visibility {
                        game.flagged_cells -= 1;
                    }
                    if let Hidden(Clear) = cell.visibility {
                        game.flagged_cells += 1;
                    }
                    Some(Diff {
                        diff: cell.diff_result(i, Hidden(flag.next())),
                        win_state: Ongoing,
                    })
                } else {
                    None
                }
            }),
            ClearFlag(xy) => xy_i(xy, w, h).and_then(|i| {
                let cell = &mut game.cells[i];
                if let Show | Hidden(Clear) = cell.visibility {
                    None
                } else {
                    if let Hidden(Flagged) = cell.visibility {
                        game.flagged_cells -= 1;
                    }
                    Some(Diff {
                        diff: cell.diff_result(i, Hidden(Clear)),
                        win_state: Ongoing,
                    })
                }
            }),
            Surrender => {
                let mut ret = vec![];
                ret.reserve_exact(game.cells.len() - game.open_cells as usize);
                for (i, cell) in game.cells.iter_mut().enumerate() {
                    if let Show = cell.visibility {
                        continue;
                    }
                    ret.push(cell.diff(i, Show))
                }

                Some(Diff {
                    diff: MultiCell(ret),
                    win_state: Lost,
                })
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

    pub fn diff_result(&mut self, i: usize, visibility: TileVisibility) -> CellDiff {
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
