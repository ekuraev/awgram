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
    // --- массовая генерация (отдельные state, не перегружают одиночные) ---
    AwaitingBulkPrefix,
    AwaitingBulkCount {
        prefix: String,
    },
    AwaitingBulkExpiry {
        prefix: String,
        count: usize,
    },
    AwaitingBulkCustomExpiry {
        prefix: String,
        count: usize,
    },
    AwaitingBulkPsk {
        prefix: String,
        count: usize,
        expires: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::State;

    #[test]
    fn bulk_state_variants_exist() {
        let _ = State::AwaitingBulkPrefix;
        let _ = State::AwaitingBulkCount {
            prefix: "user".into(),
        };
        let _ = State::AwaitingBulkExpiry {
            prefix: "user".into(),
            count: 10,
        };
        let _ = State::AwaitingBulkCustomExpiry {
            prefix: "user".into(),
            count: 10,
        };
        let _ = State::AwaitingBulkPsk {
            prefix: "user".into(),
            count: 10,
            expires: None,
        };
    }
}
