//! Группы, групповые админы и инвайты (issue #20). Принадлежность клиента к
//! группе — колонка clients.group_id; сами клиенты создаёт/удаляет vpn-слой,
//! здесь только атрибуция.

use crate::store::Store;

pub struct GroupRow {
    pub id: i64,
    pub name: String,
    pub max_clients: Option<i64>,
    pub created_at: i64,
}

#[derive(Debug, PartialEq)]
pub enum GroupError {
    /// Имя группы занято (UNIQUE violation).
    NameTaken,
    /// Прочая ошибка БД — уже залогирована.
    Db,
}

/// UNIQUE violation → NameTaken, прочее → Db (с логом) — общий маппинг для
/// create_group/rename_group.
fn map_unique(e: rusqlite::Error, ctx: &str) -> GroupError {
    if let rusqlite::Error::SqliteFailure(f, _) = &e {
        if f.code == rusqlite::ErrorCode::ConstraintViolation {
            return GroupError::NameTaken;
        }
    }
    tracing::error!(error = %e, ctx, "ошибка БД в groups");
    GroupError::Db
}

impl Store {
    pub fn create_group(&self, name: &str, now: i64) -> Result<i64, GroupError> {
        self.with_conn(|c| {
            c.execute(
                "INSERT INTO groups(name, created_at) VALUES(?1, ?2)",
                rusqlite::params![name, now],
            )?;
            Ok(c.last_insert_rowid())
        })
        .map_err(|e| map_unique(e, "create_group"))
    }

    pub fn rename_group(&self, id: i64, name: &str) -> Result<(), GroupError> {
        self.with_conn(|c| {
            c.execute(
                "UPDATE groups SET name=?1 WHERE id=?2",
                rusqlite::params![name, id],
            )
        })
        .map(|_| ())
        .map_err(|e| map_unique(e, "rename_group"))
    }

    pub fn list_groups(&self) -> Vec<GroupRow> {
        self.with_conn(|c| {
            let mut stmt =
                c.prepare("SELECT id, name, max_clients, created_at FROM groups ORDER BY name")?;
            let rows = stmt.query_map([], |r| {
                Ok(GroupRow {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    max_clients: r.get(2)?,
                    created_at: r.get(3)?,
                })
            })?;
            rows.collect()
        })
        .unwrap_or_else(|e| {
            tracing::error!(error = %e, "не удалось прочитать группы");
            Vec::new()
        })
    }

    pub fn group(&self, id: i64) -> Option<GroupRow> {
        self.with_conn(|c| {
            c.query_row(
                "SELECT id, name, max_clients, created_at FROM groups WHERE id=?1",
                [id],
                |r| {
                    Ok(GroupRow {
                        id: r.get(0)?,
                        name: r.get(1)?,
                        max_clients: r.get(2)?,
                        created_at: r.get(3)?,
                    })
                },
            )
        })
        .ok()
    }

    pub fn set_group_quota(&self, id: i64, max: Option<i64>) {
        if let Err(e) = self.with_conn(|c| {
            c.execute(
                "UPDATE groups SET max_clients=?1 WHERE id=?2",
                rusqlite::params![max, id],
            )
        }) {
            tracing::error!(error = %e, id, "не удалось сохранить квоту группы");
        }
    }

    pub fn group_client_count(&self, id: i64) -> i64 {
        self.with_conn(|c| {
            c.query_row(
                "SELECT COUNT(*) FROM clients WHERE group_id=?1 AND removed_at IS NULL",
                [id],
                |r| r.get(0),
            )
        })
        .unwrap_or(0)
    }

