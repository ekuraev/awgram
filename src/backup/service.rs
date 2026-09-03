//! Сценарии бэкапов. Файловая система — источник истины; таблица `backups`
//! сводится с ней в `reconcile`. Тексты для пользователя здесь не
//! формируются — только структуры результата и `crate::error::Error`.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::backup::format::{self, FileKind, Meta, DB_NAME};
use crate::error::{Error, Result};
use crate::store::{BackupKind, BackupRow, Store};
use crate::vpn::{BackupFile, Vpn};

/// Адрес бэкапа в callback: бандл бота (`<ts>`) или pre-restore снапшот
/// инсталлера в родительской папке (`i:<ts>`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    Bundle(String),
    Installer(String),
}

fn ts_ok(ts: &str) -> bool {
    !ts.is_empty()
        && ts.len() <= 40
        && ts
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

impl Key {
    pub fn encode(&self) -> String {
        match self {
            Key::Bundle(ts) => ts.clone(),
            Key::Installer(ts) => format!("i:{ts}"),
        }
    }
    pub fn parse(s: &str) -> Option<Key> {
        if let Some(ts) = s.strip_prefix("i:") {
            return ts_ok(ts).then(|| Key::Installer(ts.to_string()));
        }
        ts_ok(s).then(|| Key::Bundle(s.to_string()))
    }
    pub fn ts(&self) -> &str {
        match self {
            Key::Bundle(t) | Key::Installer(t) => t,
        }
    }
}

#[derive(Debug)]
pub struct Created {
    pub row: BackupRow,
    pub elapsed_ms: u64,
    pub rotated: usize,
}

pub enum Located {
    Bundle(BackupRow, PathBuf),
    Installer(BackupFile),
}

/// Результат восстановления. `db_error` заполняется, только если AWG уже
/// восстановлен, а БД бота — нет: возвращать в этом случае `Err` нельзя,
/// иначе пользователь видит общую ошибку и повторяет всю операцию, хотя
/// главная её часть уже прошла.
#[derive(Debug)]
pub struct RestoreOutcome {
    pub awg: bool,
    pub db: bool,
    pub db_error: Option<String>,
}

fn now_epoch() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn bundle_path(vpn: &Vpn, ts: &str) -> PathBuf {
    vpn.bot_backups_dir().join(format::bundle_name(ts))
}

pub fn installer_snapshots(vpn: &Vpn) -> Vec<BackupFile> {
    vpn.list_backups().unwrap_or_default()
}

/// Свободное место в байтах по `df`; `None`, если каталог не существует или
/// `df` недоступен/не смог его разобрать. Сначала пробуем GNU-вариант
/// (`--output=avail -B1`, есть на Linux), если он не сработал — BSD/macOS
/// вариант (`df -k`, 4-я колонка Avail в 1К-блоках).
pub fn free_bytes(dir: &Path) -> Option<u64> {
    if let Some(out) = std::process::Command::new("df")
        .args(["--output=avail", "-B1"])
        .arg(dir)
        .output()
        .ok()
        .filter(|o| o.status.success())
    {
        let s = String::from_utf8_lossy(&out.stdout);
        if let Some(v) = s.lines().nth(1).and_then(|l| l.trim().parse().ok()) {
            return Some(v);
        }
    }
    let out = std::process::Command::new("df")
        .arg("-k")
        .arg(dir)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    let avail_k: u64 = s.lines().nth(1)?.split_whitespace().nth(3)?.parse().ok()?;
    Some(avail_k * 1024)
}

fn row_from_meta(name: &str, size: u64, sha: Option<String>, m: &Meta) -> BackupRow {
    BackupRow {
        name: name.to_string(),
        created_at: m.created_at,
        kind: BackupKind::parse(&m.kind).unwrap_or(BackupKind::Upload),
        actor: m.actor,
        comment: m.comment.clone(),
        pinned: false,
        size,
        sha256: sha,
        has_db: m.has_db,
        clients: Some(m.clients),
        groups: m.groups,
    }
}

/// Сводит таблицу с каталогом бота и возвращает актуальные строки (новые
/// первыми). Строки без файла удаляются; файлы без строки заводятся из
/// meta.json (нечитаемые пропускаются с warn).
pub fn reconcile(vpn: &Vpn, store: &Store) -> Vec<BackupRow> {
    let dir = vpn.bot_backups_dir();
    let mut on_disk: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    match std::fs::read_dir(&dir) {
        Ok(rd) => {
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                if format::ts_from_bundle_name(&name).is_none() {
                    continue;
                }
                if let Ok(m) = e.metadata() {
                    if m.is_file() {
                        on_disk.insert(name, m.len());
                    }
                }
            }
        }
        // Каталога ещё нет — легитимно «файлов нет»: ниже все строки будут
        // признаны осиротевшими и удалены (тот самый случай, когда бэкапов
        // ещё не было).
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        // Каталог есть, но прочитать не вышло (например, на его месте
        // оказался обычный файл — ENOTDIR, или временная проблема с
        // правами/ФС). Это НЕ «файлов нет»: если считать иначе, ниже
        // удалятся ВСЕ строки, включая закреплённые, и следующий rotate
        // может снести закреплённые бэкапы просто потому, что reconcile не
        // смог прочитать каталог. Строки не трогаем и возвращаем как есть.
        Err(e) => {
            tracing::warn!(
                error = %e,
                dir = %dir.display(),
                "не удалось прочитать каталог бэкапов, строки не трогаю"
            );
            return store.list_backup_rows();
        }
    }
    for r in store.list_backup_rows() {
        if !on_disk.contains_key(&r.name) {
            store.delete_backup_row(&r.name);
        }
    }
    let known: std::collections::HashSet<String> = store
        .list_backup_rows()
        .into_iter()
        .map(|r| r.name)
        .collect();
    for (name, size) in &on_disk {
        if known.contains(name) {
            continue;
        }
        // Файл с ts, который не парсится обратно в Key (слишком длинный,
        // посторонние символы), заводить нельзя: callback `bk:card:<ts>` не
        // влезет в 64 байта лимита Telegram или не разберётся, и карточка со
        // списком станут мёртвыми кнопками.
        if format::ts_from_bundle_name(name)
            .and_then(Key::parse)
            .is_none()
        {
            tracing::warn!(file = %name, "ts бандла не годится для callback, не подхватываю");
            continue;
        }
        let path = dir.join(name);
        match format::inspect_bundle(&path, Store::current_schema()) {
            Ok(insp) => {
                let sha = format::sha256_file(&path).ok();
                store.upsert_backup(&row_from_meta(name, *size, sha, &insp.meta));
            }
            Err(e) => tracing::warn!(error = %e, file = %name, "бандл не прочитан, пропускаю"),
        }
    }
    store.list_backup_rows()
}

