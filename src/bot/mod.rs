pub mod handlers;
pub mod menu;
pub mod render;

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Idle,
    AwaitingName,
    AwaitingExpiry {
        name: String,
        recreate: bool,
    },
    AwaitingCustomExpiry {
        name: String,
        recreate: bool,
    },
    AwaitingPsk {
        name: String,
        expires: Option<String>,
        recreate: bool,
    },
}
