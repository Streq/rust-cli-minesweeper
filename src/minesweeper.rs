use crate::action::Action::*;
use crate::action::Action::*;
use crate::action::DebugAction::*;
use crate::action::GameAction::*;
use crate::action::RestartAction::*;
use crate::action::{DebugAction, RestartAction};
use crate::args::MinesweeperArgs;
use crate::flag::Flag::*;
use crate::input_state::InputState;
use crate::tile::Tile;
use crate::tile_content::TileContent::*;
use crate::tile_visibility::TileVisibility;
use crate::tile_visibility::TileVisibility::*;
use crate::util::*;
use crate::win_state::WinState;
use crate::win_state::WinState::*;
use rand::RngCore;
use std::cmp::{max, min};
use std::collections::{BTreeSet, VecDeque};
use std::fmt;
use std::fmt::{Display, Formatter};

#[derive(Debug, Default)]
pub struct Minesweeper {
    pub args: MinesweeperArgs,
    pub win_state: WinState,
    pub cells: Vec<Tile>,
    pub input_state: InputState,
    pub flagged_cells: u32,
    pub open_cells: u32,

    // display fields, maybe should be moved somewhere else
    pub text_top: &'static str,
    pub title: &'static str,
    pub text_bottom: &'static str,
    pub width_digits: usize,
    pub height_digits: usize,
    pub mines_digits: usize,

    // utility struct to ensure it's only allocated once
    pub point_stack: VecDeque<(u16, u16)>,
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