    pub fn group_client_names(&self, id: i64) -> Vec<String> {
        self.with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT name FROM clients WHERE group_id=?1 AND removed_at IS NULL ORDER BY name",
            )?;
            let rows = stmt.query_map([id], |r| r.get(0))?;
            rows.collect()
        })
        .unwrap_or_else(|e| {
            tracing::error!(error = %e, id, "не удалось прочитать клиентов группы");
            Vec::new()
        })
    }

    /// None = безлимит; иначе сколько ещё можно создать (не меньше 0).
    pub fn group_remaining(&self, id: i64) -> Option<i64> {
        let max = self.group(id)?.max_clients?;
        Some((max - self.group_client_count(id)).max(0))
    }

    /// Привязка клиента к группе (или отвязка при None). Строка clients может
    /// ещё не существовать (её обычно создаёт collector.ingest) — upsert, не
    /// трогающий first_seen/last_seen существующей строки.
    pub fn assign_client_group(&self, name: &str, group_id: Option<i64>, now: i64) {
        if let Err(e) = self.with_conn(|c| {
            c.execute(
                "INSERT INTO clients(name, ip, first_seen, last_seen, group_id)
                 VALUES(?1, '', ?2, ?2, ?3)
                 ON CONFLICT(name) DO UPDATE SET group_id=?3",
                rusqlite::params![name, now, group_id],
            )
        }) {
            tracing::error!(error = %e, client = name, "не удалось привязать клиента к группе");
        }
    }

    pub fn client_group(&self, name: &str) -> Option<i64> {
        self.with_conn(|c| {
            c.query_row("SELECT group_id FROM clients WHERE name=?1", [name], |r| {
                r.get::<_, Option<i64>>(0)
            })
        })
        .ok()
        .flatten()
    }

    /// Удаление группы: отвязать клиентов, удалить админов и инвайты, затем
    /// саму группу. Удаление VPN-клиентов (если выбрано) делает handler ДО
    /// вызова — здесь только БД.
    pub fn delete_group(&self, id: i64) {
        if let Err(e) = self.with_conn(|c| {
            c.execute("UPDATE clients SET group_id=NULL WHERE group_id=?1", [id])?;
            c.execute("DELETE FROM group_admins WHERE group_id=?1", [id])?;
            c.execute("DELETE FROM invites WHERE group_id=?1", [id])?;
            c.execute("DELETE FROM groups WHERE id=?1", [id])
        }) {
            tracing::error!(error = %e, id, "не удалось удалить группу");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;

    #[test]
    fn migration_v2_applied() {
        let store = Store::open_in_memory();
        assert_eq!(store.schema_version(), 2);
    }

    #[test]
    fn migration_v1_to_v2_preserves_clients() {
        // Честная миграция: файл-БД со схемой v1 и живым клиентом → открытие
        // Store доводит до v2, клиент цел, group_id = NULL.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("awgram.db");
        {
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.execute_batch("CREATE TABLE meta(key TEXT PRIMARY KEY, value TEXT NOT NULL)")
                .unwrap();
            conn.execute_batch(crate::store::MIGRATIONS[0]).unwrap();
            conn.execute(
                "INSERT INTO meta(key,value) VALUES('schema_version','1')",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO clients(name, ip, first_seen, last_seen) VALUES('alice','10.0.0.2',5,9)",
                [],
            )
            .unwrap();
        }
        let store = Store::open(&path).unwrap();
        assert_eq!(store.schema_version(), 2);
        assert_eq!(store.client_group("alice"), None);
        let g = store.create_group("g", 0).unwrap();
        store.assign_client_group("alice", Some(g), 100);
        // first_seen существующей строки не затёрт upsert'ом.
        let first_seen: i64 = store
            .with_conn(|c| {
                c.query_row(
                    "SELECT first_seen FROM clients WHERE name='alice'",
                    [],
                    |r| r.get(0),
                )
            })
            .unwrap();
        assert_eq!(first_seen, 5);
    }

    #[test]
    fn create_list_rename_group() {
        let store = Store::open_in_memory();
        let id = store.create_group("family", 100).unwrap();
        assert_eq!(
            store.create_group("family", 200),
            Err(GroupError::NameTaken)
        );
        let groups = store.list_groups();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "family");
        assert_eq!(groups[0].max_clients, None);
        store.rename_group(id, "home").unwrap();
        assert_eq!(store.group(id).unwrap().name, "home");
    }

    #[test]
    fn rename_to_taken_name_fails() {
        let store = Store::open_in_memory();
        let a = store.create_group("a", 0).unwrap();
        store.create_group("b", 0).unwrap();
        assert_eq!(store.rename_group(a, "b"), Err(GroupError::NameTaken));
    }

    #[test]
    fn quota_and_counts() {
        let store = Store::open_in_memory();
        let id = store.create_group("g", 0).unwrap();
        assert_eq!(store.group_remaining(id), None); // безлимит по умолчанию
        store.set_group_quota(id, Some(2));
        store.assign_client_group("alice", Some(id), 10);
        store.assign_client_group("bob", Some(id), 10);
        assert_eq!(store.group_client_count(id), 2);
        assert_eq!(store.group_remaining(id), Some(0));
        assert_eq!(store.group_client_names(id), vec!["alice", "bob"]);
    }

    #[test]
    fn assign_upserts_and_reassigns() {
        let store = Store::open_in_memory();
        let a = store.create_group("a", 0).unwrap();
        let b = store.create_group("b", 0).unwrap();
        store.assign_client_group("alice", Some(a), 10);
        assert_eq!(store.client_group("alice"), Some(a));
        store.assign_client_group("alice", Some(b), 20);
        assert_eq!(store.client_group("alice"), Some(b));
        store.assign_client_group("alice", None, 30);
        assert_eq!(store.client_group("alice"), None);
    }

    #[test]
    fn removed_clients_not_counted() {
        let store = Store::open_in_memory();
        let id = store.create_group("g", 0).unwrap();
        store.assign_client_group("alice", Some(id), 10);
        store
            .with_conn(|c| c.execute("UPDATE clients SET removed_at=99 WHERE name='alice'", []))
            .unwrap();
        assert_eq!(store.group_client_count(id), 0);
        assert!(store.group_client_names(id).is_empty());
    }

    #[test]
    fn delete_group_detaches_clients() {
        let store = Store::open_in_memory();
        let id = store.create_group("g", 0).unwrap();
        store.assign_client_group("alice", Some(id), 10);
        store.delete_group(id);
        assert!(store.group(id).is_none());
        assert_eq!(store.client_group("alice"), None);
    }
}
