#[derive(Copy, Clone, Debug)]
pub enum WinState {
    Untouched,
    Ongoing,
    Lost,
    Won,
}

impl Default for WinState {
    fn default() -> Self {
        Self::Untouched
    }
}
