use crate::action::Action::*;
use crate::action::Cursor;
use crate::action::DebugAction::*;
use crate::action::GameCommand::*;
use crate::action::RestartAction::*;
use crate::args::MinesweeperArgs;
use crate::cell::Cell;
use crate::cell_content::CellContent::*;
use crate::diff::Diff::{MultiCell, SingleCell};
use crate::diff::{Diff, SingleCellDiff};
use crate::flag::Flag::*;
use crate::input_state::InputState;
use crate::tile_visibility::TileVisibility;
use crate::tile_visibility::TileVisibility::Hidden;
use crate::util::{DIRS_8, DIRS_9, fill_random, i_xy, valid_neighbors, xy_i};
use crate::win_state::WinState;
use crate::win_state::WinState::{Lost, Ongoing, Won};
use TileVisibility::Show;
use WinState::Untouched;
use std::cmp::{max, min};
use std::default::Default;
use std::fmt;
use std::fmt::{Display, Formatter};

#[derive(Debug, Default)]
pub struct Minesweeper {
    pub args: MinesweeperArgs,
    pub history: History,
    pub game_state: GameState,
    pub input_state: InputState,
    pub display: DisplayText,
}

#[derive(Debug, Default)]
pub struct DisplayText {
    pub text_top: &'static str,
    pub title: &'static str,
    pub text_bottom: &'static str,
    pub width_digits: usize,
    pub height_digits: usize,
    pub mines_digits: usize,
}

#[derive(Debug, Default)]
pub struct History {
    pub entries: Vec<Diff>,
    // from the back
    pub index: usize,
}

impl History {
    fn push(&mut self, diff: Diff) {
        self.entries.truncate(self.entries.len() - self.index);
        self.index = 0;
        self.entries.push(diff);
    }
    fn step_forward(&mut self, game: &mut GameState) {
        let mut i = self.index;
        if i == 0 {
            return;
        }
        i -= 1;
        self.index = i;

        let ri = self.entries.len() - i - 1;
        game.apply(&self.entries[ri]);
    }
    fn step_back(&mut self, game: &mut GameState) {
        if self.index >= self.entries.len() {
            return;
        }
        let ri = self.entries.len() - self.index - 1;
        self.index += 1;
        game.undo(&self.entries[ri]);
    }
}

#[derive(Debug, Default)]
pub struct GameState {
    pub win_state: WinState,
    pub cells: Vec<Cell>,
    pub flagged_cells: u32,
    pub closed_empty_cells: u32,
    pub open_mine_cells: u32,
}

impl Minesweeper {
    pub fn get_tile(&self, x: u16, y: u16) -> Option<&Cell> {
        let w = self.args.width;
        let h = self.args.height;
        xy_i((x, y), w, h).map(|i| &self.game_state.cells[i])
    }
}

impl Minesweeper {
    pub fn new(args: MinesweeperArgs) -> Self {
        let args = args.clamped();
        let width = args.width;
        let height = args.height;
        let mines = args.mines;

        let size = width as u32 * height as u32;

        const RETRY: &str = "(R)etry (Q)uit";
        const RETRY_SHORT: &str = "(R) (Q)";
        const NEXT: &str = "(N)ext (P)rev";
        const NEXT_SHORT: &str = "(N) (P)";
        const TITLE: &str = "Minesweeper!";
        const TITLE_SHORT: &str = "mnswpr!!";

        let title = if args.width < TITLE.len() as u16 {
            TITLE_SHORT
        } else {
            TITLE
        };
        let (text_top, text_bottom) = if args.width < max(RETRY.len(), NEXT.len()) as u16 {
            (RETRY_SHORT, NEXT_SHORT)
        } else {
            (RETRY, NEXT)
        };

        let max_x = width - 1;
        let width_digits = max_x.to_string().len();
        let max_y = height - 1;
        let height_digits = max_y.to_string().len();
        let mines_digits = mines.to_string().len();

        let display = DisplayText {
            text_top,
            title,
            text_bottom,
            width_digits,
            height_digits,
            mines_digits,
        };

        let game_state = GameState {
            cells: vec![Cell::default(); size as usize],
            closed_empty_cells: size - mines,
            ..GameState::default()
        };

        Self {
            args,
            display,
            game_state,
            ..Self::default()
        }
    }

