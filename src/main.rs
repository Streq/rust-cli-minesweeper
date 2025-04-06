use crate::Flag::*;
use crate::MineState::*;
use crate::Visibility::*;
use rand::RngCore;
use std::cmp::{max, min};
use std::collections::VecDeque;
use std::fmt::{Debug, Display, Formatter, Result, Write};
use std::ops::{Index, IndexMut, Range};

#[derive(Copy, Clone, Debug)]
enum MineState {
    Empty(u8),
    Mine,
}

#[derive(Copy, Clone, Debug)]
enum Flag {
    Flag,
    FlagMaybe,
}

#[derive(Copy, Clone, Debug)]
enum Visibility {
    Hidden(Option<Flag>),
    Show,
}
#[derive(Copy, Clone, Debug)]
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
                visibility: Hidden(Some(Flag)),
                ..
            } => '!',
            Tile {
                visibility: Hidden(Some(FlagMaybe)),
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

#[derive(Debug, Default)]
struct Grid {
    width: usize,
    height: usize,
    tiles: Vec<Tile>,
    lost: bool,
}

impl Display for Grid {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        for line in self.tiles.chunks_exact(self.width) {
            for cell in line {
                write!(f, "{cell}")?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

impl Grid {
    fn new(width: usize, height: usize) -> Self {
        let ret: Self = Self {
            width,
            height,
            tiles: vec![
                Tile {
                    mine: Empty(0),
                    visibility: Hidden(None)
                };
                width * height
            ],
            lost: false,
        };
        ret
    }

    fn set_mine(&mut self, x: usize, y: usize) {
        let tile = &mut self[y][x].mine;

        *tile = match *tile {
            Mine => return,
            _ => Mine,
        };

        let x0 = x.max(1) - 1;
        let x1 = (x + 1).min(self.width - 1);
        let y0 = y.max(1) - 1;
        let y1 = (y + 1).min(self.height - 1);

        for j in y0..=y1 {
            for i in x0..=x1 {
                let mine: &mut MineState = &mut self[j][i].mine;
                match mine {
                    Empty(count) => *count += 1,
                    Mine => continue,
                }
            }
        }
    }
    fn unset_mine(&mut self, x: usize, y: usize) {
        let x0 = x.max(1) - 1;
        let x1 = (x + 1).min(self.width);
        let y0 = y.max(1) - 1;
        let y1 = (y + 1).min(self.height);

        let tile = self[y][x].mine;
        let Mine = tile else { return };
        let mut neighbor_mines = 0;

        for j in y0..=y1 {
            for i in x0..=x1 {
                let mine = &mut self[j][i].mine;
                match mine {
                    Empty(count) => *count -= 1,
                    Mine => neighbor_mines += 1,
                }
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
        let tile = &mut self[y][x];
        if let Show | Hidden(Some(Flag::Flag)) = tile.visibility {
            return;
        }

        if let Mine = tile.mine {
            // explode
            self.lost = true;
            for tile in &mut self.tiles {
                let Mine = tile.mine else { continue };
                tile.visibility = Show;
            }
            return;
        }

        let mut stack = VecDeque::<(usize, usize)>::new();
        stack.push_back((x, y));
        while let Some((x, y)) = stack.pop_back() {
            let tile = &mut self[y][x];

            tile.visibility = Show;
            match tile.mine {
                Empty(0) => {
                    // expand
                    let y0 = max(y, 1) - 1;
                    let y1 = min(y, self.height - 2) + 1;

                    let x0 = max(x, 1) - 1;
                    let x1 = min(x, self.width - 2) + 1;

                    for j in y0..=y1 {
                        for i in x0..=x1 {
                            let tile = &mut self[j][i];
                            let Hidden(_) = tile.visibility else { continue };
                            stack.push_back((i, j))
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

fn make_random_holes(holes_to_make: usize, range: Range<usize>) -> Vec<usize> {
    let mut ret: Vec<usize> = Vec::from_iter(range);
    for _ in 0..holes_to_make {
        let n = rand::rng().next_u32() as usize % ret.len();
        ret.remove(n);
    }
    ret
}

impl Index<usize> for Grid {
    type Output = [Tile];
    fn index(&self, index: usize) -> &Self::Output {
        let start = index * self.width;
        &self.tiles[start..start + self.width]
    }
}
impl IndexMut<usize> for Grid {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let start = index * self.width;
        &mut self.tiles[start..start + self.width]
    }
}

fn main() {
    /*    let mut grid = Grid::new(32, 32);
    for i in make_random_holes(32 * 32 * 90 / 100, 0..32 * 32) {
        grid.set_mine(i % 32, i / 32);
    }
    println!("{grid}");
    grid.mines.iter_mut().for_each(|t| (*t).visibility = Show);
    println!("{grid}");*/

    ui::main().unwrap()
}

mod ui {
    use crate::Visibility::Hidden;
    use crate::{Flag, Grid, MineState, Tile, Visibility, make_random_holes};
    use color_eyre::Result;
    use compact_str::CompactString;
    use crossterm::ExecutableCommand;
    use crossterm::event::{
        self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind,
    };
    use ratatui::buffer::{Buffer, Cell};
    use ratatui::layout::{Margin, Position, Rect};
    use ratatui::style::Color;
    use ratatui::widgets::Widget;
    use ratatui::{
        DefaultTerminal, Frame,
        style::Stylize,
        text::Line,
        widgets::{Block, Paragraph},
    };
    use std::fmt::Display;

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
        cursor: Position,
        clicked: bool,
        grid: Grid,
    }
    impl App {
        /// Construct a new instance of [`App`].
        pub fn new() -> Self {
            Self {
                grid: {
                    let mut grid = Grid::new(32, 32);
                    for i in make_random_holes(32 * 32 * 90 / 100, 0..32 * 32) {
                        grid.set_mine(i % 32, i / 32);
                    }
                    grid
                },
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
            let title = Line::from("Minesweeper!").bold().blue().centered();
            let grid = &self.grid;
            let text = format!("{grid}");
            let area = frame.area().clamp(Rect::new(
                0,
                0,
                grid.width as u16 + 2,
                grid.height as u16 + 2,
            ));

            frame.render_widget(
                Paragraph::new(text)
                    .block(Block::bordered().title(title))
                    .centered(),
                area,
            );

            let subarea = area.inner(Margin::new(1, 1));

            if self.clicked && subarea.contains(self.cursor) {
                let c = self.cursor;
                let x = c.x - 1;
                let y = c.y - 1;
                self.grid.show_tile(x as usize, y as usize);
            }
            if area.height == 0 && area.width == 0 {
                return;
            }

            for j in area.y + 1..area.y + area.height - 1 {
                for i in area.x + 1..area.x + area.width - 1 {
                    let tile = self.grid[(j - 1) as usize][(i - 1) as usize];
                    let cell = match tile {
                        Tile {
                            visibility: Hidden(None),
                            ..
                        } => Cell::new("#"),
                        Tile {
                            visibility: Hidden(Some(Flag::Flag)),
                            ..
                        } => {
                            let mut c = Cell::new("!");
                            c.set_fg(Color::Red);
                            c
                        }
                        Tile {
                            visibility: Hidden(Some(Flag::FlagMaybe)),
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
                            let mut c = Cell::new(" ");
                            c
                        }
                        Tile {
                            mine: MineState::Empty(n),
                            ..
                        } => {
                            let (n, color) = match n {
                                1 => ("1", Color::Blue),
                                2 => ("2", Color::Cyan),
                                3 => ("3", Color::Green),
                                4 => ("4", Color::LightYellow),
                                5 => ("5", Color::Magenta),
                                6 => ("6", Color::LightMagenta),
                                7 => ("7", Color::LightCyan),
                                8 => ("8", Color::Gray),
                                _ => (" ", Color::White),
                            };
                            let mut c = Cell::new(n);
                            c.set_fg(color);

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

            frame.set_cursor_position(self.cursor);
            self.clicked = false;
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
                    MouseEventKind::Down(
                        MouseButton::Left | MouseButton::Right | MouseButton::Middle,
                    ) => {
                        self.cursor = Position::new(m.column, m.row);
                        self.clicked = true;
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
                _ => {}
            }
        }

        /// Set running to false to quit the application.
        fn quit(&mut self) {
            self.running = false;
        }
    }
}