pub fn find(vpn: &Vpn, store: &Store, key: &Key) -> Option<Located> {
    match key {
        Key::Bundle(ts) => {
            let rows = reconcile(vpn, store);
            let name = format::bundle_name(ts);
            let row = rows.into_iter().find(|r| r.name == name)?;
            Some(Located::Bundle(row, bundle_path(vpn, ts)))
        }
        Key::Installer(ts) => {
            let name = format!("awg_backup_{ts}.tar.gz");
            installer_snapshots(vpn)
                .into_iter()
                .find(|b| b.name == name)
                .map(Located::Installer)
        }
    }
}

/// Удаляет незакреплённые бандлы сверх `keep` (старые первыми). Возвращает
/// число удалённых.
pub fn rotate(vpn: &Vpn, store: &Store, keep: u32) -> usize {
    let rows = reconcile(vpn, store);
    let mut victims = 0;
    for (i, r) in rows.iter().filter(|r| !r.pinned).enumerate() {
        if (i as u32) < keep {
            continue;
        }
        if let Some(ts) = format::ts_from_bundle_name(&r.name) {
            if std::fs::remove_file(bundle_path(vpn, ts)).is_ok() {
                store.delete_backup_row(&r.name);
                victims += 1;
            }
        }
    }
    victims
}

fn unreadable(e: std::io::Error, path: &Path) -> Error {
    if e.kind() == std::io::ErrorKind::PermissionDenied {
        Error::BackupUnreadable(path.display().to_string())
    } else {
        Error::Io(e)
    }
}

/// Ошибка разбора архива: `PermissionDenied` — это не «битый бэкап», а
/// недоступный файл (hardened-режим инсталлера, root:600), и текст для
/// пользователя должен быть другим.
fn format_err(e: format::FormatError, path: &Path) -> Error {
    match e {
        format::FormatError::Io(io) => unreadable(io, path),
        other => Error::BackupInvalid(other),
    }
}

/// `JoinError` бывает только при панике в блокирующей задаче или её отмене:
/// наружу это ошибка ввода-вывода, а не отдельный класс сбоя.
fn blocking_err(e: tokio::task::JoinError) -> Error {
    Error::Io(std::io::Error::other(format!(
        "фоновая задача бэкапа прервана: {e}"
    )))
}

/// Метка времени для бандла, когда взять её из имени архива не вышло.
fn generated_ts() -> String {
    chrono::Local::now()
        .format("%Y-%m-%d_%H-%M-%S.000")
        .to_string()
}

/// Проверка места: грубая оценка «нужно примерно вдвое больше веса
/// последнего бэкапа» — неточная (новый может оказаться больше), но
/// дешёвая и достаточная, чтобы не запускать заведомо провальную
/// операцию. Нет ни одного бэкапа (need==0) — проверять нечего.
fn check_space(vpn: &Vpn, store: &Store) -> Result<()> {
    let Some(free) = free_bytes(&vpn.bot_backups_dir()).or_else(|| free_bytes(&vpn.backups_dir()))
    else {
        return Ok(());
    };
    let last = store
        .list_backup_rows()
        .first()
        .map(|r| r.size)
        .unwrap_or(0);
    let need = last.saturating_mul(2);
    if need > 0 && free < need {
        return Err(Error::BackupNoSpace { need, free });
    }
    Ok(())
}

/// Синхронная половина `create`: всё после того, как инсталлер отдал архив.
/// Здесь tar/gzip, sha256 по двум файлам и запись в SQLite — секунды
/// заблокированного потока, поэтому вызывается только из `spawn_blocking`.
#[allow(clippy::too_many_arguments)]
fn pack_bundle(
    vpn: &Vpn,
    store: &Store,
    awg: BackupFile,
    kind: BackupKind,
    actor: Option<i64>,
    comment: Option<String>,
    include_db: bool,
    keep: u32,
) -> Result<(BackupRow, usize)> {
    let ts = format::ts_from_awg_name(&awg.name)
        .ok_or_else(|| Error::Parse(format!("неожиданное имя архива {}", awg.name)))?
        .to_string();
    let clients =
        format::validate_installer_archive(&awg.path).map_err(|e| format_err(e, &awg.path))?;
    let awg_sha = format::sha256_file(&awg.path).map_err(|e| unreadable(e, &awg.path))?;

    let tmp = tempfile::tempdir()?;
    let (db_path, groups) = if include_db {
        let p = tmp.path().join(DB_NAME);
        store
            .snapshot_to(&p)
            .map_err(|e| Error::Parse(format!("снимок БД: {e}")))?;
        (Some(p), Some(store.list_groups().len() as u32))
    } else {
        (None, None)
    };
    let meta = Meta {
        format: format::FORMAT_VERSION,
        awgram_version: env!("CARGO_PKG_VERSION").to_string(),
        created_at: now_epoch(),
        kind: kind.as_str().to_string(),
        actor,
        comment: comment.clone(),
        has_db: db_path.is_some(),
        awg_archive: awg.name.clone(),
        awg_sha256: awg_sha,
        clients,
        groups,
    };
    let dir = vpn.bot_backups_dir();
    std::fs::create_dir_all(&dir)?;
    let name = format::bundle_name(&ts);
    let out = dir.join(&name);
    let tmp_out = dir.join(format!(".{name}.part"));
    if let Err(e) = format::build_bundle(&tmp_out, &awg.path, db_path.as_deref(), &meta) {
        // Частично записанный .part не должен остаться мусором.
        let _ = std::fs::remove_file(&tmp_out);
        return Err(e.into());
    }
    std::fs::rename(&tmp_out, &out)?;
    // Архив инсталлера теперь внутри бандла; из backups/ убираем, чтобы он
    // не считался снапшотом и не занимал место дважды. Путь пришёл из JSON
    // самого инсталлера — на всякий случай проверяем, что он и правда внутри
    // ожидаемого каталога, прежде чем удалять чужой файл по чужому пути.
    if awg.path.starts_with(vpn.backups_dir()) {
        if let Err(e) = std::fs::remove_file(&awg.path) {
            tracing::warn!(error = %e, "не удалось убрать архив инсталлера после упаковки");
        }
    } else {
        tracing::warn!(
            path = %awg.path.display(),
            "архив инсталлера вне backups_dir — не удаляю"
        );
    }
    let size = std::fs::metadata(&out)?.len();
    if size > format::MAX_UPLOAD_BYTES {
        tracing::warn!(
            size,
            limit = format::MAX_UPLOAD_BYTES,
            file = %name,
            "бандл больше лимита Telegram — скачать его через бота не выйдет"
        );
    }
    let sha = format::sha256_file(&out)?;
    let mut row = row_from_meta(&name, size, Some(sha), &meta);
    row.kind = kind;
    store.upsert_backup(&row);
    if let Some(c) = comment.as_deref() {
        store.set_backup_comment(&name, Some(c));
    }
    let rotated = rotate(vpn, store, keep);
    let row = store.backup_row(&name).unwrap_or(row);
    Ok((row, rotated))
}

