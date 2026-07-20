pub mod handlers;
pub mod menu;
pub mod render;

use crate::vpn::validate::ModifyParam;

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
    AwaitingModifyParam {
        name: String,
    },
    AwaitingModifyValue {
        name: String,
        param: ModifyParam,
    },
}
