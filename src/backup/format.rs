//! Формат бандла `awgram_backup_<ts>.tar.gz`: meta.json + awg/<архив
//! инсталлера> + опционально awgram.db. Только чистые функции над путями.
//! Валидация повторяет проверки `restore` инсталлера (обычные файлы и
//! каталоги, относительные пути, без `..`), чтобы отказать раньше и понятнее.

use std::io::{Read, Write};
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const FORMAT_VERSION: u32 = 1;
pub const MAX_FILE_BYTES: u64 = 20 * 1024 * 1024;
pub const META_NAME: &str = "meta.json";
pub const DB_NAME: &str = "awgram.db";
pub const AWG_DIR: &str = "awg";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Meta {
    pub format: u32,
    pub awgram_version: String,
    pub created_at: i64,
    pub kind: String,
    pub actor: Option<i64>,
    pub comment: Option<String>,
    pub has_db: bool,
    pub awg_archive: String,
    pub awg_sha256: String,
    pub clients: u32,
    pub groups: Option<u32>,
}

#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("meta.json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("файл больше лимита: {0} байт")]
    TooLarge(u64),
    #[error("недопустимая запись архива: {0}")]
    BadEntry(String),
    #[error("это не архив инсталлера: нет server/*.conf")]
    NotInstallerArchive,
    #[error("это не бандл awgram: {0}")]
    NotBundle(String),
    #[error("снимок БД повреждён: {0}")]
    DbInvalid(String),
    #[error("схема БД в бэкапе ({found}) новее текущей ({current})")]
    DbTooNew { found: i64, current: i64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Bundle,
    InstallerArchive,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Inspection {
    pub meta: Meta,
    pub inner_name: String,
    pub has_db: bool,
}

pub fn ts_from_awg_name(name: &str) -> Option<&str> {
    name.strip_prefix("awg_backup_")?.strip_suffix(".tar.gz")
}
pub fn bundle_name(ts: &str) -> String {
    format!("awgram_backup_{ts}.tar.gz")
}
pub fn ts_from_bundle_name(name: &str) -> Option<&str> {
    name.strip_prefix("awgram_backup_")?.strip_suffix(".tar.gz")
}

pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut f = std::fs::File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

fn open_targz(
    path: &Path,
) -> Result<tar::Archive<flate2::read::GzDecoder<std::fs::File>>, FormatError> {
    let len = std::fs::metadata(path)?.len();
    if len > MAX_FILE_BYTES {
        return Err(FormatError::TooLarge(len));
    }
    let f = std::fs::File::open(path)?;
    Ok(tar::Archive::new(flate2::read::GzDecoder::new(f)))
}

/// Имя записи без ведущего `./`. Отклоняет ссылки, устройства, абсолютные
/// пути и `..` — ровно то, что режет `restore` инсталлера. Также проверяет
/// заявленный в заголовке размер записи: поодиночке и нарастающим итогом по
/// архиву (`total_bytes`) он не должен превышать `MAX_FILE_BYTES` — иначе
/// компактный по весу (сжатому) архив может заявить гигабайты содержимого
/// и уронить бота по памяти при чтении записи в `Vec`.
fn checked_entry_name<R: Read>(
    e: &tar::Entry<'_, R>,
    total_bytes: &mut u64,
) -> Result<String, FormatError> {
    let et = e.header().entry_type();
    if !(et.is_file() || et.is_dir()) {
        return Err(FormatError::BadEntry(format!("{et:?}")));
    }
    let size = e.size();
    if size > MAX_FILE_BYTES {
        return Err(FormatError::TooLarge(size));
    }
    *total_bytes += size;
    if *total_bytes > MAX_FILE_BYTES {
        return Err(FormatError::TooLarge(*total_bytes));
    }
    let p = e.path().map_err(|e| FormatError::BadEntry(e.to_string()))?;
    let mut out = Vec::new();
    for c in p.components() {
        match c {
            Component::Normal(s) => out.push(s.to_string_lossy().into_owned()),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(FormatError::BadEntry(p.to_string_lossy().into_owned()));
            }
        }
    }
    Ok(out.join("/"))
}

