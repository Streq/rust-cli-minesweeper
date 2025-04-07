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
    #[arg(short, long, default_value_t = 55)]
    pub mines: usize,
}