/// Создаёт бэкап: спрашивает архив у инсталлера (async) и упаковывает бандл
/// (sync, в `spawn_blocking` — как `collector` уводит запись статистики).
pub async fn create(
    vpn: &Arc<Vpn>,
    store: &Arc<Store>,
    kind: BackupKind,
    actor: Option<i64>,
    comment: Option<String>,
    include_db: bool,
    keep: u32,
) -> Result<Created> {
    {
        // `df` — это запуск процесса, а чтение таблицы — SQLite: обе операции
        // блокирующие, на реакторе им не место.
        let (v, s) = (vpn.clone(), store.clone());
        tokio::task::spawn_blocking(move || check_space(&v, &s))
            .await
            .map_err(blocking_err)??;
    }
    let started = std::time::Instant::now();
    let awg = vpn.backup().await?;
    let (v, s) = (vpn.clone(), store.clone());
    let (row, rotated) = tokio::task::spawn_blocking(move || {
        pack_bundle(&v, &s, awg, kind, actor, comment, include_db, keep)
    })
    .await
    .map_err(blocking_err)??;
    Ok(Created {
        row,
        elapsed_ms: started.elapsed().as_millis() as u64,
        rotated,
    })
}

pub fn delete(vpn: &Vpn, store: &Store, key: &Key) -> Result<()> {
    match find(vpn, store, key).ok_or(Error::BackupNotFound)? {
        Located::Bundle(row, path) => {
            std::fs::remove_file(&path)?;
            store.delete_backup_row(&row.name);
        }
        Located::Installer(bf) => std::fs::remove_file(&bf.path)?,
    }
    Ok(())
}

/// Пересчитывает SHA-256 бандла. `true` — совпала с сохранённой (или
/// сохранённой не было — тогда запоминаем свежую).
pub fn verify(vpn: &Vpn, store: &Store, ts: &str) -> Result<bool> {
    let Some(Located::Bundle(row, path)) = find(vpn, store, &Key::Bundle(ts.into())) else {
        return Err(Error::BackupNotFound);
    };
    let actual = format::sha256_file(&path).map_err(|e| unreadable(e, &path))?;
    match row.sha256 {
        Some(stored) => Ok(stored == actual),
        None => {
            store.set_backup_sha256(&row.name, &actual);
            Ok(true)
        }
    }
}

/// Внутренний архив инсталлера во временной папке (для restore и «скачать
/// архив AWG»). Сверяет SHA-256 распакованного архива с записанным в
/// meta.json при сборке — порча бандла (битый диск, ручное вмешательство)
/// не должна молча дойти до `restore_path`. TempDir нужно держать живым,
/// пока файл используется.
pub fn extract_inner(vpn: &Vpn, ts: &str) -> Result<(tempfile::TempDir, PathBuf)> {
    let path = bundle_path(vpn, ts);
    if !path.exists() {
        return Err(Error::BackupNotFound);
    }
    let insp =
        format::inspect_bundle(&path, Store::current_schema()).map_err(|e| format_err(e, &path))?;
    let tmp = tempfile::tempdir()?;
    let inner = tmp.path().join(&insp.inner_name);
    format::extract_entry(
        &path,
        &format!("{}/{}", format::AWG_DIR, insp.inner_name),
        &inner,
    )
    .map_err(|e| format_err(e, &path))?;
    let actual = format::sha256_file(&inner).map_err(|e| unreadable(e, &inner))?;
    if actual != insp.meta.awg_sha256 {
        return Err(Error::BackupInvalid(format::FormatError::BadEntry(
            "awg_sha256 не совпадает".into(),
        )));
    }
    Ok((tmp, inner))
}

/// Достаёт снимок БД из бандла и накатывает его. Ошибку возвращает текстом,
/// а не `Error`: вызывается уже ПОСЛЕ восстановления AWG, и наверх она едет
/// как поле `RestoreOutcome::db_error`, а не как провал всей операции.
fn restore_db(store: &Store, bundle: &Path, tmp_dir: &Path) -> std::result::Result<(), String> {
    let db = tmp_dir.join(DB_NAME);
    format::extract_entry(bundle, DB_NAME, &db).map_err(|e| e.to_string())?;
    store.restore_from(&db).map_err(|e| e.to_string())
}

