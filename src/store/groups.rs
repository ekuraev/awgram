//! Группы, групповые админы и инвайты (issue #20). Принадлежность клиента к
//! группе — колонка clients.group_id; сами клиенты создаёт/удаляет vpn-слой,
//! здесь только атрибуция.

use crate::store::Store;

/// Время жизни инвайта: 24 часа.
pub const INVITE_TTL_SECS: i64 = 86_400;

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
    /// Группы с таким id нет (удалили за время диалога).
    NotFound,
    /// Прочая ошибка БД — уже залогирована.
    Db,
}

/// Итог атомарной привязки с проверкой квоты.
#[derive(Debug, PartialEq)]
pub enum QuotaAssign {
    /// Привязан (или уже был живым клиентом этой группы — no-op).
    Assigned,
    /// Квота исчерпана либо группы уже нет: привязка не выполнена.
    Full,
    /// Ошибка БД (залогирована): привязка не выполнена.
    Db,
}

pub struct InviteRow {
    pub token: String,
    pub group_id: i64,
    pub expires_at: i64,
}

/// Скоуп списка клиентов: все / без группы / конкретная группа.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ListScope {
    All,
    NoGroup,
    Group(i64),
}

impl ListScope {
    /// Пропускает ли скоуп клиента с данной группой.
    pub fn admits(self, group: Option<i64>) -> bool {
        match self {
            ListScope::All => true,
            ListScope::NoGroup => group.is_none(),
            ListScope::Group(g) => group == Some(g),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum InviteUse {
    /// Пользователь стал админом группы.
    Joined(i64),
    /// Уже был админом этой группы (инвайт всё равно потрачен).
    AlreadyAdmin(i64),
    /// Токен не найден / истёк / использован / отозван.
    Invalid,
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

/// 26 случайных символов a-z0-9 (~134 бита) — как gen_slug, но длиннее.
/// Payload "inv_<token>" укладывается в лимит 64 символа start-параметра.
pub fn gen_invite_token() -> String {
    const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    (0..26)
        .map(|_| CHARS[rand::random_range(0..CHARS.len())] as char)
        .collect()
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
        match self.with_conn(|c| {
            c.execute(
                "UPDATE groups SET name=?1 WHERE id=?2",
                rusqlite::params![name, id],
            )
        }) {
            Ok(0) => Err(GroupError::NotFound),
            Ok(_) => Ok(()),
            Err(e) => Err(map_unique(e, "rename_group")),
        }
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

    /// false — группы нет (или ошибка БД): квота не сохранена.
    pub fn set_group_quota(&self, id: i64, max: Option<i64>) -> bool {
        match self.with_conn(|c| {
            c.execute(
                "UPDATE groups SET max_clients=?1 WHERE id=?2",
                rusqlite::params![max, id],
            )
        }) {
            Ok(n) => n > 0,
            Err(e) => {
                tracing::error!(error = %e, id, "не удалось сохранить квоту группы");
                false
            }
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
    /// трогающий first_seen/last_seen существующей строки. Если клиент был помечен
    /// удалённым (removed_at IS NOT NULL), привязка «оживляет» его: снимает removed_at
    /// так, что живой клиент сразу считается в квоте группы, не дожидаясь тика коллектора.
    pub fn assign_client_group(&self, name: &str, group_id: Option<i64>, now: i64) {
        if let Err(e) = self.with_conn(|c| {
            c.execute(
                "INSERT INTO clients(name, ip, first_seen, last_seen, group_id)
                 VALUES(?1, '', ?2, ?2, ?3)
                 ON CONFLICT(name) DO UPDATE SET group_id=?3, removed_at=NULL",
                rusqlite::params![name, now, group_id],
            )
        }) {
            tracing::error!(error = %e, client = name, "не удалось привязать клиента к группе");
        }
    }

    /// Атомарный вариант assign_client_group для путей, где действует квота:
    /// проверка лимита и upsert выполняются в одном замыкании with_conn, то
    /// есть под одним захватом мьютекса соединения — конкурирующий вызов
    /// увидит уже обновлённый счётчик (фикс TOCTOU-гонки квоты).
    pub fn try_assign_client_group(&self, name: &str, group_id: i64, now: i64) -> QuotaAssign {
        use rusqlite::OptionalExtension;
        let res = self.with_conn(|c| {
            // Живой клиент уже в этой группе → no-op: перенос в свою группу
            // и гонки recreate не блокируются полнотой группы.
            let already: Option<Option<i64>> = c
                .query_row(
                    "SELECT group_id FROM clients WHERE name=?1 AND removed_at IS NULL",
                    [name],
                    |r| r.get(0),
                )
                .optional()?;
            if already.flatten() == Some(group_id) {
                return Ok(true);
            }
            // Нет строки группы → отказ (Full-класс): группу удалили за время
            // диалога, привязывать не к чему.
            let max: Option<Option<i64>> = c
                .query_row(
                    "SELECT max_clients FROM groups WHERE id=?1",
                    [group_id],
                    |r| r.get(0),
                )
                .optional()?;
            let Some(max) = max else { return Ok(false) };
            if let Some(max) = max {
                let count: i64 = c.query_row(
                    "SELECT COUNT(*) FROM clients WHERE group_id=?1 AND removed_at IS NULL",
                    [group_id],
                    |r| r.get(0),
                )?;
                if count >= max {
                    return Ok(false);
                }
            }
            c.execute(
                "INSERT INTO clients(name, ip, first_seen, last_seen, group_id)
                 VALUES(?1, '', ?2, ?2, ?3)
                 ON CONFLICT(name) DO UPDATE SET group_id=?3, removed_at=NULL",
                rusqlite::params![name, now, group_id],
            )?;
            Ok(true)
        });
        match res {
            Ok(true) => QuotaAssign::Assigned,
            Ok(false) => QuotaAssign::Full,
            Err(e) => {
                tracing::error!(error = %e, client = name, group_id, "не удалось атомарно привязать клиента к группе");
                QuotaAssign::Db
            }
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

    /// true — добавлен; false — уже был админом этой группы.
    pub fn add_group_admin(&self, group_id: i64, user_id: i64, added_by: i64, now: i64) -> bool {
        self.with_conn(|c| {
            c.execute(
                "INSERT OR IGNORE INTO group_admins(group_id, user_id, added_at, added_by)
                 VALUES(?1, ?2, ?3, ?4)",
                rusqlite::params![group_id, user_id, now, added_by],
            )
        })
        .map(|n| n > 0)
        .unwrap_or_else(|e| {
            tracing::error!(error = %e, group_id, user_id, "не удалось добавить админа группы");
            false
        })
    }

    pub fn remove_group_admin(&self, group_id: i64, user_id: i64) {
        if let Err(e) = self.with_conn(|c| {
            c.execute(
                "DELETE FROM group_admins WHERE group_id=?1 AND user_id=?2",
                rusqlite::params![group_id, user_id],
            )
        }) {
            tracing::error!(error = %e, group_id, user_id, "не удалось удалить админа группы");
        }
    }

    pub fn group_admin_ids(&self, group_id: i64) -> Vec<i64> {
        self.with_conn(|c| {
            let mut stmt =
                c.prepare("SELECT user_id FROM group_admins WHERE group_id=?1 ORDER BY added_at")?;
            let rows = stmt.query_map([group_id], |r| r.get(0))?;
            rows.collect()
        })
        .unwrap_or_default()
    }

    /// Группы пользователя, отсортированные по имени группы — стабильный
    /// порядок для меню выбора.
    pub fn admin_group_ids(&self, user_id: i64) -> Vec<i64> {
        self.with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT ga.group_id FROM group_admins ga
                 JOIN groups g ON g.id = ga.group_id
                 WHERE ga.user_id=?1 ORDER BY g.name",
            )?;
            let rows = stmt.query_map([user_id], |r| r.get(0))?;
            rows.collect()
        })
        .unwrap_or_default()
    }

    pub fn has_any_group_admin(&self) -> bool {
        self.with_conn(|c| {
            c.query_row("SELECT EXISTS(SELECT 1 FROM group_admins)", [], |r| {
                r.get::<_, i64>(0)
            })
        })
        .map(|v| v > 0)
        .unwrap_or(false)
    }

    /// Создаёт инвайт, отзывая прежний активный (у группы максимум один живой
    /// инвайт — иначе владелец теряет контроль над тем, какая ссылка ходит по
    /// рукам). None — ошибка БД: ссылки нет, «успех» не показываем.
    pub fn create_invite(&self, group_id: i64, created_by: i64, now: i64) -> Option<String> {
        let token = gen_invite_token();
        match self.with_conn(|c| {
            c.execute(
                "DELETE FROM invites WHERE group_id=?1 AND used_by IS NULL",
                [group_id],
            )?;
            c.execute(
                "INSERT INTO invites(token, group_id, created_by, created_at, expires_at)
                 VALUES(?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![token, group_id, created_by, now, now + INVITE_TTL_SECS],
            )
        }) {
            Ok(_) => Some(token),
            Err(e) => {
                tracing::error!(error = %e, group_id, "не удалось создать инвайт");
                None
            }
        }
    }

    pub fn active_invite(&self, group_id: i64, now: i64) -> Option<InviteRow> {
        self.with_conn(|c| {
            c.query_row(
                "SELECT token, group_id, expires_at FROM invites
                 WHERE group_id=?1 AND used_by IS NULL AND expires_at > ?2",
                rusqlite::params![group_id, now],
                |r| {
                    Ok(InviteRow {
                        token: r.get(0)?,
                        group_id: r.get(1)?,
                        expires_at: r.get(2)?,
                    })
                },
            )
        })
        .ok()
    }

    pub fn revoke_invite(&self, group_id: i64) {
        if let Err(e) = self.with_conn(|c| {
            c.execute(
                "DELETE FROM invites WHERE group_id=?1 AND used_by IS NULL",
                [group_id],
            )
        }) {
            tracing::error!(error = %e, group_id, "не удалось отозвать инвайт");
        }
    }

    /// Атомарно: UPDATE помечает токен использованным только если он жив;
    /// изменённых строк 0 → Invalid. Затем INSERT в group_admins решает
    /// Joined vs AlreadyAdmin. Все три запроса — под одним захватом мьютекса
    /// соединения: токен не может сгореть без добавления админа.
    pub fn use_invite(&self, token: &str, user_id: i64, now: i64) -> InviteUse {
        let res = self.with_conn(|c| {
            let n = c.execute(
                "UPDATE invites SET used_by=?1, used_at=?2
                 WHERE token=?3 AND used_by IS NULL AND expires_at > ?2",
                rusqlite::params![user_id, now, token],
            )?;
            if n == 0 {
                return Ok(None);
            }
            let g = c.query_row(
                "SELECT group_id FROM invites WHERE token=?1",
                [token],
                |r| r.get::<_, i64>(0),
            )?;
            let added = c.execute(
                "INSERT OR IGNORE INTO group_admins(group_id, user_id, added_at, added_by)
                 VALUES(?1, ?2, ?3, ?4)",
                rusqlite::params![g, user_id, now, user_id],
            )?;
            Ok(Some((g, added > 0)))
        });
        match res {
            Ok(Some((g, true))) => InviteUse::Joined(g),
            Ok(Some((g, false))) => InviteUse::AlreadyAdmin(g),
            Ok(None) => InviteUse::Invalid,
            Err(e) => {
                tracing::error!(error = %e, "не удалось применить инвайт");
                InviteUse::Invalid
            }
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
    fn rename_missing_group_is_not_found() {
        // Группу могли удалить, пока владелец вводил новое имя, — UPDATE
        // по 0 строк не должен выглядеть успехом.
        let store = Store::open_in_memory();
        assert_eq!(store.rename_group(999, "x"), Err(GroupError::NotFound));
    }

    #[test]
    fn set_quota_on_missing_group_is_false() {
        let store = Store::open_in_memory();
        assert!(!store.set_group_quota(999, Some(5)));
        let id = store.create_group("g", 0).unwrap();
        assert!(store.set_group_quota(id, Some(5)));
        assert_eq!(store.group(id).unwrap().max_clients, Some(5));
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

    #[test]
    fn add_and_remove_group_admin() {
        let store = Store::open_in_memory();
        let g = store.create_group("g", 0).unwrap();
        assert!(!store.has_any_group_admin());
        assert!(store.add_group_admin(g, 42, 1, 10));
        assert!(!store.add_group_admin(g, 42, 1, 20)); // повторно — false
        assert!(store.has_any_group_admin());
        assert_eq!(store.group_admin_ids(g), vec![42]);
        assert_eq!(store.admin_group_ids(42), vec![g]);
        store.remove_group_admin(g, 42);
        assert!(store.group_admin_ids(g).is_empty());
        assert!(store.admin_group_ids(42).is_empty());
    }

    #[test]
    fn admin_of_multiple_groups_sorted_by_group_name() {
        let store = Store::open_in_memory();
        let b = store.create_group("beta", 0).unwrap();
        let a = store.create_group("alfa", 0).unwrap();
        store.add_group_admin(b, 7, 1, 10);
        store.add_group_admin(a, 7, 1, 10);
        assert_eq!(store.admin_group_ids(7), vec![a, b]);
    }

    #[test]
    fn invite_roundtrip_joined() {
        let store = Store::open_in_memory();
        let g = store.create_group("g", 0).unwrap();
        let token = store.create_invite(g, 1, 100).unwrap();
        assert_eq!(token.len(), 26);
        assert!(store.active_invite(g, 150).is_some());
        assert_eq!(store.use_invite(&token, 42, 200), InviteUse::Joined(g));
        assert_eq!(store.admin_group_ids(42), vec![g]);
        // одноразовость: повторное использование — Invalid
        assert_eq!(store.use_invite(&token, 43, 300), InviteUse::Invalid);
        // использованный инвайт больше не активен
        assert!(store.active_invite(g, 300).is_none());
    }

    #[test]
    fn invite_expired_is_invalid() {
        let store = Store::open_in_memory();
        let g = store.create_group("g", 0).unwrap();
        let token = store.create_invite(g, 1, 100).unwrap();
        let too_late = 100 + super::INVITE_TTL_SECS + 1;
        assert_eq!(store.use_invite(&token, 42, too_late), InviteUse::Invalid);
        assert!(store.active_invite(g, too_late).is_none());
    }

    #[test]
    fn invite_for_existing_admin_reports_already() {
        let store = Store::open_in_memory();
        let g = store.create_group("g", 0).unwrap();
        store.add_group_admin(g, 42, 1, 10);
        let token = store.create_invite(g, 1, 100).unwrap();
        assert_eq!(
            store.use_invite(&token, 42, 200),
            InviteUse::AlreadyAdmin(g)
        );
        // инвайт при этом потрачен
        assert_eq!(store.use_invite(&token, 43, 250), InviteUse::Invalid);
    }

    #[test]
    fn new_invite_revokes_previous() {
        let store = Store::open_in_memory();
        let g = store.create_group("g", 0).unwrap();
        let first = store.create_invite(g, 1, 100).unwrap();
        let second = store.create_invite(g, 1, 200).unwrap();
        assert_eq!(store.use_invite(&first, 42, 250), InviteUse::Invalid);
        assert_eq!(store.use_invite(&second, 42, 250), InviteUse::Joined(g));
    }

    #[test]
    fn revoke_invite_kills_active() {
        let store = Store::open_in_memory();
        let g = store.create_group("g", 0).unwrap();
        let token = store.create_invite(g, 1, 100).unwrap();
        store.revoke_invite(g);
        assert!(store.active_invite(g, 150).is_none());
        assert_eq!(store.use_invite(&token, 42, 150), InviteUse::Invalid);
    }

    #[test]
    fn create_invite_none_on_db_error() {
        // При ошибке БД токен не возвращается — иначе владелец получил бы
        // «успешную» ссылку, которая никогда не сработает.
        let store = Store::open_in_memory();
        let g = store.create_group("g", 0).unwrap();
        store
            .with_conn(|c| c.execute("DROP TABLE invites", []))
            .unwrap();
        assert_eq!(store.create_invite(g, 1, 100), None);
    }

    #[test]
    fn gen_invite_token_charset() {
        let t = super::gen_invite_token();
        assert_eq!(t.len(), 26);
        assert!(t
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
    }

    #[test]
    fn try_assign_respects_quota() {
        let store = Store::open_in_memory();
        let g = store.create_group("g", 0).unwrap();
        store.set_group_quota(g, Some(2));
        assert_eq!(
            store.try_assign_client_group("a", g, 10),
            QuotaAssign::Assigned
        );
        assert_eq!(
            store.try_assign_client_group("b", g, 10),
            QuotaAssign::Assigned
        );
        // Третий не влезает; привязка не выполнена.
        assert_eq!(store.try_assign_client_group("c", g, 10), QuotaAssign::Full);
        assert_eq!(store.group_client_count(g), 2);
        assert_eq!(store.client_group("c"), None);
    }

    #[test]
    fn try_assign_already_in_group_is_noop_even_when_full() {
        // Перенос в свою же группу / гонка recreate: клиент уже внутри —
        // полная группа не должна его «выталкивать».
        let store = Store::open_in_memory();
        let g = store.create_group("g", 0).unwrap();
        store.set_group_quota(g, Some(1));
        assert_eq!(
            store.try_assign_client_group("a", g, 10),
            QuotaAssign::Assigned
        );
        assert_eq!(
            store.try_assign_client_group("a", g, 20),
            QuotaAssign::Assigned
        );
        assert_eq!(store.group_client_count(g), 1);
    }

    #[test]
    fn try_assign_unlimited_group() {
        let store = Store::open_in_memory();
        let g = store.create_group("g", 0).unwrap(); // max_clients = NULL
        for i in 0..5 {
            let name = format!("c{i}");
            assert_eq!(
                store.try_assign_client_group(&name, g, 10),
                QuotaAssign::Assigned
            );
        }
        assert_eq!(store.group_client_count(g), 5);
    }

    #[test]
    fn try_assign_missing_group_is_full() {
        // Группу удалили за время гонки: привязка не выполняется, клиент
        // остаётся как был (Full-класс отказа — см. спеку).
        let store = Store::open_in_memory();
        assert_eq!(
            store.try_assign_client_group("a", 999, 10),
            QuotaAssign::Full
        );
        assert_eq!(store.client_group("a"), None);
    }

    #[test]
    fn try_assign_removed_client_in_group_counts_as_new() {
        // «Воскресшее» имя: строка clients с этим именем существует, но removed_at IS NOT NULL.
        // При успешной привязке removed_at должен быть NULL, и воскрешённый клиент сразу
        // считается в квоте (не дожидаясь тика коллектора).
        let store = Store::open_in_memory();
        let g = store.create_group("g", 0).unwrap();
        store.set_group_quota(g, Some(2));
        store.assign_client_group("dead", Some(g), 10);
        store
            .with_conn(|c| c.execute("UPDATE clients SET removed_at=99 WHERE name='dead'", []))
            .unwrap();

        // Слот свободен (dead не считается, т.к. removed_at=99) — займём его alice.
        assert_eq!(
            store.try_assign_client_group("alice", g, 100),
            QuotaAssign::Assigned
        );

        // Теперь воскресим dead: try_assign возвращает Assigned и очищает removed_at.
        assert_eq!(
            store.try_assign_client_group("dead", g, 200),
            QuotaAssign::Assigned
        );

        // После воскрешения removed_at должен быть NULL сразу (не дожидаясь коллектора).
        let removed_at: Option<i64> = store
            .with_conn(|c| {
                c.query_row(
                    "SELECT removed_at FROM clients WHERE name='dead'",
                    [],
                    |r| r.get(0),
                )
            })
            .unwrap();
        assert_eq!(removed_at, None);

        // Счётчик должен отражать обоих живых клиентов сразу (без тика коллектора).
        assert_eq!(store.group_client_count(g), 2);

        // Квота выбрана: следующий new клиент не влезает (даже без коллектора).
        assert_eq!(
            store.try_assign_client_group("charlie", g, 300),
            QuotaAssign::Full
        );
    }

    #[test]
    fn try_assign_db_error_is_db() {
        let store = Store::open_in_memory();
        let g = store.create_group("g", 0).unwrap();
        store
            .with_conn(|c| c.execute("DROP TABLE clients", []))
            .unwrap();
        assert_eq!(store.try_assign_client_group("a", g, 10), QuotaAssign::Db);
    }

    #[test]
    fn try_assign_concurrent_never_overshoots() {
        // Гонка N потоков за M слотов: ровно M привязок. Провал этого теста до
        // фикса — сам смысл задачи (старый check-then-act перелетал квоту).
        use std::sync::Arc;
        let store = Arc::new(Store::open_in_memory());
        let g = store.create_group("g", 0).unwrap();
        store.set_group_quota(g, Some(3));
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let store = Arc::clone(&store);
                std::thread::spawn(move || {
                    let name = format!("c{i}");
                    store.try_assign_client_group(&name, g, 10)
                })
            })
            .collect();
        let results: Vec<QuotaAssign> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        let assigned = results
            .iter()
            .filter(|r| **r == QuotaAssign::Assigned)
            .count();
        assert_eq!(assigned, 3);
        assert_eq!(store.group_client_count(g), 3);
    }

    #[test]
    fn try_assign_move_live_client_between_groups() {
        // Перенос живого клиента между группами: попытка положить в полную группу
        // должна вернуть Full и оставить клиента в исходной группе; перенос в
        // группу со свободным слотом должен удаться и изменить счётчики обеих групп.
        let store = Store::open_in_memory();
        let g1 = store.create_group("g1", 0).unwrap();
        let g2 = store.create_group("g2", 0).unwrap();

        store.set_group_quota(g1, Some(1));
        store.set_group_quota(g2, Some(1));

        // Привяжем клиента alice к g1
        assert_eq!(
            store.try_assign_client_group("alice", g1, 10),
            QuotaAssign::Assigned
        );
        assert_eq!(store.group_client_count(g1), 1);
        assert_eq!(store.group_client_count(g2), 0);

        // Заполним g2
        assert_eq!(
            store.try_assign_client_group("bob", g2, 20),
            QuotaAssign::Assigned
        );
        assert_eq!(store.group_client_count(g2), 1);

        // Попытка перенести alice в полную g2 должна вернуть Full и оставить alice в g1
        assert_eq!(
            store.try_assign_client_group("alice", g2, 30),
            QuotaAssign::Full
        );
        assert_eq!(store.client_group("alice"), Some(g1));
        assert_eq!(store.group_client_count(g1), 1);
        assert_eq!(store.group_client_count(g2), 1);

        // Освободим g2 перемещением bob
        let g3 = store.create_group("g3", 0).unwrap();
        assert_eq!(
            store.try_assign_client_group("bob", g3, 40),
            QuotaAssign::Assigned
        );
        assert_eq!(store.group_client_count(g2), 0);

        // Теперь перенос alice в g2 должен успешно выполниться
        assert_eq!(
            store.try_assign_client_group("alice", g2, 50),
            QuotaAssign::Assigned
        );
        assert_eq!(store.client_group("alice"), Some(g2));
        assert_eq!(store.group_client_count(g1), 0);
        assert_eq!(store.group_client_count(g2), 1);
        assert_eq!(store.group_client_count(g3), 1);
    }
}
