use crate::Flag::*;
use crate::MineState::*;
use crate::Visibility::*;
use rand::RngCore;
use std::fmt::{Debug, Display, Formatter, Result, Write};
use std::ops::{Index, IndexMut, Range};

#[derive(Copy, Clone)]
enum MineState {
    Empty(u8),
    Mine,
}

#[derive(Copy, Clone)]
enum Flag {
    Flag,
    FlagMaybe,
}

#[derive(Copy, Clone)]
enum Visibility {
    Hidden(Option<Flag>),
    Show,
}
#[derive(Copy, Clone)]
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

struct Grid {
    width: usize,
    height: usize,
    mines: Vec<Tile>,
}

impl Display for Grid {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        for line in self.mines.chunks_exact(self.width) {
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
        let mut ret: Self = Self {
            width,
            height,
            mines: vec![
                Tile {
                    mine: Empty(0),
                    visibility: Hidden(None)
                };
                width * height
            ],
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
            self.mines[tile].mine = match (self.mines[tile].mine) {
                Mine => continue,
                _ => Mine,
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
        &self.mines[start..start + self.width]
    }
}
impl IndexMut<usize> for Grid {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let start = index * self.width;
        &mut self.mines[start..start + self.width]
    }
}

fn main() {
    let mut grid = Grid::new(32, 32);
    for i in make_random_holes(32 * 32 * 90 / 100, 0..32 * 32) {
        grid.set_mine(i % 32, i / 32);
    }
    println!("{grid}");
    grid.mines.iter_mut().for_each(|t| (*t).visibility = Show);
    println!("{grid}");
}