pub async fn restore(
    vpn: &Arc<Vpn>,
    store: &Arc<Store>,
    key: &Key,
    with_db: bool,
) -> Result<RestoreOutcome> {
    let located = {
        let (v, s, k) = (vpn.clone(), store.clone(), key.clone());
        tokio::task::spawn_blocking(move || find(&v, &s, &k))
            .await
            .map_err(blocking_err)?
    };
    match located.ok_or(Error::BackupNotFound)? {
        Located::Installer(bf) => {
            {
                let p = bf.path.clone();
                tokio::task::spawn_blocking(move || {
                    format::validate_installer_archive(&p).map_err(|e| format_err(e, &p))
                })
                .await
                .map_err(blocking_err)??;
            }
            vpn.restore_path(&bf.path).await?;
            Ok(RestoreOutcome {
                awg: true,
                db: false,
                db_error: None,
            })
        }
        Located::Bundle(row, path) => {
            let (tmp, inner) = {
                let (v, ts) = (vpn.clone(), key.ts().to_string());
                tokio::task::spawn_blocking(move || extract_inner(&v, &ts))
                    .await
                    .map_err(blocking_err)??
            };
            vpn.restore_path(&inner).await?;
            let mut out = RestoreOutcome {
                awg: true,
                db: false,
                db_error: None,
            };
            if with_db && row.has_db {
                let s = store.clone();
                // `tmp` переезжает в задачу: временный каталог должен жить,
                // пока в него распаковывают снимок БД.
                let res = tokio::task::spawn_blocking(move || restore_db(&s, &path, tmp.path()))
                    .await
                    .map_err(blocking_err)?;
                match res {
                    Ok(()) => out.db = true,
                    // AWG уже восстановлен — откатывать нечего и незачем.
                    // Сообщаем ровно то, что произошло, отдельной строкой.
                    Err(e) => {
                        tracing::error!(
                            error = %e,
                            backup = %row.name,
                            "БД бота не восстановлена, AmneziaWG уже восстановлен"
                        );
                        out.db_error = Some(e);
                    }
                }
            }
            Ok(out)
        }
    }
}

/// Кладёт `src` в `dir` под именем `awgram_backup_<ts>[-N].tar.gz`, подбирая
/// свободное имя при коллизии (`-1`, `-2`, …). Копирует во временный
/// `.<имя>.part` и переименовывает его в целевое одним rename — чтобы
/// `reconcile` (или параллельный запрос) не увидел частично записанный
/// файл. Возвращает итоговое имя.
fn place_bundle(dir: &Path, ts: &str, src: &Path) -> Result<String> {
    let mut name = format::bundle_name(ts);
    let mut n = 0;
    while dir.join(&name).exists() {
        n += 1;
        name = format!("awgram_backup_{ts}-{n}.tar.gz");
    }
    let dest = dir.join(&name);
    let tmp = dir.join(format!(".{name}.part"));
    // `fs::copy` переносит режим исходника — а он пришёл из временной папки
    // после скачивания из Telegram. Бандл содержит ключи сервера, поэтому
    // режим выставляем свой ещё до появления файла под целевым именем.
    std::fs::copy(src, &tmp)?;
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
    std::fs::rename(&tmp, &dest)?;
    Ok(name)
}

/// Принимает присланный файл: бандл кладётся как есть (имя по ts из meta),
/// чистый архив инсталлера оборачивается в бандл без БД. При коллизии имени —
/// суффикс `-1`, `-2`, …
pub fn accept_upload(
    vpn: &Vpn,
    store: &Store,
    tmp_file: &Path,
    actor: Option<i64>,
) -> Result<BackupRow> {
    let dir = vpn.bot_backups_dir();
    std::fs::create_dir_all(&dir)?;
    let kind = format::detect(tmp_file)?;
    match kind {
        FileKind::Bundle => {
            let insp = format::inspect_bundle(tmp_file, Store::current_schema())?;
            // Имя внутреннего архива — из чужого файла: там может быть что
            // угодно по длине и составу. Ровно как в ветке архива инсталлера,
            // берём ts только если он пройдёт `Key::parse` (иначе
            // `bk:card:<ts>` не влезет в 64 байта callback'а Telegram и
            // карточка окажется недостижимой).
            let ts = format::ts_from_awg_name(&insp.inner_name)
                .filter(|t| ts_ok(t))
                .map(str::to_string)
                .unwrap_or_else(generated_ts);
            let name = place_bundle(&dir, &ts, tmp_file)?;
            finish_upload(store, &dir.join(&name), &name, kind, actor)
        }
        FileKind::InstallerArchive => {
            let clients = format::validate_installer_archive(tmp_file)?;
            let fname = tmp_file
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            // ts из имени файла принимаем только если он же пройдёт Key::parse
            // (та же проверка `ts_ok`) — иначе, например, `awg_backup_.tar.gz`
            // даёт пустой ts, бандл с именем `awgram_backup_.tar.gz` и ключ,
            // который потом не парсится обратно — мёртвые кнопки в карточке.
            let ts = format::ts_from_awg_name(&fname)
                .filter(|t| ts_ok(t))
                .map(str::to_string)
                .unwrap_or_else(generated_ts);
            let awg_name = format!("awg_backup_{ts}.tar.gz");
            // Собираем бандл во временном каталоге — он должен жить, пока
            // place_bundle не скопирует готовый файл в bot_backups_dir.
            let stage_dir = tempfile::tempdir()?;
            let awg_copy = stage_dir.path().join(&awg_name);
            std::fs::copy(tmp_file, &awg_copy)?;
            let meta = Meta {
                format: format::FORMAT_VERSION,
                awgram_version: env!("CARGO_PKG_VERSION").to_string(),
                created_at: now_epoch(),
                kind: BackupKind::Upload.as_str().to_string(),
                actor,
                comment: None,
                has_db: false,
                awg_archive: awg_name,
                awg_sha256: format::sha256_file(&awg_copy)?,
                clients,
                groups: None,
            };
            let staged = stage_dir.path().join("bundle.tar.gz");
            format::build_bundle(&staged, &awg_copy, None, &meta)?;
            let name = place_bundle(&dir, &ts, &staged)?;
            finish_upload(store, &dir.join(&name), &name, kind, actor)
        }
    }
}

