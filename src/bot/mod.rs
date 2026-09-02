pub mod handlers;
pub mod menu;
pub mod render;

use teloxide::types::MessageId;

use crate::vpn::validate::{ModifyParam, RouteSelection};

/// Куда применить выбранные маршруты: в создаваемого клиента (со всеми уже
/// собранными параметрами диалога) или в существующего. Один экран обслуживает
/// оба сценария, различие живёт здесь, а не в дублирующихся State.
#[derive(Clone)]
pub enum RouteCtx {
    Create {
        name: String,
        expires: Option<String>,
        recreate: bool,
        psk: bool,
    },
    Edit {
        name: String,
    },
}

impl RouteCtx {
    pub fn name(&self) -> &str {
        match self {
            RouteCtx::Create { name, .. } | RouteCtx::Edit { name } => name,
        }
    }

    pub fn is_create(&self) -> bool {
        matches!(self, RouteCtx::Create { .. })
    }
}

/// `prompt` в состояниях текстового ввода — id сообщения с заданным вопросом.
/// Ответ бота редактирует именно его, а не добавляет новое: диалог живёт в
/// одном сообщении, и в чате не копятся мёртвые экраны с кнопками.
#[derive(Clone, Default)]
pub enum State {
    #[default]
    Idle,
    AwaitingName {
        prompt: MessageId,
    },
    AwaitingExpiry {
        name: String,
        recreate: bool,
    },
    AwaitingCustomExpiry {
        name: String,
        recreate: bool,
        prompt: MessageId,
    },
    AwaitingPsk {
        name: String,
        expires: Option<String>,
        recreate: bool,
    },
    // --- маршруты клиента (AllowedIPs): экран пресетов и ручной ввод ---
    AwaitingRoutes {
        ctx: RouteCtx,
        sel: RouteSelection,
        subnet: Option<String>,
        current: Option<String>,
    },
    AwaitingRoutesCustom {
        ctx: RouteCtx,
        prompt: MessageId,
    },
    AwaitingModifyParam {
        name: String,
        prompt: MessageId,
    },
    AwaitingModifyValue {
        name: String,
        param: ModifyParam,
        prompt: MessageId,
    },
    // --- массовая генерация (отдельные state, не перегружают одиночные) ---
    AwaitingBulkPrefix {
        prompt: MessageId,
    },
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
        prompt: MessageId,
    },
    AwaitingBulkPsk {
        prefix: String,
        count: usize,
        expires: Option<String>,
    },
    // --- группы (#20): диалоги владельца ---
    AwaitingGroupName {
        prompt: MessageId,
    },
    AwaitingGroupRename {
        id: i64,
        prompt: MessageId,
    },
    AwaitingGroupQuota {
        id: i64,
        prompt: MessageId,
    },
    AwaitingGroupAdminId {
        id: i64,
        prompt: MessageId,
    },
}

#[cfg(test)]
mod tests {
    use super::{RouteCtx, State};
    use teloxide::types::MessageId;

    #[test]
    fn bulk_state_variants_exist() {
        let _ = State::AwaitingBulkPrefix {
            prompt: MessageId(1),
        };
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
            prompt: MessageId(1),
        };
        let _ = State::AwaitingBulkPsk {
            prefix: "user".into(),
            count: 10,
            expires: None,
        };
    }

    #[test]
    fn route_ctx_exposes_name_and_mode() {
        let create = RouteCtx::Create {
            name: "alice".into(),
            expires: None,
            recreate: false,
            psk: true,
        };
        assert_eq!(create.name(), "alice");
        assert!(create.is_create());
        let edit = RouteCtx::Edit { name: "bob".into() };
        assert_eq!(edit.name(), "bob");
        assert!(!edit.is_create());
    }
}