/// Список записей архива после проверки каждой. gzip-мусор даёт `Io`.
/// Отклоняет повторяющиеся имена записей: GNU tar при распаковке берёт
/// последнюю запись с таким именем, а наше чтение — первую, так что дубликат
/// не должен молча пройти.
fn list_entries(path: &Path) -> Result<Vec<String>, FormatError> {
    let mut ar = open_targz(path)?;
    let mut names = Vec::new();
    let mut total_bytes: u64 = 0;
    for e in ar.entries()? {
        let e = e?;
        let name = checked_entry_name(&e, &mut total_bytes)?;
        if names.contains(&name) {
            return Err(FormatError::BadEntry(format!("дубликат записи {name}")));
        }
        names.push(name);
    }
    Ok(names)
}

fn count_clients(names: &[String]) -> u32 {
    names
        .iter()
        .filter(|n| n.starts_with("clients/") && n.ends_with(".conf"))
        .count() as u32
}

pub fn validate_installer_archive(path: &Path) -> Result<u32, FormatError> {
    let names = list_entries(path)?;
    let has_server = names
        .iter()
        .any(|n| n.starts_with("server/") && n.ends_with(".conf"));
    if !has_server {
        return Err(FormatError::NotInstallerArchive);
    }
    Ok(count_clients(&names))
}

pub fn detect(path: &Path) -> Result<FileKind, FormatError> {
    let names = list_entries(path)?;
    if names.iter().any(|n| n == META_NAME) {
        return Ok(FileKind::Bundle);
    }
    if names
        .iter()
        .any(|n| n.starts_with("server/") && n.ends_with(".conf"))
    {
        return Ok(FileKind::InstallerArchive);
    }
    Err(FormatError::NotBundle(
        "нет meta.json и нет server/*.conf".into(),
    ))
}

/// Читает одну запись бандла в память (meta.json, ≤ MAX_FILE_BYTES по построению).
fn read_entry(path: &Path, name: &str) -> Result<Vec<u8>, FormatError> {
    let mut ar = open_targz(path)?;
    let mut total_bytes: u64 = 0;
    for e in ar.entries()? {
        let mut e = e?;
        if checked_entry_name(&e, &mut total_bytes)? == name {
            let mut buf = Vec::new();
            e.read_to_end(&mut buf)?;
            return Ok(buf);
        }
    }
    Err(FormatError::NotBundle(format!("нет записи {name}")))
}

pub fn extract_entry(bundle: &Path, entry_name: &str, dest: &Path) -> Result<(), FormatError> {
    let bytes = read_entry(bundle, entry_name)?;
    let mut f = std::fs::File::create(dest)?;
    f.write_all(&bytes)?;
    Ok(())
}

fn check_db_snapshot(db: &Path, current_schema: i64) -> Result<(), FormatError> {
    let conn = rusqlite::Connection::open_with_flags(
        db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| FormatError::DbInvalid(e.to_string()))?;
    let ok: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(|e| FormatError::DbInvalid(e.to_string()))?;
    if ok != "ok" {
        return Err(FormatError::DbInvalid(ok));
    }
    let found: i64 = conn
        .query_row(
            "SELECT CAST(value AS INTEGER) FROM meta WHERE key='schema_version'",
            [],
            |r| r.get(0),
        )
        .map_err(|e| FormatError::DbInvalid(format!("schema_version: {e}")))?;
    if found > current_schema {
        return Err(FormatError::DbTooNew {
            found,
            current: current_schema,
        });
    }
    Ok(())
}

pub fn inspect_bundle(path: &Path, current_schema: i64) -> Result<Inspection, FormatError> {
    let names = list_entries(path)?;
    if !names.iter().any(|n| n == META_NAME) {
        return Err(FormatError::NotBundle("нет meta.json".into()));
    }
    let inner: Vec<&String> = names
        .iter()
        .filter(|n| n.starts_with("awg/") && n.ends_with(".tar.gz"))
        .collect();
    if inner.len() != 1 {
        return Err(FormatError::NotBundle(format!(
            "ожидался один awg/*.tar.gz, найдено {}",
            inner.len()
        )));
    }
    let meta: Meta = serde_json::from_slice(&read_entry(path, META_NAME)?)?;
    if meta.format != FORMAT_VERSION {
        return Err(FormatError::NotBundle(format!("format {}", meta.format)));
    }
    let inner_name = inner[0]
        .strip_prefix("awg/")
        .unwrap_or(inner[0].as_str())
        .to_string();
    let tmp = tempfile::tempdir()?;
    let inner_path = tmp.path().join("inner.tar.gz");
    extract_entry(path, inner[0], &inner_path)?;
    validate_installer_archive(&inner_path)?;
    let has_db = names.iter().any(|n| n == DB_NAME);
    if has_db {
        let db_path = tmp.path().join(DB_NAME);
        extract_entry(path, DB_NAME, &db_path)?;
        check_db_snapshot(&db_path, current_schema)?;
    }
    Ok(Inspection {
        meta,
        inner_name,
        has_db,
    })
}

