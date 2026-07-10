pub mod menu;
pub mod render;

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Idle,
    AwaitingName,
    AwaitingExpiry { name: String },
    AwaitingCustomExpiry { name: String },
}