    pub fn update(&mut self) {
        let Some(n) = self.input_state.action else {
            return;
        };
        let args @ MinesweeperArgs {
            mines,
            width: w,
            height: h,
        } = self.args;
        match n {
            Command(a) => 'b: {
                if let (OpenCell(cursor), Untouched) = (a, self.game_state.win_state) {
                    // initialization
                    if let None = xy_i(cursor, w, h) {
                        break 'b;
                    }
                    initialize(&mut self.game_state.cells, cursor, args);
                    self.game_state.win_state = Ongoing;
                }

                let Some(diff) = a.apply(&mut self.game_state, &self.args) else {
                    break 'b;
                };
                self.game_state.apply(&diff);
                self.history.push(diff);
            }
            Restart(option) => {
                if let Some(action) = option {
                    match action {
                        IncrementMinesPercent(unit) => {
                            let size: u32 = w as u32 * h as u32;
                            let hundredth = max(1, size / 100);
                            let increment = unit as i32;
                            self.args.mines = if increment > 0 {
                                (mines + 1).next_multiple_of(hundredth)
                            } else {
                                mines.saturating_sub(hundredth).next_multiple_of(hundredth)
                            };
                        }
                        ResizeH(dx) => {
                            self.args.width = self.args.width.saturating_add_signed(dx as i16);
                        }
                        ResizeV(dy) => {
                            self.args.height = self.args.height.saturating_add_signed(dy as i16);
                        }
                        IncrementMines(sign) => {
                            self.args.mines = self.args.mines.saturating_add_signed(sign as i32);
                        }
                    }
                }
                let cursor = self.input_state.cursor;
                *self = Self::new(self.args);
                self.input_state.cursor = (
                    cursor.0.clamp(0, self.args.width - 1),
                    cursor.1.clamp(0, self.args.height - 1),
                );
                self.input_state.cursor = cursor
            }
            Debug(a) => match a {
                Undo => self.history.step_back(&mut self.game_state),
                Redo => self.history.step_forward(&mut self.game_state),
            },
        };

        self.input_state.action = None;
    }

    pub fn move_cursor(&mut self, dx: i32, dy: i32) {
        let (x, y) = &mut self.input_state.cursor;
        *x = if dx < 0 {
            x.saturating_sub(-dx as u16)
        } else {
            min(self.args.width - 1, *x + dx as u16)
        };

        *y = if dy < 0 {
            y.saturating_sub(-dy as u16)
        } else {
            min(self.args.height - 1, *y + dy as u16)
        };
    }
}

fn initialize(cells: &mut Vec<Cell>, cursor: Cursor, args: MinesweeperArgs) {
    let m = args.mines;
    let w = args.width;
    let h = args.height;
    let neighbors = valid_neighbors(&DIRS_9, cursor, w, h);

    let mines = fill_random(
        neighbors.map(|cursor| xy_i(cursor, w, h).unwrap()),
        w as usize * h as usize,
        m as usize,
        false,
        true,
    );

    for (i, &has_mine) in mines.iter().enumerate() {
        if !has_mine {
            continue;
        }
        cells[i].content = Mine;
        let mine_cursor = i_xy(i, w, h).unwrap();
        for neigh_cursor in valid_neighbors(&DIRS_8, mine_cursor, w, h) {
            let neigh_idx = xy_i(neigh_cursor, w, h).unwrap();
            let neigh_cell = &mut cells[neigh_idx];
            if let Empty(ref mut n) = neigh_cell.content {
                *n += 1;
            };
        }
    }
}

impl GameState {
    fn apply_single_diff(
        &mut self,
        SingleCellDiff {
            index,
            before,
            after,
        }: &SingleCellDiff,
    ) {
        //let cell = &mut self.cells[*index];
        //assert_eq!(*before, *cell);

        self.apply_state(before, after);
        let cell = &mut self.cells[*index];
        *cell = *after;
    }
    fn undo_single_diff(
        &mut self,
        SingleCellDiff {
            index,
            before,
            after,
        }: &SingleCellDiff,
    ) {
        let cell = &self.cells[*index];
        assert_eq!(*after, *cell);

        self.apply_state(after, before);
        let cell = &mut self.cells[*index];
        *cell = *before;
    }

    fn apply_state(&mut self, before: &Cell, after: &Cell) {
        let visibility_diff = (before.content, before.visibility, after.visibility);

        match visibility_diff {
            // empty
            (Empty(_), Hidden(_), Show) => self.closed_empty_cells -= 1,
            (Empty(_), Show, Hidden(_)) => self.closed_empty_cells += 1,
            // mine
            (Mine, Hidden(_), Show) => self.open_mine_cells += 1,
            (Mine, Show, Hidden(_)) => self.open_mine_cells -= 1,
            _ => {}
        };

        match visibility_diff {
            (_, Show | Hidden(FlaggedMaybe | Clear), Hidden(Flagged)) => self.flagged_cells += 1,
            (_, Hidden(Flagged), Show | Hidden(FlaggedMaybe | Clear)) => self.flagged_cells -= 1,
            _ => {}
        };

        self.win_state = match (self.closed_empty_cells, self.open_mine_cells) {
            (0, 0) => Won,
            (_, 0) => Ongoing,
            (_, _) => Lost,
        }
    }

    fn apply(&mut self, diff: &Diff) {
        match diff {
            SingleCell(diff) => {
                self.apply_single_diff(diff);
            }
            MultiCell(diffs) => {
                for diff in diffs {
                    self.apply_single_diff(diff);
                }
            }
        }
    }

    fn undo(&mut self, diff: &Diff) {
        match diff {
            SingleCell(diff) => {
                self.undo_single_diff(diff);
            }
            MultiCell(diffs) => {
                for diff in diffs {
                    self.undo_single_diff(diff);
                }
            }
        }
    }
}

impl Display for Minesweeper {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for line in self.game_state.cells.chunks_exact(self.args.width as usize) {
            for cell in line {
                write!(f, "{cell}")?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}
