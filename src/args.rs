use clap::Parser;

/// Command line minesweeper
#[derive(Parser, Copy, Clone, Default, Debug)]
#[command(version, about, long_about = None)]
pub struct MinesweeperArgs {
    /// width
    #[arg(short = 'x', long, default_value_t = 32)]
    pub width: usize,
    /// height
    #[arg(short = 'y', long, default_value_t = 18)]
    pub height: usize,
    /// amount of mines
    #[arg(short, long, default_value_t = 10)]
    pub mines: usize,
}

impl MinesweeperArgs {
    pub fn clamped(mut self) -> Self {
        self.width = self.width.clamp(8, 1000);
        self.height = self.height.clamp(8, 1000);
        let max_mines = self.width * self.height - 9;
        self.mines = self.mines.clamp(1, max_mines);
        self
    }
}
