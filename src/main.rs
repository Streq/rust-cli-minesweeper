use crate::FlagState::*;
use crate::MineState::*;
use crate::Visibility::*;
use crate::WinState::{Lost, Ongoing, Untouched, Won};
use rand::RngCore;
use std::cmp::min;
use std::collections::VecDeque;
use std::fmt::{Debug, Display, Formatter, Result, Write};
use std::iter::Iterator;
use std::ops::{Index, IndexMut, Range};

const DIRS_CARDINAL: [(i8, i8); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];
const DIRS_DIAGONAL: [(i8, i8); 4] = [(1, 1), (-1, 1), (-1, -1), (1, -1)];
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
    Flag,
    FlagMaybe,
}
impl FlagState {
    // man this sucks, you'd think compile time sized enums would be a given
    const SIZE: u32 = FlagMaybe as u32 + 1;
    fn next(self) -> Self {
        // you'd also think "if enum to int is allowed, so is int to enum", well think again
        let next = (self as u32 + 1) % Self::SIZE;
        match next {
            0 => None,
            1 => Flag,
            2 => FlagMaybe,
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
                visibility: Hidden(Flag),
                ..
            } => '!',
            Tile {
                visibility: Hidden(FlagMaybe),
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
    Show,
    Flag,
    Surrender,
    Restart,
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

#[derive(Copy, Clone, Default, Debug)]
struct MinesweeperArgs {
    width: usize,
    height: usize,
    mines: usize,
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
            Action::Show => self.show_tile(x, y),
            Action::Flag => self.flag_tile(x, y),
            Action::Surrender => {
                self.show_all();
                self.win_state = Lost
            }
            Action::Restart => {
                *self = Self::new(self.args);
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

    fn unset_mine(&mut self, x: usize, y: usize) {
        let tile = self[y][x].mine;
        let Mine = tile else { return };
        let mut neighbor_mines = 0;

        for (i, j) in valid_neighbors(&DIRS_8, x, y, self.args.width, self.args.height) {
            let mine = &mut self[j][i].mine;
            match mine {
                Empty(count) => *count -= 1,
                Mine => neighbor_mines += 1,
            }
        }
        self[y][x].mine = Empty(neighbor_mines);
    }

    fn fill_iter<T: Iterator<Item = usize>>(&mut self, mut iterator: T) {
        while let Some(tile) = iterator.next() {
            self.tiles[tile].mine = match (self.tiles[tile].mine) {
                Mine => continue,
                _ => Mine,
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
        if let Show | Hidden(Flag) = tile.visibility {
            return;
        }

        tile.visibility = Show;
        self.shown_tiles += 1;

        if let Mine = tile.mine {
            // explode
            for tile in &mut self.tiles {
                let Mine = tile.mine else { continue };
                tile.visibility = Show;
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
                        tile.visibility = Show;
                        self.shown_tiles += 1;
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
        if let Flag = flag {
            self.flagged_tiles -= 1;
        }

        let mut flag = flag.next();
        if let Flag = flag {
            if self.flagged_tiles == self.args.mines {
                flag = flag.next();
            } else {
                self.flagged_tiles += 1;
            }
        }
        tile.visibility = Hidden(flag);
    }
    pub fn show_all(&mut self) {
        for tile in &mut self.tiles {
            tile.visibility = Show;
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
    ui::main().unwrap()
}

mod ui {
    use crate::FlagState::*;
    use crate::Visibility::*;
    use crate::{Action, MineState, Minesweeper, MinesweeperArgs, Tile, WinState};
    use color_eyre::Result;
    use crossterm::ExecutableCommand;
    use crossterm::event::{
        self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind,
    };
    use ratatui::buffer::Cell;
    use ratatui::layout::{Position, Rect};
    use ratatui::style::Color;
    use ratatui::{
        DefaultTerminal, Frame,
        style::Stylize,
        text::Line,
        widgets::{Block, Paragraph},
    };

    pub fn main() -> color_eyre::Result<()> {
        color_eyre::install()?;
        let terminal = ratatui::init();
        let result = App::new().run(terminal);
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
        pub fn new() -> Self {
            Self {
                game: Minesweeper::new(MinesweeperArgs {
                    width: 32,
                    height: 32,
                    mines: 100,
                }),
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
            let title = match self.game.win_state {
                WinState::Lost => Line::from("You lose!! Press R to restart").bold().red(),
                WinState::Won => Line::from("You win!!! Press R to restart").bold().green(),
                _ => Line::from("Minesweeper!").bold().blue(),
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
            self.game.update();

            if area.height == 0 && area.width == 0 {
                return;
            }

            for j in area.y + 1..area.y + area.height - 1 {
                for i in area.x + 1..area.x + area.width - 1 {
                    let tile = self.game[(j - 1) as usize][(i - 1) as usize];
                    let cell = match tile {
                        Tile {
                            visibility: Hidden(None),
                            ..
                        } => Cell::new("#"),
                        Tile {
                            visibility: Hidden(Flag),
                            ..
                        } => {
                            let mut c = Cell::new("!");
                            c.set_fg(Color::Red);
                            c
                        }
                        Tile {
                            visibility: Hidden(FlagMaybe),
                            ..
                        } => {
                            let mut c = Cell::new("?");
                            c.set_fg(Color::Red);
                            c
                        }
                        Tile {
                            mine: MineState::Empty(0),
                            ..
                        } => {
                            let c = Cell::new(" ");
                            c
                        }
                        Tile {
                            mine: MineState::Empty(n),
                            ..
                        } => {
                            let (n, bg, fg) = match n {
                                1 => ("1", Color::LightBlue, Color::Black),
                                2 => ("2", Color::LightCyan, Color::Black),
                                3 => ("3", Color::LightGreen, Color::Black),
                                4 => ("4", Color::LightYellow, Color::Black),
                                5 => ("5", Color::LightMagenta, Color::Black),
                                6 => ("6", Color::LightBlue, Color::LightYellow),
                                7 => ("7", Color::LightCyan, Color::LightYellow),
                                8 => ("8", Color::LightGreen, Color::LightYellow),
                                _ => (" ", Color::White, Color::Black),
                            };
                            let mut c = Cell::new(n);
                            c.set_bg(bg);
                            c.set_fg(Color::Black);
                            c
                        }
                        Tile {
                            mine: MineState::Mine,
                            ..
                        } => {
                            let mut c = Cell::new("*");
                            c.set_bg(Color::LightRed);
                            c
                        }
                    };

                    let w = frame.area().width;
                    frame.buffer_mut().content[(w * j + i) as usize] = cell;
                }
            }

            frame.set_cursor_position(Position {
                x: self.game.input_state.cursor.0 as u16 + 1,
                y: self.game.input_state.cursor.1 as u16 + 1,
            });
        }

        /// Reads the crossterm events and updates the state of [`App`].
        ///
        /// If your application needs to perform work in between handling events, you can use the
        /// [`event::poll`] function to check if there are any events available with a timeout.
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
                            MouseButton::Left => self.game.input_state.action = Some(Action::Show),
                            MouseButton::Right | MouseButton::Middle => {
                                self.game.input_state.action = Some(Action::Flag)
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
                    self.game.input_state.action = Some(Action::Surrender);
                }
                (_, KeyCode::Char('r')) => {
                    self.game.input_state.action = Some(Action::Restart);
                }
                (_, KeyCode::Char('x')) => {
                    self.game.input_state.action = Some(Action::Show);
                }
                (_, KeyCode::Char('z')) => {
                    self.game.input_state.action = Some(Action::Flag);
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
