use crate::FlagState::*;
use crate::MineState::*;
use crate::Visibility::*;
use crate::WinState::*;
use Action::*;
use args::MinesweeperArgs;
use clap::Parser;
// use log::LevelFilter;
use rand::RngCore;
// use simplelog::{Config, WriteLogger};
use std::cmp::{max, min};
use std::collections::{BTreeSet, VecDeque};
use std::fmt::{Debug, Display, Formatter, Result, Write};
// use std::fs::File;
use std::iter::Iterator;

mod args;

const DIRS_8: [(i8, i8); 8] = [
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
];
const DIRS_9: [(i8, i8); 9] = [
    (0, 0),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
];

#[derive(Copy, Clone, Debug)]
enum MineState {
    Empty(u8),
    Mine,
}
impl Default for MineState {
    fn default() -> Self {
        Empty(0)
    }
}
#[derive(Copy, Clone, Debug)]
enum FlagState {
    None,
    Flagged,
    FlaggedMaybe,
}
impl FlagState {
    // man this sucks, you'd think compile time sized enums would be a given
    const SIZE: u32 = FlaggedMaybe as u32 + 1;
    fn next(self) -> Self {
        // you'd also think "if enum to int is allowed, so is int to enum", well think again
        let next = (self as u32 + 1) % Self::SIZE;
        match next {
            0 => None,
            1 => Flagged,
            2 => FlaggedMaybe,
            //purposely not _ so that it breaks if new flags are added
            Self::SIZE.. => None, // unreachable due to previous line
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum Visibility {
    Hidden(FlagState),
    Show,
}
impl Default for Visibility {
    fn default() -> Self {
        Hidden(None)
    }
}
#[derive(Copy, Clone, Debug, Default)]
struct Tile {
    visibility: Visibility,
    mine: MineState,
}

impl Display for Tile {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let c = match self.visibility {
            Hidden(flag) => match flag {
                None => '#',
                Flagged => '!',
                FlaggedMaybe => '?',
            },
            Show => match self.mine {
                Empty(neighbor_mines) => {
                    if neighbor_mines == 0 {
                        '.'
                    } else {
                        std::char::from_digit(neighbor_mines as u32, 10).unwrap()
                    }
                }
                Mine => '*',
            },
        };

        f.write_char(c)
    }
}

#[derive(Copy, Clone, Debug)]
enum WinState {
    Untouched,
    Ongoing,
    Lost,
    Won,
}
impl Default for WinState {
    fn default() -> Self {
        Untouched
    }
}

#[derive(Copy, Clone, Debug)]
enum Unit {
    Negative = -1,
    Zero = 0,
    Positive = 1,
}
#[derive(Copy, Clone, Debug)]
enum Action {
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

#[derive(Default, Debug)]
struct InputState {
    cursor: (u16, u16),
    action: Option<Action>,
}
#[derive(Debug, Default)]
struct Minesweeper {
    args: MinesweeperArgs,
    win_state: WinState,
    tiles: Vec<Tile>,
    input_state: InputState,
    flagged_tiles: u32,
    shown_tiles: u32,
    text_top: &'static str,
    title: &'static str,
    text_bottom: &'static str,
    width_digits: usize,
    height_digits: usize,
    mines_digits: usize,
    // utility struct to ensure it's only allocated once
    point_stack: VecDeque<(u16, u16)>,
}

impl Minesweeper {
    pub fn get_tile(&self, x: i16, y: i16) -> Option<&Tile> {
        Self::_get_tile(&self.tiles, self.args.width, self.args.height, x, y)
    }
}

impl Minesweeper {
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

impl Display for Minesweeper {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        for line in self.tiles.chunks_exact(self.args.width as usize) {
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

impl Minesweeper {
    pub fn new(args: MinesweeperArgs) -> Self {
        let args = args.clamped();
        let width = args.width;
        let height = args.height;
        let mines = args.mines;

        let size = width as u32 * height as u32;

        const RETRY: &'static str = "(R)etry (Q)uit";
        const RETRY_SHORT: &'static str = "(R) (Q)";
        const NEXT: &'static str = "(N)ext (P)rev";
        const NEXT_SHORT: &'static str = "(N) (P)";
        const TITLE: &'static str = "Minesweeper!";
        const TITLE_SHORT: &'static str = "mnswpr!!";

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
            tiles: vec![Tile::default(); size as usize],
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
        let cursor @ (xu, yu) = self.input_state.cursor;
        let (x, y) = (xu as i16, yu as i16);
        match n {
            ShowTile => self.show_tile(x, y),
            FlagTile => self.flag_tile(x, y),
            ClearFlag => self.clear_flag(x, y),
            Surrender => {
                self.show_all();
                self.win_state = Lost
            }
            Restart => {
                *self = Self::new(self.args);
                self.input_state.cursor = cursor
            }
            Next => {
                self.args.mines += self.args.mines / 5 + 1;
                *self = Self::new(self.args);
                self.input_state.cursor = cursor
            }
            Previous => {
                self.args.mines -= self.args.mines / 6 + 1;
                *self = Self::new(self.args);
                self.input_state.cursor = cursor
            }
            Resize(dx, dy) => {
                self.args.width = self.args.width.saturating_add_signed(dx as i16);
                self.args.height = self.args.height.saturating_add_signed(dy as i16);
                *self = Self::new(self.args);
                self.input_state.cursor = (
                    xu.clamp(0, self.args.width - 1),
                    yu.clamp(0, self.args.height - 1),
                )
            }
            IncrementMines(sign) => {
                self.args.mines = self.args.mines.saturating_add_signed(sign as i32);
                *self = Self::new(self.args);
                self.input_state.cursor = cursor
            }
        };

        self.input_state.action = Option::None;
    }

    fn set_mine(&mut self, x: i16, y: i16) {
        let w = self.args.width;
        let h = self.args.height;

        // match empty tile
        let Some(Tile {
            mine: mine @ Empty(_),
            ..
        }) = &mut Self::_get_tile_mut(&mut self.tiles, w, h, x, y)
        else {
            return;
        };

        *mine = Mine;
        for (dx, dy) in DIRS_8 {
            let (i, j) = (x + dx as i16, y + dy as i16);
            let Some(Tile {
                mine: Empty(count), ..
            }) = &mut Self::_get_tile_mut(&mut self.tiles, w, h, i, j)
            else {
                continue;
            };
            *count += 1
        }
    }

    fn show_tile(&mut self, x: i16, y: i16) {
        let w = self.args.width;
        let h = self.args.height;

        if let Untouched = self.win_state {
            let x = x.clamp(1, w as i16 - 2) as u16;
            let y = y.clamp(1, h as i16 - 2) as u16;
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
                self.set_mine((i % w as usize) as i16, (i / w as usize) as i16);
            }
            self.win_state = Ongoing
        }

        let Ongoing = self.win_state else { return };

        let Some(tile) = Self::_get_tile_mut(&mut self.tiles, w, h, x, y) else {
            return;
        };
        if let Show | Hidden(Flagged) = tile.visibility {
            return;
        }

        let flagged_tiles = &mut self.flagged_tiles;
        let shown_tiles = &mut self.shown_tiles;
        Self::_show_tile(&mut tile.visibility, flagged_tiles, shown_tiles);

        if let Mine = tile.mine {
            // explode
            for tile in &mut self.tiles {
                let Mine = tile.mine else { continue };
                Self::_show_tile(&mut tile.visibility, flagged_tiles, shown_tiles);
            }
            self.win_state = Lost;
            return;
        }
        let w = w;
        let h = self.args.height;

        // Self::fill_8_way_stack(
        //     x,
        //     y,
        //     |x, y| {
        //         Self::_get_tile(&mut self.tiles, w, h, x, y);
        //     },
        //     |x, y| true,
        // );

        (&mut self.point_stack).push_back((x as u16, y as u16));

        while let Some((xu, yu)) = (&mut self.point_stack).pop_front() {
            let (x, y) = (xu as i16, yu as i16);
            let Some(tile) = Self::_get_tile_mut(&mut self.tiles, w, h, x, y) else {
                continue;
            };

            match tile.mine {
                Empty(0) => {
                    // expand
                    for (dx, dy) in DIRS_8 {
                        let (i, j) = (x + dx as i16, y + dy as i16);
                        let Some(tile) = Self::_get_tile_mut(&mut self.tiles, w, h, i, j) else {
                            continue;
                        };
                        let Hidden(_) = tile.visibility else { continue };
                        let flagged_tiles = &mut self.flagged_tiles;
                        let shown_tiles = &mut self.shown_tiles;
                        Self::_show_tile(&mut tile.visibility, flagged_tiles, shown_tiles);
                        (&mut self.point_stack).push_back((i as u16, j as u16));
                        //log::info!("\n{self}");
                    }
                }

                _ => {}
            }
        }

        if self.shown_tiles + self.args.mines == w as u32 * self.args.height as u32 {
            self.win_state = Won
        }
    }

    pub fn clear_flag(&mut self, x: i16, y: i16) {
        let Some(tile) =
            Self::_get_tile_mut(&mut self.tiles, self.args.width, self.args.height, x, y)
        else {
            return;
        };
        match tile.visibility {
            Show => return,
            Hidden(flag) => {
                if let Flagged = flag {
                    self.flagged_tiles -= 1;
                }
                tile.visibility = Hidden(None);
            }
        }
    }

    pub fn flag_tile(&mut self, x: i16, y: i16) {
        let w = self.args.width;
        let h = self.args.height;
        let Some(tile) = Self::_get_tile_mut(&mut self.tiles, w, h, x, y) else {
            return;
        };
        let Hidden(flag) = tile.visibility else {
            return;
        };
        if let Flagged = flag {
            self.flagged_tiles -= 1;
        }

        let mut flag = flag.next();
        if let Flagged = flag {
            if self.flagged_tiles == self.args.mines {
                flag = flag.next();
            } else {
                self.flagged_tiles += 1;
            }
        }
        tile.visibility = Hidden(flag);
    }
    fn _show_tile(visibility: &mut Visibility, flagged_tiles: &mut u32, shown_tiles: &mut u32) {
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
    pub fn show_all(&mut self) {
        for tile in &mut self.tiles {
            Self::_show_tile(
                &mut tile.visibility,
                &mut self.flagged_tiles,
                &mut self.shown_tiles,
            );
        }
    }

    fn _get_tile_mut(vec: &mut [Tile], w: u16, h: u16, x: i16, y: i16) -> Option<&mut Tile> {
        if !(0..w as i16).contains(&x) || !(0..h as i16).contains(&y) {
            Option::None
        } else {
            Some(&mut vec[x as usize + y as usize * w as usize])
        }
    }

    fn _get_tile(vec: &[Tile], w: u16, h: u16, x: i16, y: i16) -> Option<&Tile> {
        if !(0..w as i16).contains(&x) || !(0..h as i16).contains(&y) {
            Option::None
        } else {
            Some(&vec[x as usize + y as usize * w as usize])
        }
    }
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

fn main() {
    let args = MinesweeperArgs::parse();
    // WriteLogger::init(
    //     LevelFilter::Info,
    //     Config::default(),
    //     File::create("debug.log").unwrap(),
    // )
    // .unwrap();
    // log::info!("This will go to a file");
    ui::main(args).unwrap()
}

mod ui {
    use crate::Action::*;
    use crate::FlagState::*;
    use crate::Unit::{Negative, Positive, Zero};
    use crate::Visibility::*;
    use crate::args::MinesweeperArgs;
    use crate::{InputState, MineState, Minesweeper, WinState};
    use color_eyre::Result;
    use crossterm::ExecutableCommand;
    use crossterm::event::{
        self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind,
    };
    use ratatui::buffer::Cell;
    use ratatui::layout::{Position, Rect};
    use ratatui::style::Color;
    use ratatui::style::Color::*;
    use ratatui::{
        DefaultTerminal, Frame,
        style::Stylize,
        text::Line,
        widgets::{Block, Paragraph},
    };

    pub fn main(args: MinesweeperArgs) -> Result<()> {
        color_eyre::install()?;
        let terminal = ratatui::init();
        let result = App::new(args).run(terminal);
        ratatui::restore();
        result
    }

    /// The main application which holds the state and logic of the application.
    #[derive(Debug, Default)]
    pub struct App {
        /// Is the application running?
        running: bool,
        game: Minesweeper,
    }
    impl App {
        /// Construct a new instance of [`App`].
        pub fn new(args: MinesweeperArgs) -> Self {
            Self {
                game: Minesweeper::new(args),
                ..Self::default()
            }
        }

        /// Run the application's main loop.
        pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
            std::io::stdout().execute(event::EnableMouseCapture)?;

            self.running = true;
            while self.running {
                terminal.draw(|frame| self.render(frame))?;
                self.handle_crossterm_events()?;
                self.game.update();
            }

            std::io::stdout().execute(event::DisableMouseCapture)?;
            Ok(())
        }

        /// Renders the user interface.
        ///
        /// This is where you add new widgets. See the following resources for more information:
        ///
        /// - <https://docs.rs/ratatui/latest/ratatui/widgets/index.html>
        /// - <https://github.com/ratatui/ratatui/tree/main/ratatui-widgets/examples>
        fn render(&mut self, frame: &mut Frame) {
            let Minesweeper {
                args:
                    MinesweeperArgs {
                        width,
                        height,
                        mines,
                        ..
                    },
                win_state,
                text_top,
                text_bottom,
                title,
                input_state: InputState { cursor: (x, y), .. },
                flagged_tiles,
                width_digits,
                height_digits,
                mines_digits,
                ..
            } = self.game;

            let (title, bottom) = match win_state {
                WinState::Untouched => (
                    Line::from(title).bold().light_blue().centered(),
                    Line::from(format!("{}x{},{}", width, height, mines)).centered(),
                ),
                WinState::Won => (
                    Line::from(text_top).bold().light_green().centered(),
                    Line::from(text_bottom).bold().light_green().centered(),
                ),
                WinState::Lost => (
                    Line::from(text_top).bold().light_red().centered(),
                    Line::from(text_bottom).bold().light_red().centered(),
                ),
                _ => {
                    let mut stats = format!(
                        "{:mines_digits$}/{} ({:width_digits$},{:height_digits$}) {}x{}",
                        flagged_tiles, mines, x, y, width, height
                    );
                    if stats.len() as u16 > width {
                        stats = format!("{} {},{}", mines - flagged_tiles, x, y);
                    }

                    (
                        Line::from(title).bold().light_blue().centered(),
                        Line::from(stats).centered(),
                    )
                }
            };
            let area = frame.area().clamp(Rect::new(0, 0, width + 2, height + 2));

            frame.render_widget(
                Paragraph::new("")
                    .block(Block::bordered().title(title).title_bottom(bottom))
                    .centered(),
                area,
            );

            if area.height == 0 && area.width == 0 {
                return;
            }

            for j in area.y + 1..area.y + area.height - 1 {
                for i in area.x + 1..area.x + area.width - 1 {
                    //                    let tile = self.game[(j - 1) as usize][(i - 1) as usize];
                    let Some(tile) = self.game.get_tile(i as i16 - 1, j as i16 - 1) else {
                        continue;
                    };

                    const HIDDEN_COLOR: Color = Reset;
                    const WARN_COLOR: Color = LightYellow;
                    const NUM_COLOR: Color = Black;
                    const NUM_COLOR2: Color = Black;

                    let (char, bg, fg) = match tile.visibility {
                        Hidden(f) => match f {
                            None => ('#', Reset, HIDDEN_COLOR),
                            Flagged => ('!', LightRed, WARN_COLOR),
                            FlaggedMaybe => ('?', LightRed, WARN_COLOR),
                        },
                        Show => match tile.mine {
                            MineState::Empty(n) => match n {
                                0 => (' ', Black, Reset),
                                1 => ('1', LightBlue, NUM_COLOR),
                                2 => ('2', LightCyan, NUM_COLOR),
                                3 => ('3', LightGreen, NUM_COLOR),
                                4 => ('4', LightYellow, NUM_COLOR),
                                5 => ('5', LightMagenta, NUM_COLOR2),
                                6 => ('6', Gray, NUM_COLOR2),
                                7 => ('7', White, NUM_COLOR2),
                                8.. => ('8', LightRed, NUM_COLOR2),
                            },
                            MineState::Mine => ('*', Black, LightRed),
                        },
                    };

                    let w = frame.area().width;
                    let mut c = Cell::new("");
                    c.set_char(char).set_fg(fg).set_bg(bg);
                    frame.buffer_mut().content[w as usize * j as usize + i as usize] = c;
                }
            }

            frame.set_cursor_position(Position {
                x: self.game.input_state.cursor.0 + 1,
                y: self.game.input_state.cursor.1 + 1,
            });
        }

        fn handle_crossterm_events(&mut self) -> Result<()> {
            match event::read()? {
                // it's important to check KeyEventKind::Press to avoid handling key release events
                Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key_event(key),
                Event::Mouse(m) => match m.kind {
                    MouseEventKind::Down(button) => 'block: {
                        if !(1..self.game.args.width + 1).contains(&m.column)
                            || !(1..self.game.args.height + 1).contains(&m.row)
                        {
                            break 'block;
                        }
                        self.game.input_state.cursor = (m.column - 1, m.row - 1);
                        match button {
                            MouseButton::Left => self.game.input_state.action = Some(ShowTile),
                            MouseButton::Right | MouseButton::Middle => {
                                self.game.input_state.action = Some(FlagTile)
                            }
                        };
                    }
                    _ => {}
                },
                Event::Resize(_, _) => {}
                _ => {}
            }
            Ok(())
        }

        /// Handles the key events and updates the state of [`App`].
        fn on_key_event(&mut self, key: KeyEvent) {
            match (key.modifiers, key.code) {
                (_, KeyCode::Esc | KeyCode::Char('q'))
                | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
                // Add other key handlers here.
                (_, KeyCode::Char('k')) => {
                    self.game.input_state.action = Some(Surrender);
                }
                (_, KeyCode::Char('r')) => {
                    self.game.input_state.action = Some(Restart);
                }
                (_, KeyCode::Char('n')) => {
                    self.game.input_state.action = Some(Next);
                }
                (_, KeyCode::Char('p')) => {
                    self.game.input_state.action = Some(Previous);
                }
                (_, KeyCode::Char('x' | ' ')) => {
                    self.game.input_state.action = Some(ShowTile);
                }
                (_, KeyCode::Char('z' | 'f')) => {
                    self.game.input_state.action = Some(FlagTile);
                }
                (_, KeyCode::Backspace) => {
                    self.game.input_state.action = Some(ClearFlag);
                }
                (_, KeyCode::Char('+')) => {
                    self.game.input_state.action = Some(IncrementMines(Positive));
                }
                (_, KeyCode::Char('-')) => {
                    self.game.input_state.action = Some(IncrementMines(Negative));
                }
                (
                    modifiers,
                    key @ (KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down),
                ) => {
                    let (x, y) = match key {
                        KeyCode::Left => (Negative, Zero),
                        KeyCode::Right => (Positive, Zero),
                        KeyCode::Up => (Zero, Negative),
                        KeyCode::Down => (Zero, Positive),
                        _ => unreachable!(),
                    };
                    if modifiers.contains(KeyModifiers::SHIFT) {
                        self.game.input_state.action = Some(Resize(x, y));
                    } else {
                        self.game.move_cursor(x as i32, y as i32);
                    }
                }

                _ => {}
            }
        }

        /// Set running to false to quit the application.
        fn quit(&mut self) {
            self.running = false;
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {}
}
