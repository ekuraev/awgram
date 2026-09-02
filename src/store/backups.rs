//! Метаданные бэкапов бота (`backups`). Файловая система — источник истины:
//! строки лишь кэш для списка и носитель комментария/пина. Сводит их с
//! диском `backup::service::reconcile`.

use crate::store::Store;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupKind {
    Auto,
    Manual,
    Upload,
}

impl BackupKind {
    pub fn as_str(self) -> &'static str {
        match self {
            BackupKind::Auto => "auto",
            BackupKind::Manual => "manual",
            BackupKind::Upload => "upload",
        }
    }
    pub fn parse(s: &str) -> Option<BackupKind> {
        match s {
            "auto" => Some(BackupKind::Auto),
            "manual" => Some(BackupKind::Manual),
            "upload" => Some(BackupKind::Upload),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BackupRow {
    pub name: String,
    pub created_at: i64,
    pub kind: BackupKind,
    pub actor: Option<i64>,
    pub comment: Option<String>,
    pub pinned: bool,
    pub size: u64,
    pub sha256: Option<String>,
    pub has_db: bool,
    pub clients: Option<u32>,
    pub groups: Option<u32>,
}

impl Store {
    /// Вставка или обновление по имени. Комментарий и пин при обновлении
    /// НЕ трогаются: они принадлежат пользователю, а не файлу.
    pub fn upsert_backup(&self, r: &BackupRow) {
        let res = self.with_conn(|c| {
            c.execute(
                "INSERT INTO backups(name,created_at,kind,actor,comment,pinned,size,sha256,has_db,clients,groups)
                 VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)
                 ON CONFLICT(name) DO UPDATE SET
                   created_at=excluded.created_at, kind=excluded.kind, actor=excluded.actor,
                   size=excluded.size, sha256=COALESCE(excluded.sha256, backups.sha256),
                   has_db=excluded.has_db, clients=excluded.clients, groups=excluded.groups",
                rusqlite::params![
                    r.name, r.created_at, r.kind.as_str(), r.actor, r.comment, r.pinned as i64,
                    r.size as i64, r.sha256, r.has_db as i64,
                    r.clients.map(|v| v as i64), r.groups.map(|v| v as i64)
                ],
            )
        });
        if let Err(e) = res {
            tracing::error!(error = %e, name = %r.name, "не удалось сохранить бэкап");
        }
    }

    fn row_from(r: &rusqlite::Row<'_>) -> rusqlite::Result<BackupRow> {
        let kind: String = r.get(2)?;
        Ok(BackupRow {
            name: r.get(0)?,
            created_at: r.get(1)?,
            kind: BackupKind::parse(&kind).unwrap_or(BackupKind::Manual),
            actor: r.get(3)?,
            comment: r.get(4)?,
            pinned: r.get::<_, i64>(5)? != 0,
            size: r.get::<_, i64>(6)? as u64,
            sha256: r.get(7)?,
            has_db: r.get::<_, i64>(8)? != 0,
            clients: r.get::<_, Option<i64>>(9)?.map(|v| v as u32),
            groups: r.get::<_, Option<i64>>(10)?.map(|v| v as u32),
        })
    }

    const COLS: &'static str =
        "name,created_at,kind,actor,comment,pinned,size,sha256,has_db,clients,groups";

    pub fn backup_row(&self, name: &str) -> Option<BackupRow> {
        self.with_conn(|c| {
            c.query_row(
                &format!("SELECT {} FROM backups WHERE name=?1", Self::COLS),
                [name],
                Self::row_from,
            )
        })
        .ok()
    }

    /// Все строки, новые первыми.
    pub fn list_backup_rows(&self) -> Vec<BackupRow> {
        self.with_conn(|c| {
            let mut st = c.prepare(&format!(
                "SELECT {} FROM backups ORDER BY created_at DESC, name DESC",
                Self::COLS
            ))?;
            let rows = st.query_map([], Self::row_from)?;
            rows.collect()
        })
        .unwrap_or_default()
    }

    pub fn set_backup_comment(&self, name: &str, comment: Option<&str>) {
        let res = self.with_conn(|c| {
            c.execute(
                "UPDATE backups SET comment=?2 WHERE name=?1",
                rusqlite::params![name, comment],
            )
        });
        if let Err(e) = res {
            tracing::error!(error = %e, name, "не удалось сохранить комментарий бэкапа");
        }
    }

    pub fn set_backup_pinned(&self, name: &str, pinned: bool) {
        let res = self.with_conn(|c| {
            c.execute(
                "UPDATE backups SET pinned=?2 WHERE name=?1",
                rusqlite::params![name, pinned as i64],
            )
        });
        if let Err(e) = res {
            tracing::error!(error = %e, name, "не удалось сохранить пин бэкапа");
        }
    }

    pub fn set_backup_sha256(&self, name: &str, sha: &str) {
        let res = self.with_conn(|c| {
            c.execute(
                "UPDATE backups SET sha256=?2 WHERE name=?1",
                rusqlite::params![name, sha],
            )
        });
        if let Err(e) = res {
            tracing::error!(error = %e, name, "не удалось сохранить sha256 бэкапа");
        }
    }

    pub fn delete_backup_row(&self, name: &str) {
        let res = self.with_conn(|c| c.execute("DELETE FROM backups WHERE name=?1", [name]));
        if let Err(e) = res {
            tracing::error!(error = %e, name, "не удалось удалить строку бэкапа");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(name: &str, ts: i64) -> BackupRow {
        BackupRow {
            name: name.into(),
            created_at: ts,
            kind: BackupKind::Manual,
            actor: Some(7),
            comment: None,
            pinned: false,
            size: 10,
            sha256: None,
            has_db: true,
            clients: Some(2),
            groups: Some(1),
        }
    }

    #[test]
    fn upsert_get_and_list_sorted_desc() {
        let s = Store::open_in_memory();
        s.upsert_backup(&row("a.tar.gz", 100));
        s.upsert_backup(&row("b.tar.gz", 300));
        s.upsert_backup(&row("c.tar.gz", 200));
        let names: Vec<_> = s.list_backup_rows().into_iter().map(|r| r.name).collect();
        assert_eq!(names, vec!["b.tar.gz", "c.tar.gz", "a.tar.gz"]);
        assert_eq!(s.backup_row("a.tar.gz").unwrap(), row("a.tar.gz", 100));
        assert!(s.backup_row("zzz").is_none());
    }

    #[test]
    fn upsert_replaces_size_but_keeps_comment_and_pin() {
        let s = Store::open_in_memory();
        s.upsert_backup(&row("a.tar.gz", 100));
        s.set_backup_comment("a.tar.gz", Some("перед обновлением"));
        s.set_backup_pinned("a.tar.gz", true);
        let mut r2 = row("a.tar.gz", 100);
        r2.size = 999;
        s.upsert_backup(&r2);
        let got = s.backup_row("a.tar.gz").unwrap();
        assert_eq!(got.size, 999);
        assert_eq!(got.comment.as_deref(), Some("перед обновлением"));
        assert!(got.pinned);
    }

    #[test]
    fn comment_clear_sha_and_delete() {
        let s = Store::open_in_memory();
        s.upsert_backup(&row("a.tar.gz", 100));
        s.set_backup_comment("a.tar.gz", Some("x"));
        s.set_backup_comment("a.tar.gz", None);
        assert_eq!(s.backup_row("a.tar.gz").unwrap().comment, None);
        s.set_backup_sha256("a.tar.gz", "abc");
        assert_eq!(
            s.backup_row("a.tar.gz").unwrap().sha256.as_deref(),
            Some("abc")
        );
        s.delete_backup_row("a.tar.gz");
        assert!(s.backup_row("a.tar.gz").is_none());
    }

    #[test]
    fn kind_roundtrip() {
        for k in [BackupKind::Auto, BackupKind::Manual, BackupKind::Upload] {
            assert_eq!(BackupKind::parse(k.as_str()), Some(k));
        }
        assert_eq!(BackupKind::parse("nope"), None);
    }
}