fn finish_upload(
    store: &Store,
    dest: &Path,
    name: &str,
    kind: FileKind,
    actor: Option<i64>,
) -> Result<BackupRow> {
    let insp = format::inspect_bundle(dest, Store::current_schema())?;
    let size = std::fs::metadata(dest)?.len();
    let sha = format::sha256_file(dest)?;
    let mut row = row_from_meta(name, size, Some(sha), &insp.meta);
    if kind == FileKind::InstallerArchive {
        row.kind = BackupKind::Upload;
        row.actor = actor;
    }
    store.upsert_backup(&row);
    if let Some(c) = row.comment.as_deref() {
        store.set_backup_comment(name, Some(c));
    }
    Ok(store.backup_row(name).unwrap_or(row))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{BackupKind, Store};
    use crate::vpn::Vpn;
    use serial_test::serial;
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;

    /// Скрипт: backup → пишет валидный архив инсталлера в $dir/backups и
    /// печатает конверт; restore → печатает ok и запоминает путь в файл
    /// restore_called. Метка времени — своя дробная часть (RANDOM % 1000),
    /// а не `%3N` (macOS `date` его не понимает): важна только уникальность
    /// имени файла между быстрыми последовательными вызовами.
    fn stub(dir: &Path) -> (tempfile::TempDir, Arc<Vpn>, Arc<Store>) {
        let awg = dir.join("fixture_awg.tar.gz");
        crate::backup::format::tests_support::installer_archive_to(&awg);
        let script = dir.join("fake.sh");
        let body = format!(
            r#"#!/bin/sh
if [ "$1" = backup ]; then
  mkdir -p "{d}/backups"
  ts=$(date +%F_%H-%M-%S).$(printf '%03d' $((RANDOM % 1000)))
  cp "{awg}" "{d}/backups/awg_backup_$ts.tar.gz"
  echo "{{\"command\":\"backup\",\"ok\":true,\"path\":\"{d}/backups/awg_backup_$ts.tar.gz\",\"size_bytes\":1}}"
  exit 0
fi
if [ "$1" = restore ]; then
  echo "$2" > "{d}/restore_called"
  echo '{{"command":"restore","ok":true,"applied":true,"rolled_back":false}}'
  exit 0
fi
exit 1
"#,
            d = dir.display(),
            awg = awg.display()
        );
        std::fs::write(&script, body).unwrap();
        let mut p = std::fs::metadata(&script).unwrap().permissions();
        p.set_mode(0o755);
        std::fs::set_permissions(&script, p).unwrap();
        let keep = tempfile::tempdir().unwrap();
        let vpn = Vpn::test_with_script(script, dir.to_path_buf());
        (keep, Arc::new(vpn), Arc::new(Store::open_in_memory()))
    }

    #[test]
    fn key_encode_parse() {
        assert_eq!(
            Key::parse("2026-09-03_03-00-00.123"),
            Some(Key::Bundle("2026-09-03_03-00-00.123".into()))
        );
        assert_eq!(
            Key::parse("i:2026-09-03_03-00-00.123"),
            Some(Key::Installer("2026-09-03_03-00-00.123".into()))
        );
        assert_eq!(Key::Bundle("t".into()).encode(), "t");
        assert_eq!(Key::Installer("t".into()).encode(), "i:t");
        assert_eq!(Key::parse(""), None);
        assert_eq!(Key::parse("a/b"), None);
        assert_eq!(Key::parse("i:"), None);
        assert_eq!(Key::parse(&"x".repeat(41)), None);
    }

    #[tokio::test]
    #[serial]
    async fn create_builds_bundle_with_db_and_moves_installer_archive() {
        let dir = tempfile::tempdir().unwrap();
        let (_k, vpn, store) = stub(dir.path());
        store.set_psk_default(true);
        let c = create(
            &vpn,
            &store,
            BackupKind::Manual,
            Some(9),
            Some("до апдейта".into()),
            true,
            7,
        )
        .await
        .unwrap();
        assert!(c.row.name.starts_with("awgram_backup_"));
        assert_eq!(c.row.kind, BackupKind::Manual);
        assert_eq!(c.row.comment.as_deref(), Some("до апдейта"));
        assert!(c.row.has_db);
        assert_eq!(c.row.clients, Some(2));
        assert_eq!(c.row.groups, Some(0));
        assert!(c.row.sha256.is_some());
        let path = bundle_path(
            &vpn,
            crate::backup::format::ts_from_bundle_name(&c.row.name).unwrap(),
        );
        assert!(path.exists());
        // архив инсталлера перенесён внутрь бандла, в backups/ его больше нет
        assert!(installer_snapshots(&vpn).is_empty());
        let insp = crate::backup::format::inspect_bundle(&path, Store::current_schema()).unwrap();
        assert!(insp.has_db);
        assert_eq!(store.list_backup_rows().len(), 1);
    }

    #[tokio::test]
    #[serial]
    async fn create_without_db() {
        let dir = tempfile::tempdir().unwrap();
        let (_k, vpn, store) = stub(dir.path());
        let c = create(&vpn, &store, BackupKind::Auto, None, None, false, 7)
            .await
            .unwrap();
        assert!(!c.row.has_db);
        assert_eq!(c.row.groups, None);
        assert_eq!(c.row.actor, None);
    }

    #[tokio::test]
    #[serial]
    async fn rotate_keeps_pinned_and_newest() {
        let dir = tempfile::tempdir().unwrap();
        let (_k, vpn, store) = stub(dir.path());
        let mut names = Vec::new();
        for _ in 0..4 {
            let c = create(&vpn, &store, BackupKind::Auto, None, None, false, 100)
                .await
                .unwrap();
            names.push(c.row.name.clone());
            // created_at у Meta — целые секунды; сортировка reconcile/rotate
            // опирается на неё, так что между созданиями нужен зазор ≥ 1 c,
            // иначе два бандла в одну секунду упорядочатся по имени (ts с
            // произвольным суффиксом — не монотонный).
            tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
        }
        store.set_backup_pinned(&names[0], true); // самый старый закреплён
        let removed = rotate(&vpn, &store, 2);
        assert_eq!(removed, 1); // из трёх незакреплённых остаются 2 новых
        let left: Vec<_> = reconcile(&vpn, &store)
            .into_iter()
            .map(|r| r.name)
            .collect();
        assert!(left.contains(&names[0]));
        assert!(!left.contains(&names[1]));
        assert!(left.contains(&names[2]) && left.contains(&names[3]));
    }

    #[tokio::test]
    #[serial]
    async fn reconcile_drops_missing_rows_and_adopts_foreign_files() {
        let dir = tempfile::tempdir().unwrap();
        let (_k, vpn, store) = stub(dir.path());
        let c = create(
            &vpn,
            &store,
            BackupKind::Manual,
            Some(1),
            Some("c".into()),
            false,
            7,
        )
        .await
        .unwrap();
        // файл удалили руками
        std::fs::remove_file(bundle_path(
            &vpn,
            crate::backup::format::ts_from_bundle_name(&c.row.name).unwrap(),
        ))
        .unwrap();
        assert!(reconcile(&vpn, &store).is_empty());
        assert!(store.backup_row(&c.row.name).is_none());
        // подбросили чужой бандл — заводится из meta.json
        let c2 = create(
            &vpn,
            &store,
            BackupKind::Manual,
            Some(1),
            Some("orig".into()),
            false,
            7,
        )
        .await
        .unwrap();
        store.delete_backup_row(&c2.row.name);
        let rows = reconcile(&vpn, &store);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].comment.as_deref(), Some("orig"));
        assert!(rows[0].sha256.is_some());
    }

    #[tokio::test]
    #[serial]
    async fn verify_detects_corruption() {
        let dir = tempfile::tempdir().unwrap();
        let (_k, vpn, store) = stub(dir.path());
        let c = create(&vpn, &store, BackupKind::Manual, None, None, false, 7)
            .await
            .unwrap();
        let ts = crate::backup::format::ts_from_bundle_name(&c.row.name)
            .unwrap()
            .to_string();
        assert!(verify(&vpn, &store, &ts).unwrap());
        std::fs::OpenOptions::new()
            .append(true)
            .open(bundle_path(&vpn, &ts))
            .unwrap()
            .write_all(b"x")
            .unwrap();
        assert!(!verify(&vpn, &store, &ts).unwrap());
    }

    #[tokio::test]
    #[serial]
    async fn restore_bundle_with_db_calls_installer_and_restores_store() {
        let dir = tempfile::tempdir().unwrap();
        let (_k, vpn, store) = stub(dir.path());
        store.set_psk_default(true);
        let c = create(&vpn, &store, BackupKind::Manual, None, None, true, 7)
            .await
            .unwrap();
        let ts = crate::backup::format::ts_from_bundle_name(&c.row.name)
            .unwrap()
            .to_string();
        store.set_psk_default(false);
        let out = restore(&vpn, &store, &Key::Bundle(ts.clone()), true)
            .await
            .unwrap();
        assert!(out.awg && out.db);
        assert!(store.psk_default()); // БД вернулась к снимку
        let called = std::fs::read_to_string(dir.path().join("restore_called")).unwrap();
        assert!(called.trim().ends_with(".tar.gz"));
        assert!(called.contains("awg_backup_"));
        // только AWG
        store.set_psk_default(false);
        let out = restore(&vpn, &store, &Key::Bundle(ts), false)
            .await
            .unwrap();
        assert!(out.awg && !out.db);
        assert!(!store.psk_default());
    }

    #[tokio::test]
    #[serial]
    async fn restore_installer_snapshot_and_missing_key() {
        let dir = tempfile::tempdir().unwrap();
        let (_k, vpn, store) = stub(dir.path());
        let bdir = dir.path().join("backups");
        std::fs::create_dir_all(&bdir).unwrap();
        crate::backup::format::tests_support::installer_archive_to(
            &bdir.join("awg_backup_T.tar.gz"),
        );
        let out = restore(&vpn, &store, &Key::Installer("T".into()), true)
            .await
            .unwrap();
        assert!(out.awg && !out.db);
        assert!(matches!(
            restore(&vpn, &store, &Key::Bundle("nope".into()), false).await,
            Err(crate::error::Error::BackupNotFound)
        ));
    }

    #[tokio::test]
    #[serial]
    async fn delete_bundle_and_installer_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let (_k, vpn, store) = stub(dir.path());
        let c = create(&vpn, &store, BackupKind::Manual, None, None, false, 7)
            .await
            .unwrap();
        let ts = crate::backup::format::ts_from_bundle_name(&c.row.name)
            .unwrap()
            .to_string();
        delete(&vpn, &store, &Key::Bundle(ts.clone())).unwrap();
        assert!(!bundle_path(&vpn, &ts).exists());
        assert!(store.backup_row(&c.row.name).is_none());
        let bdir = dir.path().join("backups");
        std::fs::create_dir_all(&bdir).unwrap();
        std::fs::write(bdir.join("awg_backup_Z.tar.gz"), b"x").unwrap();
        delete(&vpn, &store, &Key::Installer("Z".into())).unwrap();
        assert!(installer_snapshots(&vpn).is_empty());
        assert!(matches!(
            delete(&vpn, &store, &Key::Installer("Z".into())),
            Err(crate::error::Error::BackupNotFound)
        ));
    }

    #[tokio::test]
    #[serial]
    async fn accept_upload_wraps_installer_archive_and_stores_bundle() {
        let dir = tempfile::tempdir().unwrap();
        let (_k, vpn, store) = stub(dir.path());
        let up = dir.path().join("awg_backup_2026-01-01_00-00-00.000.tar.gz");
        crate::backup::format::tests_support::installer_archive_to(&up);
        let row = accept_upload(&vpn, &store, &up, Some(5)).unwrap();
        assert_eq!(row.kind, BackupKind::Upload);
        assert!(!row.has_db);
        assert_eq!(row.name, "awgram_backup_2026-01-01_00-00-00.000.tar.gz");
        // повторная загрузка того же имени — суффикс
        let row2 = accept_upload(&vpn, &store, &up, Some(5)).unwrap();
        assert_eq!(row2.name, "awgram_backup_2026-01-01_00-00-00.000-1.tar.gz");
        // готовый бандл принимается как есть, чужое имя нормализуется по ts из meta
        let c = create(
            &vpn,
            &store,
            BackupKind::Manual,
            None,
            Some("moved".into()),
            true,
            7,
        )
        .await
        .unwrap();
        let src = bundle_path(
            &vpn,
            crate::backup::format::ts_from_bundle_name(&c.row.name).unwrap(),
        );
        let copy = dir.path().join("downloaded.tar.gz");
        std::fs::copy(&src, &copy).unwrap();
        delete(
            &vpn,
            &store,
            &Key::Bundle(
                crate::backup::format::ts_from_bundle_name(&c.row.name)
                    .unwrap()
                    .into(),
            ),
        )
        .unwrap();
        let row3 = accept_upload(&vpn, &store, &copy, Some(5)).unwrap();
        assert_eq!(row3.name, c.row.name);
        assert_eq!(row3.comment.as_deref(), Some("moved"));
        assert!(row3.has_db);
        // имя без ts (`awg_backup_.tar.gz`) даёт пустой ts_from_awg_name — он
        // не должен дойти до имени бандла как есть (иначе ключ карточки не
        // парсится обратно, и кнопки мертвы); должен подставиться сгенерированный.
        let empty_ts = dir.path().join("awg_backup_.tar.gz");
        crate::backup::format::tests_support::installer_archive_to(&empty_ts);
        let row4 = accept_upload(&vpn, &store, &empty_ts, Some(5)).unwrap();
        let ts4 = crate::backup::format::ts_from_bundle_name(&row4.name).unwrap();
        assert!(Key::parse(ts4).is_some());
        // мусор отклоняется
        std::fs::write(&copy, b"junk").unwrap();
        assert!(matches!(
            accept_upload(&vpn, &store, &copy, None),
            Err(crate::error::Error::BackupInvalid(_))
        ));
    }

    /// Meta для бандлов, которые тест собирает руками.
    fn handmade_meta(awg: &Path, has_db: bool) -> Meta {
        Meta {
            format: format::FORMAT_VERSION,
            awgram_version: "0.0.0-test".into(),
            created_at: now_epoch(),
            kind: "manual".into(),
            actor: None,
            comment: None,
            has_db,
            awg_archive: awg.file_name().unwrap().to_string_lossy().into_owned(),
            awg_sha256: format::sha256_file(awg).unwrap(),
            clients: 2,
            groups: None,
        }
    }

    #[tokio::test]
    #[serial]
    async fn accept_upload_replaces_unusable_ts_from_bundle_inner_name() {
        let dir = tempfile::tempdir().unwrap();
        let (_k, vpn, store) = stub(dir.path());
        // Бандл собран не нами: внутренний архив назван так, что ts из его
        // имени не годится для callback — `bk:card:<70 символов>` уходит за
        // 64-байтный лимит Telegram, и карточка со списком стали бы мёртвыми.
        let stage = tempfile::tempdir().unwrap();
        let long = "a".repeat(70);
        let awg = stage.path().join(format!("awg_backup_{long}.tar.gz"));
        crate::backup::format::tests_support::installer_archive_to(&awg);
        let bundle = stage.path().join("weird_bundle.tar.gz");
        format::build_bundle(&bundle, &awg, None, &handmade_meta(&awg, false)).unwrap();

        let row = accept_upload(&vpn, &store, &bundle, Some(7)).unwrap();
        let ts = format::ts_from_bundle_name(&row.name).unwrap();
        assert!(Key::parse(ts).is_some(), "ts {ts} не разбирается обратно");
        assert!(
            format!("bk:card:{}", Key::Bundle(ts.into()).encode()).len() <= 64,
            "callback не влезает в лимит: {ts}"
        );
        assert!(
            !row.name.contains(&long),
            "длинный ts утёк в имя {}",
            row.name
        );
        // и по этому ключу бэкап находится
        assert!(find(&vpn, &store, &Key::Bundle(ts.into())).is_some());
        // принятый файл лежит с правами 0600: внутри ключи сервера
        let mode = std::fs::metadata(bundle_path(&vpn, ts))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "принятый бандл должен быть 0600");
    }

    #[tokio::test]
    #[serial]
    async fn reconcile_skips_file_with_unusable_ts() {
        let dir = tempfile::tempdir().unwrap();
        let (_k, vpn, store) = stub(dir.path());
        let stage = tempfile::tempdir().unwrap();
        let awg = stage
            .path()
            .join("awg_backup_2026-01-01_00-00-00.000.tar.gz");
        crate::backup::format::tests_support::installer_archive_to(&awg);
        // Файл подброшен в каталог руками, ts в имени — 70 символов.
        let bot_dir = vpn.bot_backups_dir();
        std::fs::create_dir_all(&bot_dir).unwrap();
        let bad = bot_dir.join(format::bundle_name(&"b".repeat(70)));
        format::build_bundle(&bad, &awg, None, &handmade_meta(&awg, false)).unwrap();
        assert!(reconcile(&vpn, &store).is_empty());
        assert!(
            bad.exists(),
            "чужой файл не трогаем, только не подхватываем"
        );
    }

    #[tokio::test]
    #[serial]
    async fn restore_reports_db_failure_instead_of_failing_whole_restore() {
        let dir = tempfile::tempdir().unwrap();
        let (_k, vpn, store) = stub(dir.path());
        let awg = dir.path().join("awg_no_db.tar.gz");
        crate::backup::format::tests_support::installer_archive_to(&awg);
        // meta обещает БД, а записи awgram.db в бандле нет: извлечение
        // сорвётся уже ПОСЛЕ восстановления AWG.
        let ts = "2026-03-04_05-06-07.000";
        std::fs::create_dir_all(vpn.bot_backups_dir()).unwrap();
        format::build_bundle(
            &bundle_path(&vpn, ts),
            &awg,
            None,
            &handmade_meta(&awg, true),
        )
        .unwrap();
        store.set_psk_default(true);

        let out = restore(&vpn, &store, &Key::Bundle(ts.into()), true)
            .await
            .unwrap();
        assert!(out.awg, "AWG должен быть восстановлен");
        assert!(!out.db);
        assert!(
            out.db_error.is_some(),
            "провал БД должен быть виден вызывающему"
        );
        // живая БД не тронута
        assert!(store.psk_default());
        // и инсталлер действительно звали
        assert!(dir.path().join("restore_called").exists());
    }

    #[tokio::test]
    #[serial]
    async fn restore_reports_db_failure_when_snapshot_breaks_migration() {
        let dir = tempfile::tempdir().unwrap();
        let (_k, vpn, store) = stub(dir.path());
        let awg = dir.path().join("awg_bad_db.tar.gz");
        crate::backup::format::tests_support::installer_archive_to(&awg);
        // Снимок проходит проверки формата (integrity_check ok, схема не
        // новее текущей), но `Store::restore_from` на нём падает: батч v1
        // создаёт `settings` без IF NOT EXISTS, а таблица уже есть.
        let db = dir.path().join("broken_snapshot.db");
        {
            let c = rusqlite::Connection::open(&db).unwrap();
            c.execute_batch(
                "CREATE TABLE meta(key TEXT PRIMARY KEY, value TEXT NOT NULL);
                 INSERT INTO meta VALUES('schema_version','0');
                 CREATE TABLE settings(key TEXT PRIMARY KEY, value TEXT NOT NULL);",
            )
            .unwrap();
        }
        let ts = "2026-03-04_05-06-08.000";
        std::fs::create_dir_all(vpn.bot_backups_dir()).unwrap();
        format::build_bundle(
            &bundle_path(&vpn, ts),
            &awg,
            Some(&db),
            &handmade_meta(&awg, true),
        )
        .unwrap();

        let out = restore(&vpn, &store, &Key::Bundle(ts.into()), true)
            .await
            .unwrap();
        assert!(out.awg && !out.db);
        assert!(out.db_error.is_some());
    }

    #[test]
    fn free_bytes_returns_some_for_tmp() {
        let d = tempfile::tempdir().unwrap();
        assert!(free_bytes(d.path()).is_some());
        assert!(free_bytes(Path::new("/definitely/missing/dir")).is_none());
    }

    #[tokio::test]
    #[serial]
    async fn create_fails_when_not_enough_space() {
        let dir = tempfile::tempdir().unwrap();
        let (_k, vpn, store) = stub(dir.path());
        // Каталог бота должен существовать заранее, иначе free_bytes не
        // сможет его опросить (df на несуществующем пути — None) и проверка
        // тихо пропустится.
        std::fs::create_dir_all(vpn.bot_backups_dir()).unwrap();
        let fake = BackupRow {
            name: "awgram_backup_prev.tar.gz".into(),
            created_at: now_epoch(),
            kind: BackupKind::Manual,
            actor: None,
            comment: None,
            pinned: false,
            size: u64::MAX / 4,
            sha256: None,
            has_db: false,
            clients: Some(1),
            groups: None,
        };
        store.upsert_backup(&fake);
        let err = create(&vpn, &store, BackupKind::Manual, None, None, false, 7)
            .await
            .unwrap_err();
        assert!(matches!(err, crate::error::Error::BackupNoSpace { .. }));
        // ни бандла, ни архива инсталлера не появилось — скрипт не вызывался
        assert!(installer_snapshots(&vpn).is_empty());
        assert_eq!(std::fs::read_dir(vpn.bot_backups_dir()).unwrap().count(), 0);
    }

    #[tokio::test]
    #[serial]
    async fn reconcile_keeps_pinned_row_when_dir_unreadable_but_drops_when_legitimately_empty() {
        let dir = tempfile::tempdir().unwrap();
        let (_k, vpn, store) = stub(dir.path());
        let c = create(&vpn, &store, BackupKind::Manual, None, None, false, 7)
            .await
            .unwrap();
        store.set_backup_pinned(&c.row.name, true);
        let bot_dir = vpn.bot_backups_dir();
        // Каталог awgram/ подменяем обычным файлом того же имени — read_dir
        // провалится (ENOTDIR), а не вернёт «пусто».
        std::fs::remove_dir_all(&bot_dir).unwrap();
        std::fs::write(&bot_dir, b"not a directory").unwrap();
        let rows = reconcile(&vpn, &store);
        assert_eq!(rows.len(), 1);
        assert!(rows[0].pinned);
        assert!(store.backup_row(&c.row.name).unwrap().pinned);
        // а вот легитимно пустой (существующий, но без файлов) каталог — уже
        // повод удалить строку.
        std::fs::remove_file(&bot_dir).unwrap();
        std::fs::create_dir_all(&bot_dir).unwrap();
        let rows2 = reconcile(&vpn, &store);
        assert!(rows2.is_empty());
        assert!(store.backup_row(&c.row.name).is_none());
    }

    #[tokio::test]
    #[serial]
    async fn restore_and_extract_inner_reject_bundle_with_wrong_inner_sha() {
        let dir = tempfile::tempdir().unwrap();
        let (_k, vpn, store) = stub(dir.path());
        let awg = dir.path().join("awg_for_tamper.tar.gz");
        crate::backup::format::tests_support::installer_archive_to(&awg);
        let ts = "2026-01-02_00-00-00.000";
        let meta = crate::backup::format::Meta {
            format: crate::backup::format::FORMAT_VERSION,
            awgram_version: "0.0.0-test".into(),
            created_at: now_epoch(),
            kind: "manual".into(),
            actor: None,
            comment: None,
            has_db: false,
            awg_archive: "awg_backup_2026-01-02_00-00-00.000.tar.gz".into(),
            // сознательно не совпадает с реальным sha256 внутреннего архива
            awg_sha256: "deadbeef".into(),
            clients: 2,
            groups: None,
        };
        std::fs::create_dir_all(vpn.bot_backups_dir()).unwrap();
        format::build_bundle(&bundle_path(&vpn, ts), &awg, None, &meta).unwrap();

        assert!(matches!(
            extract_inner(&vpn, ts),
            Err(crate::error::Error::BackupInvalid(_))
        ));
        assert!(matches!(
            restore(&vpn, &store, &Key::Bundle(ts.into()), false).await,
            Err(crate::error::Error::BackupInvalid(_))
        ));
    }
}
