//! Журнал событий операций (`events`) — регистрация действий над клиентами
//! (добавление, удаление, regen и т.д.) и выборка истории по клиенту.
//! События online/offline пишет напрямую `ingest` в stats.rs строками —
//! поэтому `EventKind` их не содержит: так никто не залогирует их вручную
//! мимо инварианта переходов состояния в ingest.

use crate::store::Store;

/// Виды операций, логируемых в `events`. online/offline сюда намеренно не
/// входят — см. модульный комментарий.
#[derive(Clone, Copy)]
pub enum EventKind {
    ClientAdd,
    ClientRemove,
    Regen,
    RegenAll,
    Modify,
    Backup,
    BackupDelete,
    Restore,
    Restart,
    Repair,
    GroupCreate,
    GroupDelete,
    GroupRename,
    GroupQuota,
    AdminAdd,
    AdminRemove,
    InviteCreate,
    InviteUse,
    InviteRevoke,
}

impl EventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EventKind::ClientAdd => "client_add",
            EventKind::ClientRemove => "client_remove",
            EventKind::Regen => "regen",
            EventKind::RegenAll => "regen_all",
            EventKind::Modify => "modify",
            EventKind::Backup => "backup",
            EventKind::BackupDelete => "backup_delete",
            EventKind::Restore => "restore",
            EventKind::Restart => "restart",
            EventKind::Repair => "repair",
            EventKind::GroupCreate => "group_create",
            EventKind::GroupDelete => "group_delete",
            EventKind::GroupRename => "group_rename",
            EventKind::GroupQuota => "group_quota",
            EventKind::AdminAdd => "admin_add",
            EventKind::AdminRemove => "admin_remove",
            EventKind::InviteCreate => "invite_create",
            EventKind::InviteUse => "invite_use",
            EventKind::InviteRevoke => "invite_revoke",
        }
    }
}

/// Строка журнала. `kind` — String, а не EventKind: в выборку попадают и
/// 'online'/'offline', которых нет в enum.
pub struct EventRow {
    pub ts: i64,
    pub kind: String,
    pub client: Option<String>,
    pub actor: Option<i64>,
    pub details: Option<String>,
}

impl Store {
    /// Best-effort: ошибка пишется в лог и не прерывает вызывающего.
    pub fn log_event(
        &self,
        ts: i64,
        kind: EventKind,
        client: Option<&str>,
        actor: Option<i64>,
        details: Option<&str>,
    ) {
        let res = self.with_conn(|c| {
            c.execute(
                "INSERT INTO events(ts, kind, client, actor, details) VALUES(?1,?2,?3,?4,?5)",
                rusqlite::params![ts, kind.as_str(), client, actor, details],
            )
        });
        if let Err(e) = res {
            tracing::error!(error = %e, kind = kind.as_str(), "не удалось записать событие");
        }
    }

    /// События клиента (включая online/offline из ingest), новые сверху.
    pub fn client_events(&self, name: &str, limit: usize) -> Vec<EventRow> {
        let res = self.with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT ts, kind, client, actor, details FROM events
                 WHERE client=?1 ORDER BY ts DESC, id DESC LIMIT ?2",
            )?;
            let rows = stmt.query_map(rusqlite::params![name, limit as i64], |r| {
                Ok(EventRow {
                    ts: r.get(0)?,
                    kind: r.get(1)?,
                    client: r.get(2)?,
                    actor: r.get(3)?,
                    details: r.get(4)?,
                })
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        });
        res.unwrap_or_else(|e| {
            tracing::error!(error = %e, client = name, "не удалось прочитать события клиента");
            Vec::new()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;

    #[test]
    fn log_and_read_client_events_newest_first() {
        let store = Store::open_in_memory();
        store.log_event(100, EventKind::ClientAdd, Some("alice"), Some(42), None);
        store.log_event(200, EventKind::Regen, Some("alice"), Some(42), None);
        store.log_event(300, EventKind::Backup, None, Some(42), None); // без клиента — не попадёт
        let events = store.client_events("alice", 10);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].kind, "regen");
        assert_eq!(events[0].actor, Some(42));
        assert_eq!(events[1].kind, "client_add");
    }

    #[test]
    fn client_events_respects_limit() {
        let store = Store::open_in_memory();
        for i in 0..5 {
            store.log_event(i, EventKind::Regen, Some("alice"), None, None);
        }
        assert_eq!(store.client_events("alice", 3).len(), 3);
    }

    #[test]
    fn group_event_kinds_have_stable_strings() {
        assert_eq!(EventKind::GroupCreate.as_str(), "group_create");
        assert_eq!(EventKind::GroupDelete.as_str(), "group_delete");
        assert_eq!(EventKind::GroupRename.as_str(), "group_rename");
        assert_eq!(EventKind::GroupQuota.as_str(), "group_quota");
        assert_eq!(EventKind::AdminAdd.as_str(), "admin_add");
        assert_eq!(EventKind::AdminRemove.as_str(), "admin_remove");
        assert_eq!(EventKind::InviteCreate.as_str(), "invite_create");
        assert_eq!(EventKind::InviteUse.as_str(), "invite_use");
        assert_eq!(EventKind::InviteRevoke.as_str(), "invite_revoke");
    }
}
