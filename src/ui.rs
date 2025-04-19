use crate::action::Action::*;
use crate::action::DebugAction::*;
use crate::action::GameCommand::*;
use crate::action::RestartAction::*;
use crate::args::MinesweeperArgs;
use crate::cell_content::CellContent;
use crate::flag::Flag::*;
use crate::input_state::InputState;
use crate::minesweeper::{DisplayText, GameState, Minesweeper};
use crate::tile_visibility::TileVisibility::*;
use crate::util::Sign::*;
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
            display:
                DisplayText {
                    text_top,
                    title,
                    text_bottom,
                    width_digits,
                    height_digits,
                    mines_digits,
                },
            game_state:
                GameState {
                    win_state,
                    cells: _,
                    flagged_cells,
                    closed_empty_cells: _,
                    open_mine_cells: _,
                },
            input_state: InputState { cursor: (x, y), .. },
            ..
        } = &self.game;

        let (title, bottom) = match win_state {
            WinState::Untouched => (
                Line::from(*title).bold().light_blue().centered(),
                Line::from(format!("{}x{},{}", width, height, mines)).centered(),
            ),
            WinState::Won => (
                Line::from(*text_top).bold().light_green().centered(),
                Line::from(*text_bottom).bold().light_green().centered(),
            ),
            WinState::Lost => (
                Line::from(*text_top).bold().light_red().centered(),
                Line::from(*text_bottom).bold().light_red().centered(),
            ),
            _ => {
                let mut stats = format!(
                    "{:mines_digits$}/{} ({:width_digits$},{:height_digits$}) {}x{}",
                    flagged_cells, mines, x, y, width, height
                );
                if stats.len() as u16 > *width {
                    stats = format!("{} {},{}", mines - flagged_cells, x, y);
                }

                (
                    Line::from(*title).bold().light_blue().centered(),
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
                let Some(tile) = self.game.get_tile(i - 1, j - 1) else {
                    continue;
                };

                const HIDDEN_COLOR: Color = Gray;
                const WARN_COLOR: Color = LightYellow;
                const CLEAR_COLOR: Color = Black;

                let (char, fg, bg, modifier) = match tile.visibility {
                    Hidden(f) => match f {
                        Clear => ('#', Black, HIDDEN_COLOR, Modifier::empty()),
                        Flagged => ('!', Black, WARN_COLOR, Modifier::BOLD),
                        FlaggedMaybe => ('?', Black, Yellow, Modifier::BOLD),
                    },
                    Show => match tile.content {
                        CellContent::Empty(n) => match n {
                            0 => (' ', Reset, CLEAR_COLOR, Modifier::empty()),
                            1 => ('1', LightBlue, CLEAR_COLOR, Modifier::empty()),
                            2 => ('2', LightGreen, CLEAR_COLOR, Modifier::empty()),
                            3 => ('3', LightRed, CLEAR_COLOR, Modifier::empty()),
                            4 => ('4', Blue, CLEAR_COLOR, Modifier::empty()),
                            5 => ('5', Red, CLEAR_COLOR, Modifier::empty()),
                            6 => ('6', Cyan, CLEAR_COLOR, Modifier::empty()),
                            7 => ('7', Gray, CLEAR_COLOR, Modifier::empty()),
                            8 => ('8', White, CLEAR_COLOR, Modifier::empty()),
                            _ => unreachable!(),
                        },
                        CellContent::Mine => ('*', Black, LightRed, Modifier::BOLD),
                    },
                };

                let w = frame.area().width;
                let mut c = Cell::new("");
                c.set_char(char).set_fg(fg).set_bg(bg);
                c.modifier = modifier;
                frame.buffer_mut().content[w as usize * j as usize + i as usize] = c;
            }
        }

        let x = self.game.input_state.cursor.0 + 1;
        let y = self.game.input_state.cursor.1 + 1;
        frame.set_cursor_position(Position { x, y });
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
                    let cursor = self.game.input_state.cursor;
                    match button {
                        MouseButton::Left => {
                            self.game.input_state.action = Some(Command(OpenCell(cursor)))
                        }
                        MouseButton::Right | MouseButton::Middle => {
                            self.game.input_state.action = Some(Command(FlagCell(cursor)))
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
        let cursor = self.game.input_state.cursor;

        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('z') | KeyCode::Char('Z')) => {
                self.game.input_state.action = Some(Debug(Undo))
            }
            (KeyModifiers::CONTROL, KeyCode::Char('y') | KeyCode::Char('Y')) => {
                self.game.input_state.action = Some(Debug(Redo))
            }
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
            // Add other key handlers here.
            (_, KeyCode::Char('k')) => {
                self.game.input_state.action = Some(Command(Surrender));
            }
            (_, KeyCode::Char('r')) => {
                self.game.input_state.action = Some(Restart(None));
            }
            (_, KeyCode::Char('n')) => {
                self.game.input_state.action = Some(Restart(Some(IncrementMinesPercent(Positive))));
            }
            (_, KeyCode::Char('p')) => {
                self.game.input_state.action = Some(Restart(Some(IncrementMinesPercent(Negative))));
            }
            (_, KeyCode::Char('x' | ' ')) => {
                self.game.input_state.action = Some(Command(OpenCell(cursor)));
            }
            (_, KeyCode::Char('z' | 'f')) => {
                self.game.input_state.action = Some(Command(FlagCell(cursor)));
            }
            (_, KeyCode::Backspace) => {
                self.game.input_state.action = Some(Command(ClearFlag(cursor)));
            }
            (_, KeyCode::Char('+')) => {
                self.game.input_state.action = Some(Restart(Some(IncrementMines(Positive))));
            }
            (_, KeyCode::Char('-')) => {
                self.game.input_state.action = Some(Restart(Some(IncrementMines(Negative))));
            }
            (modifiers, KeyCode::Right) => {
                if modifiers.contains(KeyModifiers::CONTROL) {
                    self.game.input_state.action = Some(Debug(Redo))
                } else if modifiers.contains(KeyModifiers::SHIFT) {
                    self.game.input_state.action = Some(Restart(Some(ResizeH(Positive))))
                } else {
                    self.game.move_cursor(1, 0)
                }
            }
            (modifiers, KeyCode::Down) => {
                if modifiers.contains(KeyModifiers::SHIFT) {
                    self.game.input_state.action = Some(Restart(Some(ResizeV(Positive))))
                } else {
                    self.game.move_cursor(0, 1)
                }
            }
            (modifiers, KeyCode::Left) => {
                if modifiers.contains(KeyModifiers::CONTROL) {
                    self.game.input_state.action = Some(Debug(Undo))
                } else if modifiers.contains(KeyModifiers::SHIFT) {
                    self.game.input_state.action = Some(Restart(Some(ResizeH(Negative))))
                } else {
                    self.game.move_cursor(-1, 0)
                }
            }
            (modifiers, KeyCode::Up) => {
                if modifiers.contains(KeyModifiers::SHIFT) {
                    self.game.input_state.action = Some(Restart(Some(ResizeV(Negative))))
                } else {
                    self.game.move_cursor(0, -1)
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
