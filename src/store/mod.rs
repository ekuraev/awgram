//! Единственный владелец SQLite-БД бота. Все таблицы, миграции схемы и
//! доступ к соединению живут здесь; наружу — типизированные методы
//! (settings/stats/events в соседних файлах модуля).

use std::path::Path;
use std::sync::Mutex;

use rusqlite::Connection;

mod events;
mod groups;
mod settings;
mod stats;

pub use events::{EventKind, EventRow};
pub use groups::{gen_invite_token, GroupError, GroupRow, InviteRow, InviteUse, INVITE_TTL_SECS};
pub use stats::{PeriodTotals, Sample, TrafficSummary};

/// SQL-батчи миграций: индекс в массиве + 1 == schema_version после применения.
/// Только добавлять в конец — существующие батчи менять нельзя (уже применены
/// на живых установках).
pub(crate) const MIGRATIONS: &[&str] = &[
    // v1: базовая схема. meta — IF NOT EXISTS: её создаёт migrate() ещё до
    // применения батчей (нужна, чтобы прочитать schema_version).
    r#"
    CREATE TABLE IF NOT EXISTS meta(key TEXT PRIMARY KEY, value TEXT NOT NULL);
    CREATE TABLE settings(key TEXT PRIMARY KEY, value TEXT NOT NULL);
    CREATE TABLE clients(
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL UNIQUE,
        ip TEXT NOT NULL DEFAULT '',
        first_seen INTEGER NOT NULL,
        last_seen INTEGER NOT NULL,
        removed_at INTEGER
    );
    CREATE TABLE traffic_samples(
        client_id INTEGER NOT NULL REFERENCES clients(id),
        ts INTEGER NOT NULL,
        rx INTEGER NOT NULL,
        tx INTEGER NOT NULL,
        rx_delta INTEGER NOT NULL,
        tx_delta INTEGER NOT NULL,
        online INTEGER NOT NULL
    );
    CREATE INDEX idx_samples_client_ts ON traffic_samples(client_id, ts);
    CREATE INDEX idx_samples_ts ON traffic_samples(ts);
    CREATE TABLE traffic_hourly(
        client_id INTEGER NOT NULL REFERENCES clients(id),
        hour_ts INTEGER NOT NULL,
        rx_bytes INTEGER NOT NULL,
        tx_bytes INTEGER NOT NULL,
        online_minutes INTEGER NOT NULL,
        PRIMARY KEY(client_id, hour_ts)
    );
    CREATE TABLE traffic_daily(
        client_id INTEGER NOT NULL REFERENCES clients(id),
        day_ts INTEGER NOT NULL,
        rx_bytes INTEGER NOT NULL,
        tx_bytes INTEGER NOT NULL,
        online_minutes INTEGER NOT NULL,
        PRIMARY KEY(client_id, day_ts)
    );
    CREATE TABLE events(
        id INTEGER PRIMARY KEY,
        ts INTEGER NOT NULL,
        kind TEXT NOT NULL,
        client TEXT,
        actor INTEGER,
        details TEXT
    );
    CREATE INDEX idx_events_ts ON events(ts);
    CREATE INDEX idx_events_client ON events(client, ts);
    "#,
    // v2: группы и делегирование (issue #20). Принадлежность клиента к группе —
    // только в БД; manage_amneziawg.sh про группы не знает.
    r#"
    CREATE TABLE groups(
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL UNIQUE,
        max_clients INTEGER,
        created_at INTEGER NOT NULL
    );
    CREATE TABLE group_admins(
        group_id INTEGER NOT NULL REFERENCES groups(id),
        user_id INTEGER NOT NULL,
        added_at INTEGER NOT NULL,
        added_by INTEGER NOT NULL,
        PRIMARY KEY(group_id, user_id)
    );
    CREATE TABLE invites(
        token TEXT PRIMARY KEY,
        group_id INTEGER NOT NULL REFERENCES groups(id),
        created_by INTEGER NOT NULL,
        created_at INTEGER NOT NULL,
        expires_at INTEGER NOT NULL,
        used_by INTEGER,
        used_at INTEGER
    );
    ALTER TABLE clients ADD COLUMN group_id INTEGER REFERENCES groups(id);
    "#,
];

pub struct Store {
    conn: Mutex<Connection>,
}

impl Store {
    pub fn open(path: &Path) -> Result<Store, rusqlite::Error> {
        if let Some(parent) = path.parent() {
            // В проде каталог создаёт install.sh; здесь — на случай dev-запуска.
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        migrate(&conn)?;
        Ok(Store {
            conn: Mutex::new(conn),
        })
    }

    /// БД в памяти — для тестов store и соседних модулей.
    pub fn open_in_memory() -> Store {
        let conn = Connection::open_in_memory().expect("in-memory sqlite");
        migrate(&conn).expect("миграции на пустой БД не могут падать");
        Store {
            conn: Mutex::new(conn),
        }
    }

    pub fn schema_version(&self) -> i64 {
        self.with_conn(|c| {
            c.query_row(
                "SELECT value FROM meta WHERE key = 'schema_version'",
                [],
                |r| r.get::<_, String>(0),
            )
        })
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
    }

    pub(crate) fn with_conn<T>(
        &self,
        f: impl FnOnce(&Connection) -> rusqlite::Result<T>,
    ) -> rusqlite::Result<T> {
        let conn = self.conn.lock().unwrap();
        f(&conn)
    }
}

fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS meta(key TEXT PRIMARY KEY, value TEXT NOT NULL)",
    )?;
    let current: i64 = conn
        .query_row(
            "SELECT value FROM meta WHERE key='schema_version'",
            [],
            |r| r.get::<_, String>(0),
        )
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    for (i, batch) in MIGRATIONS.iter().enumerate() {
        let version = (i + 1) as i64;
        if version > current {
            conn.execute_batch(batch)?;
            conn.execute(
                "INSERT INTO meta(key,value) VALUES('schema_version',?1)
                 ON CONFLICT(key) DO UPDATE SET value=?1",
                [version.to_string()],
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_creates_schema_and_version() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("sub/awgram.db")).unwrap();
        assert_eq!(store.schema_version(), 2);
    }

    #[test]
    fn reopen_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("awgram.db");
        drop(Store::open(&path).unwrap());
        let store = Store::open(&path).unwrap();
        assert_eq!(store.schema_version(), 2);
    }

    #[test]
    fn in_memory_store_works() {
        let store = Store::open_in_memory();
        assert_eq!(store.schema_version(), 2);
    }
}
