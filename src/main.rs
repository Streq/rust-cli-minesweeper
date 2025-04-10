use args::MinesweeperArgs;
use clap::Parser;
mod action;
mod args;
mod flag;
mod input_state;
mod minesweeper;
mod tile;
mod tile_content;
mod tile_visibility;
mod ui;
mod util;
mod win_state;

fn main() {
    let args = MinesweeperArgs::parse();
    ui::main(args).unwrap()
}
