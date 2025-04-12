use crate::action::Action::*;
use crate::args::MinesweeperArgs;
use crate::flag::Flag::*;
use crate::input_state::InputState;
use crate::minesweeper::Minesweeper;
use crate::tile_content::TileContent;
use crate::tile_visibility::TileVisibility::*;
use crate::util::Unit::{Negative, Positive, Zero};
use crate::win_state::WinState;
use color_eyre::Result;
use crossterm::ExecutableCommand;
use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind,
};
use ratatui::buffer::Cell;
use ratatui::layout::{Position, Rect};
use ratatui::style::Color::*;
use ratatui::style::{Color, Modifier};
use ratatui::{
    DefaultTerminal, Frame,
    style::Stylize,
    text::Line,
    widgets::{Block, Paragraph},
};

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = std::io::stdout()
            .execute(event::DisableMouseCapture)
            .unwrap();
        println!("Cleanup in drop.");
    }
}
pub fn main(args: MinesweeperArgs) -> Result<()> {
    let _ = TerminalGuard {};
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

                const HIDDEN_COLOR: Color = Gray;
                const WARN_COLOR: Color = Gray;
                const NUM_COLOR: Color = Black;
                const NUM_COLOR2: Color = Black;

                let (char, fg, bg, modifier) = match tile.visibility {
                    Hidden(f) => match f {
                        None => ('#', Black, HIDDEN_COLOR, Modifier::empty()),
                        Flagged => ('!', Red, LightYellow, Modifier::BOLD),
                        FlaggedMaybe => ('?', Black, WARN_COLOR, Modifier::BOLD),
                    },
                    Show => match tile.content {
                        TileContent::Empty(n) => match n {
                            0 => (' ', Reset, Reset, Modifier::empty()),
                            1 => ('1', LightBlue, NUM_COLOR, Modifier::empty()),
                            2 => ('2', LightGreen, NUM_COLOR, Modifier::empty()),
                            3 => ('3', LightRed, NUM_COLOR, Modifier::empty()),
                            4 => ('4', Blue, NUM_COLOR, Modifier::empty()),
                            5 => ('5', Red, NUM_COLOR2, Modifier::empty()),
                            6 => ('6', Cyan, NUM_COLOR2, Modifier::empty()),
                            7 => ('7', Gray, NUM_COLOR2, Modifier::empty()),
                            8.. => ('8', White, NUM_COLOR2, Modifier::empty()),
                        },
                        TileContent::Mine => ('*', LightRed, Black, Modifier::BOLD),
                    },
                };

                let w = frame.area().width;
                let mut c = Cell::new("");
                c.set_char(char).set_fg(fg).set_bg(bg);
                c.modifier = modifier;
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
                        MouseButton::Left => self.game.input_state.action = Some(OpenCell),
                        MouseButton::Right | MouseButton::Middle => {
                            self.game.input_state.action = Some(FlagCell)
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
                self.game.input_state.action = Some(OpenCell);
            }
            (_, KeyCode::Char('z' | 'f')) => {
                self.game.input_state.action = Some(FlagCell);
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
            (modifiers, key @ (KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down)) => {
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
