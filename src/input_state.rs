use crate::action::Action;

#[derive(Default, Debug)]
pub struct InputState {
    pub cursor: (u16, u16),
    pub action: Option<Action>,
}