pub fn build_bundle(
    out: &Path,
    awg_archive: &Path,
    db_snapshot: Option<&Path>,
    meta: &Meta,
) -> Result<(), FormatError> {
    let f = std::fs::File::create(out)?;
    let enc = flate2::write::GzEncoder::new(f, flate2::Compression::default());
    let mut ar = tar::Builder::new(enc);
    let mj = serde_json::to_vec_pretty(meta)?;
    let mut h = tar::Header::new_gnu();
    h.set_size(mj.len() as u64);
    h.set_mode(0o600);
    h.set_mtime(meta.created_at.max(0) as u64);
    h.set_cksum();
    ar.append_data(&mut h, META_NAME, &mj[..])?;
    let inner_name = awg_archive
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "awg_backup.tar.gz".into());
    ar.append_path_with_name(awg_archive, format!("{AWG_DIR}/{inner_name}"))?;
    if let Some(db) = db_snapshot {
        ar.append_path_with_name(db, DB_NAME)?;
    }
    ar.into_inner()?.finish()?;
    Ok(())
}

/// Фикстуры для тестов соседних модулей (service, handlers).
#[cfg(test)]
pub mod tests_support {
    use std::path::Path;

    /// Минимальный валидный архив инсталлера: server/awg0.conf + 2 клиента.
    pub fn installer_archive_to(path: &Path) {
        let f = std::fs::File::create(path).unwrap();
        let enc = flate2::write::GzEncoder::new(f, flate2::Compression::default());
        let mut ar = tar::Builder::new(enc);
        for (name, body) in [
            ("./server/awg0.conf", &b"[Interface]\n"[..]),
            ("./clients/a.conf", b"x"),
            ("./clients/b.conf", b"y"),
        ] {
            let mut h = tar::Header::new_gnu();
            h.set_size(body.len() as u64);
            h.set_mode(0o600);
            h.set_cksum();
            ar.append_data(&mut h, name, body).unwrap();
        }
        ar.into_inner().unwrap().finish().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// tar.gz из (путь, содержимое). Пустое содержимое с завершающим '/' — каталог.
    fn make_targz(path: &Path, entries: &[(&str, &[u8])]) {
        let f = std::fs::File::create(path).unwrap();
        let enc = flate2::write::GzEncoder::new(f, flate2::Compression::default());
        let mut ar = tar::Builder::new(enc);
        for (name, body) in entries {
            let mut h = tar::Header::new_gnu();
            if name.ends_with('/') {
                h.set_entry_type(tar::EntryType::Directory);
                h.set_size(0);
            } else {
                h.set_size(body.len() as u64);
            }
            h.set_mode(0o600);
            // `Header::set_path` (её вызывает `append_data`) в текущей версии
            // крейта `tar` сама отклоняет `..` и абсолютные пути ещё при
            // записи — а нам здесь нужно собрать заведомо вредоносный архив,
            // чтобы проверить свою защиту на чтении. Пишем имя записи в
            // заголовок напрямую, в обход этой проверки.
            let name_bytes = name.as_bytes();
            h.as_old_mut().name[..name_bytes.len()].copy_from_slice(name_bytes);
            h.set_cksum();
            ar.append(&h, &body[..]).unwrap();
        }
        ar.into_inner().unwrap().finish().unwrap();
    }

    fn installer_archive(dir: &Path) -> std::path::PathBuf {
        let p = dir.join("awg_backup_2026-09-03_03-00-00.123.tar.gz");
        make_targz(
            &p,
            &[
                ("./server/", b""),
                ("./server/awg0.conf", b"[Interface]\n"),
                ("./clients/a.conf", b"x"),
                ("./clients/b.conf", b"y"),
                ("./clients/a.png", b"z"),
                ("./server_private.key", b"k"),
            ],
        );
        p
    }

    fn meta(has_db: bool) -> Meta {
        Meta {
            format: FORMAT_VERSION,
            awgram_version: "0.10.0".into(),
            created_at: 1_756_861_200,
            kind: "manual".into(),
            actor: Some(1),
            comment: Some("тест".into()),
            has_db,
            awg_archive: "awg_backup_2026-09-03_03-00-00.123.tar.gz".into(),
            awg_sha256: String::new(),
            clients: 2,
            groups: if has_db { Some(0) } else { None },
        }
    }

    #[test]
    fn names_roundtrip() {
        assert_eq!(
            ts_from_awg_name("awg_backup_2026-09-03_03-00-00.123.tar.gz"),
            Some("2026-09-03_03-00-00.123")
        );
        assert_eq!(ts_from_awg_name("other.tar.gz"), None);
        assert_eq!(bundle_name("T"), "awgram_backup_T.tar.gz");
        assert_eq!(ts_from_bundle_name("awgram_backup_T.tar.gz"), Some("T"));
        assert_eq!(ts_from_bundle_name("awg_backup_T.tar.gz"), None);
    }

    #[test]
    fn validate_installer_archive_counts_clients() {
        let d = tempfile::tempdir().unwrap();
        let p = installer_archive(d.path());
        assert_eq!(validate_installer_archive(&p).unwrap(), 2);
        assert_eq!(detect(&p).unwrap(), FileKind::InstallerArchive);
    }

    #[test]
    fn validate_rejects_missing_server_conf_traversal_abs_and_links() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("x.tar.gz");
        make_targz(&p, &[("./clients/a.conf", b"x")]);
        assert!(matches!(
            validate_installer_archive(&p),
            Err(FormatError::NotInstallerArchive)
        ));

        make_targz(&p, &[("./server/awg0.conf", b"x"), ("../evil", b"x")]);
        assert!(matches!(
            validate_installer_archive(&p),
            Err(FormatError::BadEntry(_))
        ));

        make_targz(&p, &[("./server/awg0.conf", b"x"), ("/etc/passwd", b"x")]);
        assert!(matches!(
            validate_installer_archive(&p),
            Err(FormatError::BadEntry(_))
        ));

        // symlink
        let f = std::fs::File::create(&p).unwrap();
        let enc = flate2::write::GzEncoder::new(f, flate2::Compression::default());
        let mut ar = tar::Builder::new(enc);
        let mut h = tar::Header::new_gnu();
        h.set_entry_type(tar::EntryType::Symlink);
        h.set_size(0);
        h.set_cksum();
        ar.append_link(&mut h, "./server/awg0.conf", "/etc/passwd")
            .unwrap();
        ar.into_inner().unwrap().finish().unwrap();
        assert!(matches!(
            validate_installer_archive(&p),
            Err(FormatError::BadEntry(_))
        ));
    }