        Self {
            args,
            cells: vec![Tile::default(); size as usize],
            title,
            text_top,
            text_bottom,
            width_digits,
            height_digits,
            mines_digits,
            point_stack: VecDeque::<(u16, u16)>::new(),
            ..Self::default()
        }
    }

    pub fn update(&mut self) {
        let Some(n) = self.input_state.action else {
            return;
        };
        let MinesweeperArgs {
            mines,
            width: w,
            height: h,
        } = self.args;
        match n {
            Command(a) => match a {
                OpenCell((x, y)) => self.show_tile(x, y),
                FlagCell((x, y)) => self.flag_tile(x, y),
                ClearFlag((x, y)) => self.clear_flag(x, y),
                Surrender => {
                    self.show_all();
                    self.win_state = Lost
                }
            },
            Restart(o) => {
                if let Some(a) = o {
                    match a {
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
                Undo => {}
                Redo => {}
            },
        };

        self.input_state.action = Option::None;
    }

    fn set_mine(&mut self, x: u16, y: u16) {
        let w = self.args.width;
        let h = self.args.height;

        // match empty tile
        let Some(Tile {
            content: mine @ Empty(_),
            ..
        }) = &mut Self::_get_tile_mut(&mut self.cells, w, h, x, y)
        else {
            return;
        };

        *mine = Mine;
        for (dx, dy) in DIRS_8 {
            let (i, j) = (
                x.overflowing_add_signed(dx as i16).0,
                y.overflowing_add_signed(dy as i16).0,
            );
            let Some(Tile {
                content: Empty(count),
                ..
            }) = &mut Self::_get_tile_mut(&mut self.cells, w, h, i, j)
            else {
                continue;
            };
            *count += 1
        }
    }

    fn show_tile(&mut self, x: u16, y: u16) {
        let w = self.args.width;
        let h = self.args.height;

        if let Untouched = self.win_state {
            let x = x.clamp(1, w - 2);
            let y = y.clamp(1, h - 2);
            let whitelisted = valid_neighbors(&DIRS_9, x, y, w, h);

            let size = w as usize * h as usize;
            let mine_grid = fill_random(
                whitelisted.map(|(x, y)| x as usize + y as usize * w as usize),
                size,
                self.args.mines as usize,
                false,
                true,
            );

            for (i, _) in mine_grid
                .iter()
                .enumerate()
                .filter(|(_, is_mine)| **is_mine)
            {
                self.set_mine((i % w as usize) as u16, (i / w as usize) as u16);
            }
            self.win_state = Ongoing
        }

        let Ongoing = self.win_state else { return };

        let Some(tile) = Self::_get_tile_mut(&mut self.cells, w, h, x, y) else {
            return;
        };
        if let Show | Hidden(Flagged) = tile.visibility {
            return;
        }

        let flagged_tiles = &mut self.flagged_cells;
        let shown_tiles = &mut self.open_cells;
        Self::_show_tile(&mut tile.visibility, flagged_tiles, shown_tiles);

        if let Mine = tile.content {
            // explode
            for tile in &mut self.cells {
                let Mine = tile.content else { continue };
                Self::_show_tile(&mut tile.visibility, flagged_tiles, shown_tiles);
            }
            self.win_state = Lost;
            return;
        }
        let w = w;
        let h = self.args.height;

        (&mut self.point_stack).push_back((x, y));

        while let Some((x, y)) = (&mut self.point_stack).pop_front() {
            let Some(tile) = Self::_get_tile_mut(&mut self.cells, w, h, x, y) else {
                continue;
            };

            match tile.content {
                Empty(0) => {
                    // expand
                    for (dx, dy) in DIRS_8 {
                        let (i, j) = (
                            x.overflowing_add_signed(dx as i16).0,
                            y.overflowing_add_signed(dy as i16).0,
                        );
                        let Some(tile) = Self::_get_tile_mut(&mut self.cells, w, h, i, j) else {
                            continue;
                        };
                        let Hidden(_) = tile.visibility else { continue };
                        let flagged_tiles = &mut self.flagged_cells;
                        let shown_tiles = &mut self.open_cells;
                        Self::_show_tile(&mut tile.visibility, flagged_tiles, shown_tiles);
                        (&mut self.point_stack).push_back((i, j));
                        //log::info!("\n{self}");
                    }
                }

                _ => {}
            }
        }

        if self.open_cells + self.args.mines == w as u32 * self.args.height as u32 {
            self.win_state = Won
        }
    }

    pub fn clear_flag(&mut self, x: u16, y: u16) {
        let Some(tile) =
            Self::_get_tile_mut(&mut self.cells, self.args.width, self.args.height, x, y)
        else {
            return;
        };
        match tile.visibility {
            Show => return,
            Hidden(flag) => {
                if let Flagged = flag {
                    self.flagged_cells -= 1;
                }
                tile.visibility = Hidden(Clear);
            }
        }
    }

    pub fn flag_tile(&mut self, x: u16, y: u16) {
        let w = self.args.width;
        let h = self.args.height;
        let Some(tile) = Self::_get_tile_mut(&mut self.cells, w, h, x, y) else {
            return;
        };
        let Hidden(flag) = tile.visibility else {
            return;
        };
        if let Flagged = flag {
            self.flagged_cells -= 1;
        }

        let mut flag = flag.next();
        if let Flagged = flag {
            if self.flagged_cells == self.args.mines {
                flag = flag.next();
            } else {
                self.flagged_cells += 1;
            }
        }
        tile.visibility = Hidden(flag);
    }
    pub fn show_all(&mut self) {
        for tile in &mut self.cells {
            Self::_show_tile(
                &mut tile.visibility,
                &mut self.flagged_cells,
                &mut self.open_cells,
            );
        }
    }

    pub fn get_tile(&self, x: u16, y: u16) -> Option<&Tile> {
        Self::_get_tile(&self.cells, self.args.width, self.args.height, x, y)
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

//private methods
impl Minesweeper {
    fn _show_tile(visibility: &mut TileVisibility, flagged_tiles: &mut u32, shown_tiles: &mut u32) {
        match visibility {
            Hidden(f) => {
                if let Flagged = f {
                    *flagged_tiles -= 1
                }
                *shown_tiles += 1
            }
            _ => {}
        }
        *visibility = Show
    }
    fn _get_tile_mut(vec: &mut [Tile], w: u16, h: u16, x: u16, y: u16) -> Option<&mut Tile> {
        if w <= x || h <= y {
            None
        } else {
            Some(&mut vec[x as usize + y as usize * w as usize])
        }
    }

    fn _get_tile(vec: &[Tile], w: u16, h: u16, x: u16, y: u16) -> Option<&Tile> {
        if w <= x || h <= y {
            None
        } else {
            Some(&vec[x as usize + y as usize * w as usize])
        }
    }
}

impl Display for Minesweeper {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for line in self.cells.chunks_exact(self.args.width as usize) {
            for cell in line {
                write!(f, "{cell}")?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

fn valid_neighbors(
    dirs: &[(i8, i8)],
    x: u16,
    y: u16,
    w: u16,
    h: u16,
) -> impl Iterator<Item = (u16, u16)> {
    dirs.iter()
        .map(|(dx, dy)| (*dx as i16, *dy as i16))
        .map(move |(dx, dy)| (x.saturating_add_signed(dx), y.saturating_add_signed(dy)))
        .filter(move |(i, j)| (0..w).contains(i) && (0..h).contains(j))
}

fn fill_random<T: PartialEq + Copy>(
    whitelisted: impl Iterator<Item = usize>,
    size: usize,
    fills: usize,
    init_value: T,
    value: T,
) -> Vec<T> {
    let mut whitelisted: BTreeSet<usize> = BTreeSet::from_iter(whitelisted);
    let (fills, init_value, value, flip) = if fills > size / 2 {
        (size - fills - whitelisted.len(), value, init_value, true)
    } else {
        (fills, init_value, value, false)
    };

    let mut ret = vec![init_value; size];

    if flip {
        for wl in &whitelisted {
            ret[*wl] = value
        }
    }

    for _ in 0..fills {
        let mut r = rand::rng().next_u32() as usize % (ret.len() - whitelisted.len());

        for wl in whitelisted.iter() {
            r = if *wl <= r { r + 1 } else { break };
        }
        ret[r] = value;
        whitelisted.insert(r);
    }

    ret
}
