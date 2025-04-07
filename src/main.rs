use crate::FlagState::*;
use crate::MineState::*;
use crate::Visibility::*;
use crate::WinState::*;
use Action::*;
use args::MinesweeperArgs;
use clap::Parser;
use rand::RngCore;
use std::cmp::min;
use std::collections::VecDeque;
use std::fmt::{Debug, Display, Formatter, Result, Write};
use std::iter::Iterator;
use std::ops::{Index, IndexMut, Range};

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
        let c = match *self {
            Tile {
                visibility: Hidden(None),
                ..
            } => '#',
            Tile {
                visibility: Hidden(Flagged),
                ..
            } => '!',
            Tile {
                visibility: Hidden(FlaggedMaybe),
                ..
            } => '?',
            Tile {
                visibility: Show,
                mine: Empty(neighbor_mines),
            } => {
                if neighbor_mines == 0 {
                    '.'
                } else {
                    std::char::from_digit(neighbor_mines as u32, 10).unwrap()
                }
            }
            Tile {
                visibility: Show,
                mine: Mine,
            } => '*',
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
enum Action {
    ShowTile,
    FlagTile,
    Surrender,
    Restart,
    Next,
}

#[derive(Default, Debug)]
struct InputState {
    cursor: (usize, usize),
    action: Option<Action>,
}
#[derive(Debug, Default)]
struct Minesweeper {
    args: MinesweeperArgs,
    tiles: Vec<Tile>,
    win_state: WinState,
    flagged_tiles: usize,
    shown_tiles: usize,
    input_state: InputState,
}

impl Minesweeper {
    pub fn move_cursor(&mut self, dx: i32, dy: i32) {
        let (x, y) = &mut self.input_state.cursor;
        *x = if dx < 0 {
            x.saturating_sub(-dx as usize)
        } else {
            min(self.args.width - 1, *x + dx as usize)
        };

        *y = if dy < 0 {
            y.saturating_sub(-dy as usize)
        } else {
            min(self.args.height - 1, *y + dy as usize)
        };
    }
}

impl Display for Minesweeper {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        for line in self.tiles.chunks_exact(self.args.width) {
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
    x: usize,
    y: usize,
    w: usize,
    h: usize,
) -> impl Iterator<Item = (usize, usize)> {
    dirs.iter()
        .map(move |(dx, dy)| (x as isize + *dx as isize, y as isize + *dy as isize))
        .filter(move |(i, j)| (0..w as isize).contains(i) && (0..h as isize).contains(j))
        .map(|(i, j)| (i as usize, j as usize))
}

impl Minesweeper {
    pub fn new(args: MinesweeperArgs) -> Self {
        let size = args.width * args.height;
        Self {
            args,
            tiles: vec![Tile::default(); size],
            ..Self::default()
        }
    }

    pub fn update(&mut self) {
        let Some(n) = self.input_state.action else {
            return;
        };
        let (x, y) = self.input_state.cursor;
        match n {
            ShowTile => self.show_tile(x, y),
            FlagTile => self.flag_tile(x, y),
            Surrender => {
                self.show_all();
                self.win_state = Lost
            }
            Restart => {
                *self = Self::new(self.args);
                self.input_state.cursor = (x, y)
            }
            Next => 'b: {
                let Won = self.win_state else { break 'b };
                *self = Self::new(MinesweeperArgs {
                    mines: self.args.mines + 1,
                    ..self.args
                });
                self.input_state.cursor = (x, y)
            }
        };

        self.input_state.action = Option::None;
    }

    fn set_mine(&mut self, x: usize, y: usize) {
        let tile = &mut self[y][x].mine;

        *tile = match *tile {
            Mine => return,
            _ => Mine,
        };
        for (i, j) in valid_neighbors(&DIRS_8, x, y, self.args.width, self.args.height) {
            let mine: &mut MineState = &mut self[j][i].mine;
            match mine {
                Empty(count) => *count += 1,
                Mine => continue,
            }
        }
    }

    fn show_tile(&mut self, x: usize, y: usize) {
        if let Untouched = self.win_state {
            let size = self.tiles.len();
            let whitelisted = valid_neighbors(&DIRS_9, x, y, self.args.width, self.args.height)
                .map(|(x, y)| y * self.args.width + x);
            let mines = make_random_holes(whitelisted, size - self.args.mines, 0..size);

            for i in mines {
                self.set_mine(i % self.args.width, i / self.args.width);
            }
            self.win_state = Ongoing
        }

        let Ongoing = self.win_state else { return };

        let tile = Self::get_tile(&mut self.tiles, self.args.width, x, y);
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

        let mut stack = VecDeque::<(usize, usize)>::new();
        stack.push_back((x, y));
        let w = self.args.width;
        let h = self.args.height;

        while let Some((x, y)) = stack.pop_back() {
            let tile = Self::get_tile(&mut self.tiles, w, x, y);

            match tile.mine {
                Empty(0) => {
                    // expand
                    for (i, j) in valid_neighbors(&DIRS_8, x, y, w, h) {
                        let tile = Self::get_tile(&mut self.tiles, w, i, j);
                        let Hidden(_) = tile.visibility else { continue };
                        Self::_show_tile(&mut tile.visibility, flagged_tiles, shown_tiles);
                        stack.push_back((i, j))
                    }
                }

                _ => {}
            }
        }

        if self.shown_tiles + self.args.mines == self.args.width * self.args.height {
            self.win_state = Won
        }
    }
    pub fn flag_tile(&mut self, x: usize, y: usize) {
        let tile = Self::get_tile(&mut self.tiles, self.args.width, x, y);
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
    fn _show_tile(visibility: &mut Visibility, flagged_tiles: &mut usize, shown_tiles: &mut usize) {
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

    fn get_tile(vec: &mut [Tile], w: usize, x: usize, y: usize) -> &mut Tile {
        &mut vec[x + y * w]
    }
}

fn make_random_holes(
    whitelisted: impl Iterator<Item = usize>,
    holes_to_make: usize,
    range: Range<usize>,
) -> Vec<usize> {
    let mut ret: Vec<usize> = Vec::from_iter(range);

    let mut whitelisted_count = 0;
    for i in whitelisted {
        ret.iter().position(|&n| n == i).map(|it| ret.remove(it));
        whitelisted_count += 1;
    }

    for _ in 0..holes_to_make - whitelisted_count {
        let n = rand::rng().next_u32() as usize % ret.len();
        ret.remove(n);
    }
    ret
}

impl Index<usize> for Minesweeper {
    type Output = [Tile];
    fn index(&self, index: usize) -> &Self::Output {
        let start = index * self.args.width;
        &self.tiles[start..start + self.args.width]
    }
}
impl IndexMut<usize> for Minesweeper {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let start = index * self.args.width;
        &mut self.tiles[start..start + self.args.width]
    }
}

fn main() {
    let args = MinesweeperArgs::parse();
    ui::main(args).unwrap()
}

mod ui {
    use crate::Action::{FlagTile, Next, Restart, ShowTile, Surrender};
    use crate::FlagState::*;
    use crate::Visibility::*;
    use crate::args::MinesweeperArgs;
    use crate::{MineState, Minesweeper, WinState};
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
            self.game.update();
            let title = match self.game.win_state {
                WinState::Lost => Line::from("You lose!! (R)estart (Q)uit").bold().light_red(),
                WinState::Won => Line::from("You win!!! (R)estart (Q)uit (N)ext")
                    .bold()
                    .light_green(),
                _ => Line::from("Minesweeper!").bold().light_blue(),
            }
            .centered();

            let game = &self.game;
            let area = frame.area().clamp(Rect::new(
                0,
                0,
                game.args.width as u16 + 2,
                game.args.height as u16 + 2,
            ));

            frame.render_widget(
                Paragraph::new("")
                    .block(Block::bordered().title(title).title_bottom(format!(
                        "{}/{} ({}, {})",
                        game.flagged_tiles,
                        game.args.mines,
                        game.input_state.cursor.0,
                        game.input_state.cursor.1
                    )))
                    .centered(),
                area,
            );

            if area.height == 0 && area.width == 0 {
                return;
            }

            for j in area.y + 1..area.y + area.height - 1 {
                for i in area.x + 1..area.x + area.width - 1 {
                    let tile = self.game[(j - 1) as usize][(i - 1) as usize];

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
                                2 => ('2', LightGreen, NUM_COLOR),
                                3 => ('3', LightCyan, NUM_COLOR),
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
                    frame.buffer_mut().content[(w * j + i) as usize] = c;
                }
            }

            frame.set_cursor_position(Position {
                x: self.game.input_state.cursor.0 as u16 + 1,
                y: self.game.input_state.cursor.1 as u16 + 1,
            });
        }

        fn handle_crossterm_events(&mut self) -> Result<()> {
            match event::read()? {
                // it's important to check KeyEventKind::Press to avoid handling key release events
                Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key_event(key),
                Event::Mouse(m) => match m.kind {
                    MouseEventKind::Down(button) => 'block: {
                        if !(1..self.game.args.width as u16 + 1).contains(&m.column)
                            || !(1..self.game.args.height as u16 + 1).contains(&m.row)
                        {
                            break 'block;
                        }
                        self.game.input_state.cursor =
                            ((m.column - 1) as usize, (m.row - 1) as usize);
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
                (_, KeyCode::Char('x' | ' ')) => {
                    self.game.input_state.action = Some(ShowTile);
                }
                (_, KeyCode::Char('z')) => {
                    self.game.input_state.action = Some(FlagTile);
                }
                (_, KeyCode::Left) => {
                    self.game.move_cursor(-1, 0);
                }
                (_, KeyCode::Right) => {
                    self.game.move_cursor(1, 0);
                }
                (_, KeyCode::Up) => {
                    self.game.move_cursor(0, -1);
                }
                (_, KeyCode::Down) => {
                    self.game.move_cursor(0, 1);
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