    #[test]
    fn validate_rejects_too_large_and_not_gzip() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("big.tar.gz");
        let f = std::fs::File::create(&p).unwrap();
        f.set_len(MAX_FILE_BYTES + 1).unwrap();
        assert!(matches!(
            validate_installer_archive(&p),
            Err(FormatError::TooLarge(_))
        ));
        std::fs::write(&p, b"not a gzip").unwrap();
        assert!(matches!(
            validate_installer_archive(&p),
            Err(FormatError::Io(_))
        ));
    }

    #[test]
    fn build_inspect_extract_bundle_with_db() {
        let d = tempfile::tempdir().unwrap();
        let awg = installer_archive(d.path());
        let db = d.path().join("snap.db");
        {
            let c = rusqlite::Connection::open(&db).unwrap();
            c.execute_batch(
                "CREATE TABLE meta(key TEXT PRIMARY KEY, value TEXT NOT NULL);
                 INSERT INTO meta VALUES('schema_version','3');",
            )
            .unwrap();
        }
        let out = d.path().join(bundle_name("2026-09-03_03-00-00.123"));
        let mut m = meta(true);
        m.awg_sha256 = sha256_file(&awg).unwrap();
        build_bundle(&out, &awg, Some(&db), &m).unwrap();

        assert_eq!(detect(&out).unwrap(), FileKind::Bundle);
        let insp = inspect_bundle(&out, 3).unwrap();
        assert_eq!(insp.meta, m);
        assert_eq!(insp.inner_name, "awg_backup_2026-09-03_03-00-00.123.tar.gz");
        assert!(insp.has_db);
        // схема новее текущей — отказ
        assert!(matches!(
            inspect_bundle(&out, 2),
            Err(FormatError::DbTooNew {
                found: 3,
                current: 2
            })
        ));

        let inner = d.path().join("inner.tar.gz");
        extract_entry(
            &out,
            "awg/awg_backup_2026-09-03_03-00-00.123.tar.gz",
            &inner,
        )
        .unwrap();
        assert_eq!(sha256_file(&inner).unwrap(), m.awg_sha256);
        assert_eq!(validate_installer_archive(&inner).unwrap(), 2);
        let db2 = d.path().join("out.db");
        extract_entry(&out, DB_NAME, &db2).unwrap();
        assert!(db2.exists());
        assert!(matches!(
            extract_entry(&out, "nope", &db2),
            Err(FormatError::NotBundle(_))
        ));
    }

    #[test]
    fn bundle_without_db_and_bad_bundles() {
        let d = tempfile::tempdir().unwrap();
        let awg = installer_archive(d.path());
        let out = d.path().join("b.tar.gz");
        build_bundle(&out, &awg, None, &meta(false)).unwrap();
        let insp = inspect_bundle(&out, 3).unwrap();
        assert!(!insp.has_db);

        // нет meta.json → это не бандл; detect говорит «архив инсталлера» только
        // при наличии server/*.conf, иначе NotBundle
        let p = d.path().join("nometa.tar.gz");
        make_targz(&p, &[("awg/awg_backup_x.tar.gz", b"x")]);
        assert!(matches!(
            inspect_bundle(&p, 3),
            Err(FormatError::NotBundle(_))
        ));
        assert!(matches!(detect(&p), Err(FormatError::NotBundle(_))));

        // два внутренних архива
        let p2 = d.path().join("two.tar.gz");
        let mj = serde_json::to_vec(&meta(false)).unwrap();
        make_targz(
            &p2,
            &[
                (META_NAME, &mj),
                ("awg/a.tar.gz", b"x"),
                ("awg/b.tar.gz", b"y"),
            ],
        );
        assert!(matches!(
            inspect_bundle(&p2, 3),
            Err(FormatError::NotBundle(_))
        ));

        // битая БД
        let p3 = d.path().join("baddb.tar.gz");
        let mut m = meta(true);
        m.has_db = true;
        let mj = serde_json::to_vec(&m).unwrap();
        let awg_bytes = std::fs::read(&awg).unwrap();
        make_targz(
            &p3,
            &[
                (META_NAME, &mj),
                ("awg/awg_backup_2026-09-03_03-00-00.123.tar.gz", &awg_bytes),
                (DB_NAME, b"garbage"),
            ],
        );
        assert!(matches!(
            inspect_bundle(&p3, 3),
            Err(FormatError::DbInvalid(_))
        ));

        // дубликат записи (два meta.json) — GNU tar при распаковке взял бы
        // последнюю, наше чтение — первую; отклоняем как BadEntry
        let p4 = d.path().join("dup.tar.gz");
        make_targz(&p4, &[(META_NAME, &mj), (META_NAME, &mj)]);
        assert!(matches!(
            inspect_bundle(&p4, 3),
            Err(FormatError::BadEntry(_))
        ));
    }

    #[test]
    fn validate_rejects_oversized_declared_entry() {
        // Заголовок заявляет размер записи больше лимита, но сама запись —
        // поток нулей, который gzip сжимает до пары килобайт: сжатый файл
        // проходит проверку размера в `open_targz`, а вот заявленный размер
        // записи — нет.
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("bomb.tar.gz");
        let f = std::fs::File::create(&p).unwrap();
        let enc = flate2::write::GzEncoder::new(f, flate2::Compression::default());
        let mut ar = tar::Builder::new(enc);
        let mut h = tar::Header::new_gnu();
        h.set_size(MAX_FILE_BYTES + 1);
        h.set_mode(0o600);
        h.set_cksum();
        ar.append_data(
            &mut h,
            "server/awg0.conf",
            std::io::repeat(0).take(MAX_FILE_BYTES + 1),
        )
        .unwrap();
        ar.into_inner().unwrap().finish().unwrap();

        assert!(matches!(
            validate_installer_archive(&p),
            Err(FormatError::TooLarge(_))
        ));
    }

    #[test]
    fn sha256_known_vector() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("f");
        std::fs::File::create(&p)
            .unwrap()
            .write_all(b"abc")
            .unwrap();
        assert_eq!(
            sha256_file(&p).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
