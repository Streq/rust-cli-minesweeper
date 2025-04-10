use clap::Parser;

/// Command line minesweeper
#[derive(Parser, Copy, Clone, Default, Debug)]
#[command(version, about, long_about = None)]
pub struct MinesweeperArgs {
    /// width
    #[arg(short = 'x', long, default_value_t = 32)]
    pub width: u16,
    /// height
    #[arg(short = 'y', long, default_value_t = 16)]
    pub height: u16,
    /// amount of mines
    #[arg(short, long, default_value_t = 100)]
    pub mines: u32,
}

impl MinesweeperArgs {
    pub fn clamped(mut self) -> Self {
        self.width = self.width.clamp(8, 256);
        self.height = self.height.clamp(8, 256);
        let max_mines = self.width as u32 * self.height as u32 - 9;
        self.mines = self.mines.clamp(1, max_mines);
        self
    }
}
