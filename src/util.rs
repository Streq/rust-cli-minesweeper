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
pub enum Unit {
    Negative = -1,
    Zero = 0,
    Positive = 1,
}

#[derive(Copy, Clone, Debug)]
pub enum Sign {
    Negative = -1,
    Positive = 1,
}
