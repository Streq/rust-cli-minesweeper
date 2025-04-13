use crate::cell_content::CellContent;
use crate::cell_content::CellContent::*;
use crate::flag::Flag::*;
use crate::tile_visibility::TileVisibility;
use crate::tile_visibility::TileVisibility::*;
use std::fmt;
use std::fmt::{Display, Formatter, Write};

#[derive(Copy, Clone, Debug, Default)]
pub struct Cell {
    pub visibility: TileVisibility,
    pub content: CellContent,
}

impl Display for Cell {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let c = match self.visibility {
            Hidden(flag) => match flag {
                Clear => '#',
                Flagged => '!',
                FlaggedMaybe => '?',
            },
            Show => match self.content {
                Empty(neighbor_mines) => {
                    if neighbor_mines == 0 {
                        '.'
                    } else {
                        std::char::from_digit(neighbor_mines as u32, 10).unwrap()
                    }
                }
                Mine => '*',
            },
        };

        f.write_char(c)
    }
}
