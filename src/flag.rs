#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Flag {
    Clear,
    Flagged,
    FlaggedMaybe,
}

impl Flag {
    // man this sucks, you'd think compile time sized enums would be a given
    const SIZE: u32 = Self::FlaggedMaybe as u32 + 1;
    pub fn next(self) -> Self {
        // you'd also think "if enum to int is allowed, so is int to enum", well think again
        let next = (self as u32 + 1) % Self::SIZE;
        match next {
            0 => Self::Clear,
            1 => Self::Flagged,
            2 => Self::FlaggedMaybe,
            //purposely not _ so that it breaks if new flags are added
            Self::SIZE.. => Self::Clear, // unreachable due to previous line
        }
    }
}
