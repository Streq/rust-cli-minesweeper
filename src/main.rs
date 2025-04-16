use args::MinesweeperArgs;
use clap::Parser;
mod action;
mod args;
mod cell;
mod cell_content;
mod diff;
mod flag;
mod input_state;
mod minesweeper;
mod tile_visibility;
mod ui;
mod util;
mod win_state;

fn main() {
    let args = MinesweeperArgs::parse();
    ui::main(args).unwrap()
}
