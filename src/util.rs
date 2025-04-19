use crate::action::Cursor;
use rand::RngCore;
use std::collections::BTreeSet;

pub const DIRS_8: [(i8, i8); 8] = [
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
];
pub const DIRS_9: [(i8, i8); 9] = [
    (0, 0),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
];

#[derive(Copy, Clone, Debug)]
pub enum Sign {
    Negative = -1,
    Positive = 1,
}

pub fn xy_i((x, y): Cursor, w: u16, h: u16) -> Option<usize> {
    if w <= x || h <= y {
        None
    } else {
        Some(y as usize * w as usize + x as usize)
    }
}

pub fn i_xy(index: usize, w: u16, h: u16) -> Option<Cursor> {
    let ws = w as usize;
    let hs = h as usize;
    if index >= (ws * hs) {
        None
    } else {
        Some(((index % ws) as u16, (index / ws) as u16))
    }
}

pub fn valid_neighbors(
    dirs: &[(i8, i8)],
    (x, y): Cursor,
    w: u16,
    h: u16,
) -> impl Iterator<Item = Cursor> {
    dirs.iter()
        .map(|(dx, dy)| (*dx as i16, *dy as i16))
        .map(move |(dx, dy)| (x.saturating_add_signed(dx), y.saturating_add_signed(dy)))
        .filter(move |(i, j)| w > *i && h > *j)
}

pub fn fill_random<T: PartialEq + Copy>(
    whitelisted: impl Iterator<Item = usize>,
    size: usize,
    fills: usize,
    init_value: T,
    value: T,
) -> Vec<T> {
    let mut whitelisted: BTreeSet<usize> = BTreeSet::from_iter(whitelisted);
    let (fills, init_value, value, flip) = if fills > size / 2 {
        (size - fills - whitelisted.len(), value, init_value, true)
    } else {
        (fills, init_value, value, false)
    };

    let mut ret = vec![init_value; size];

    if flip {
        for wl in &whitelisted {
            ret[*wl] = value
        }
    }

    for _ in 0..fills {
        let mut r = rand::rng().next_u32() as usize % (ret.len() - whitelisted.len());

        for wl in whitelisted.iter() {
            r = if *wl <= r { r + 1 } else { break };
        }
        ret[r] = value;
        whitelisted.insert(r);
    }

    ret
}
