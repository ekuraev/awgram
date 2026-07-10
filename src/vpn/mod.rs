pub mod model;
pub mod runner;
pub mod validate;

use std::path::PathBuf;

use crate::config::Config;
use crate::error::Result;
use model::{AddResult, Client};
use runner::{run, run_capture, RunSpec};

pub struct Vpn {
    script: PathBuf,
    sudo_prefix: String,
    timeout_secs: u64,
    clients_dir: PathBuf,
}

impl Vpn {
    pub fn from_config(cfg: &Config) -> Vpn {
        Vpn {
            script: cfg.manage_script.clone(),
            sudo_prefix: cfg.sudo_prefix.clone(),
            timeout_secs: cfg.op_timeout_secs,
            clients_dir: cfg.clients_dir.clone(),
        }
    }

    fn spec(&self) -> RunSpec<'_> {
        RunSpec { script: &self.script, sudo_prefix: &self.sudo_prefix, timeout_secs: self.timeout_secs }
    }

    pub async fn list(&self) -> Result<Vec<Client>> {
        let out = run(&self.spec(), &["list", "--json"]).await?;
        model::parse_client_list(&out).map_err(|e| crate::error::Error::Parse(e.to_string()))
    }

    pub async fn stats(&self) -> Result<Vec<Client>> {
        let out = run(&self.spec(), &["stats", "--json"]).await?;
        model::parse_client_list(&out).map_err(|e| crate::error::Error::Parse(e.to_string()))
    }

    pub async fn add(&self, name: &str, expires: Option<&str>) -> Result<AddResult> {
        let name = validate::validate_name(name).map_err(|e| crate::error::Error::Parse(e.to_string()))?;
        let mut args: Vec<String> = vec!["add".into(), name.clone()];
        if let Some(exp) = expires {
            let exp = validate::validate_expiry(exp).map_err(|e| crate::error::Error::Parse(e.to_string()))?;
            args.push(format!("--expires={exp}"));
        }
        let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        // `add` prints no JSON — just human-readable logs. The created files are
        // read back from `clients_dir` afterwards.
        run(&self.spec(), &arg_refs).await?;
        self.existing_files(&name)
    }

    pub async fn remove(&self, name: &str) -> Result<()> {
        let name = validate::validate_name(name).map_err(|e| crate::error::Error::Parse(e.to_string()))?;
        run(&self.spec(), &["remove", &name]).await?;
        Ok(())
    }

    /// Повторная выдача уже созданных файлов клиента из `clients_dir` (для кнопки «📄 Конфиг»
    /// и как последний шаг `add`). Только `.conf` обязателен — QR (`.png`) и ссылка
    /// (`.vpnuri`) создаются скриптом условно (например, если `qrencode` не установлен).
    pub fn existing_files(&self, name: &str) -> Result<AddResult> {
        let name = validate::validate_name(name).map_err(|e| crate::error::Error::Parse(e.to_string()))?;
        let conf = self.clients_dir.join(format!("{name}.conf"));
        let qr = self.clients_dir.join(format!("{name}.png"));
        let uri_path = self.clients_dir.join(format!("{name}.vpnuri"));
        if !conf.exists() {
            return Err(crate::error::Error::Parse("файлы клиента не найдены".into()));
        }
        let uri = std::fs::read_to_string(&uri_path).unwrap_or_default().trim().to_string();
        Ok(AddResult {
            name,
            conf_path: conf.to_string_lossy().into_owned(),
            qr_path: qr.to_string_lossy().into_owned(),
            uri,
        })
    }

    /// Читает срок действия клиента из `<clients_dir>/expiry/<name>` (epoch, сек).
    /// None, если файла нет или содержимое не парсится (значит — бессрочно).
    pub fn client_expiry(&self, name: &str) -> Option<i64> {
        let name = validate::validate_name(name).ok()?;
        let path = self.clients_dir.join("expiry").join(&name);
        let raw = std::fs::read_to_string(path).ok()?;
        raw.trim().parse::<i64>().ok()
    }

    fn backups_dir(&self) -> PathBuf {
        self.clients_dir.join("backups")
    }

    /// Читает `clients_dir/backups/`, отбирая только `*.tar.gz`, отсортированные по mtime убыв.
    pub fn list_backups(&self) -> Result<Vec<BackupFile>> {
        let dir = self.backups_dir();
        let mut out = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for e in entries.flatten() {
                let path = e.path();
                let name = e.file_name().to_string_lossy().into_owned();
                if !name.ends_with(".tar.gz") {
                    continue;
                }
                let meta = match e.metadata() {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                let mtime = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                out.push(BackupFile { name, path, size: meta.len(), mtime });
            }
        }
        out.sort_by(|a, b| b.mtime.cmp(&a.mtime));
        Ok(out)
    }

    /// Запускает `backup` и возвращает свежесозданный архив (самый новый по mtime).
    pub async fn backup(&self) -> Result<BackupFile> {
        run(&self.spec(), &["backup"]).await?;
        self.list_backups()?
            .into_iter()
            .next()
            .ok_or_else(|| crate::error::Error::Parse("бэкап не найден после создания".into()))
    }

    /// Восстанавливает из бэкапа по индексу в списке `list_backups()` (0 = самый новый).
    pub async fn restore(&self, index: usize) -> Result<()> {
        let backups = self.list_backups()?;
        let bf = backups.get(index).ok_or_else(|| crate::error::Error::Parse("бэкап не найден".into()))?;
        // basename-валидация: имя без разделителей пути и по шаблону
        if bf.name.contains('/') || !bf.name.starts_with("awg_backup_") || !bf.name.ends_with(".tar.gz") {
            return Err(crate::error::Error::Parse("некорректное имя бэкапа".into()));
        }
        let path = bf.path.to_string_lossy().into_owned();
        run(&self.spec(), &["restore", &path]).await?;
        Ok(())
    }

    /// Запускает `check` и возвращает stdout независимо от кода выхода
    /// (ненулевой код означает «обнаружены проблемы», а не ошибку выполнения).
    pub async fn check(&self) -> Result<String> {
        let (out, _code) = run_capture(&self.spec(), &["check"]).await?;
        Ok(out)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BackupFile {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub mtime: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;

    fn vpn_with_script(body: &str) -> (tempfile::TempDir, Vpn) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fake.sh");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(body.as_bytes()).unwrap();
        let mut perm = std::fs::metadata(&path).unwrap().permissions();
        perm.set_mode(0o755);
        std::fs::set_permissions(&path, perm).unwrap();
        let vpn = Vpn {
            script: path,
            sudo_prefix: String::new(),
            timeout_secs: 5,
            clients_dir: dir.path().to_path_buf(),
        };
        (dir, vpn)
    }

    #[tokio::test]
    async fn list_parses_stub_output() {
        let (_d, vpn) = vpn_with_script(
            "#!/bin/sh\necho '[{\"name\":\"alice\",\"status_code\":\"active\"}]'\n",
        );
        let clients = vpn.list().await.unwrap();
        assert_eq!(clients.len(), 1);
        assert_eq!(clients[0].name, "alice");
        assert!(clients[0].active());
    }

    #[tokio::test]
    async fn add_rejects_bad_name_before_running() {
        let (_d, vpn) = vpn_with_script("#!/bin/sh\necho should-not-run 1>&2\nexit 1\n");
        let err = vpn.add("bad name;rm", None).await.unwrap_err();
        // Ошибка валидации, а не запуска скрипта.
        assert!(matches!(err, crate::error::Error::Parse(_)));
    }

    #[tokio::test]
    async fn add_runs_script_then_reads_created_conf() {
        // Real `add` prints no JSON — just logs — and creates `<name>.conf` on disk.
        let (dir, vpn) = vpn_with_script("#!/bin/sh\nexit 0\n");
        std::fs::write(dir.path().join("alice.conf"), "conf").unwrap();
        let res = vpn.add("alice", None).await.unwrap();
        assert!(res.conf_path.ends_with("alice.conf"));
        assert_eq!(res.uri, "");
    }

    #[tokio::test]
    async fn add_errors_when_script_did_not_create_conf() {
        let (_d, vpn) = vpn_with_script("#!/bin/sh\nexit 0\n");
        let err = vpn.add("alice", None).await.unwrap_err();
        assert!(matches!(err, crate::error::Error::Parse(_)));
    }

    #[test]
    fn existing_files_returns_paths_when_conf_present() {
        let (dir, vpn) = vpn_with_script("#!/bin/sh\n");
        std::fs::write(dir.path().join("alice.conf"), "conf").unwrap();
        let res = vpn.existing_files("alice").unwrap();
        assert!(res.conf_path.ends_with("alice.conf"));
        assert!(res.qr_path.ends_with("alice.png"));
        assert_eq!(res.uri, "");
    }

    #[test]
    fn existing_files_reads_optional_qr_and_uri_when_present() {
        let (dir, vpn) = vpn_with_script("#!/bin/sh\n");
        std::fs::write(dir.path().join("alice.conf"), "conf").unwrap();
        std::fs::write(dir.path().join("alice.png"), "png").unwrap();
        std::fs::write(dir.path().join("alice.vpnuri"), "vpn://x\n").unwrap();
        let res = vpn.existing_files("alice").unwrap();
        assert!(res.qr_path.ends_with("alice.png"));
        assert_eq!(res.uri, "vpn://x");
    }

    #[test]
    fn existing_files_errors_when_missing() {
        let (_d, vpn) = vpn_with_script("#!/bin/sh\n");
        assert!(matches!(vpn.existing_files("ghost"), Err(crate::error::Error::Parse(_))));
    }

    #[test]
    fn client_expiry_reads_epoch_from_file() {
        let (dir, vpn) = vpn_with_script("#!/bin/sh\n");
        std::fs::create_dir_all(dir.path().join("expiry")).unwrap();
        std::fs::write(dir.path().join("expiry").join("alice"), "1893456000").unwrap();
        assert_eq!(vpn.client_expiry("alice"), Some(1893456000));
    }

    #[test]
    fn client_expiry_none_when_file_missing() {
        let (_d, vpn) = vpn_with_script("#!/bin/sh\n");
        assert_eq!(vpn.client_expiry("bob"), None);
    }

    #[test]
    fn client_expiry_none_when_content_unparseable() {
        let (dir, vpn) = vpn_with_script("#!/bin/sh\n");
        std::fs::create_dir_all(dir.path().join("expiry")).unwrap();
        std::fs::write(dir.path().join("expiry").join("carol"), "not-a-number").unwrap();
        assert_eq!(vpn.client_expiry("carol"), None);
    }

    #[test]
    fn client_expiry_rejects_traversal_name() {
        let (_d, vpn) = vpn_with_script("#!/bin/sh\n");
        assert_eq!(vpn.client_expiry("../etc/passwd"), None);
        assert_eq!(vpn.client_expiry("a/b"), None);
    }

    #[tokio::test]
    async fn backup_returns_newest_archive() {
        // заглушка создаёт файл в clients_dir/backups/
        let (dir, vpn) = vpn_with_script(
            "#!/bin/sh\nmkdir -p \"$(dirname \"$0\")/../backups\" 2>/dev/null; true\n",
        );
        let bdir = dir.path().join("backups");
        std::fs::create_dir_all(&bdir).unwrap();
        std::fs::write(bdir.join("awg_backup_2026-01-01_00-00-00.000Z.tar.gz"), b"x").unwrap();
        let bf = vpn.backup().await.unwrap();
        assert!(bf.name.ends_with(".tar.gz"));
    }

    #[test]
    fn list_backups_sorted_and_filtered() {
        let (dir, vpn) = vpn_with_script("#!/bin/sh\n");
        let bdir = dir.path().join("backups");
        std::fs::create_dir_all(&bdir).unwrap();
        std::fs::write(bdir.join("awg_backup_a.tar.gz"), b"x").unwrap();
        std::fs::write(bdir.join("note.txt"), b"x").unwrap(); // должен быть отфильтрован
        let list = vpn.list_backups().unwrap();
        assert_eq!(list.len(), 1);
        assert!(list[0].name.ends_with(".tar.gz"));
    }

    #[tokio::test]
    async fn restore_rejects_out_of_range() {
        let (_d, vpn) = vpn_with_script("#!/bin/sh\n");
        assert!(matches!(vpn.restore(999).await, Err(crate::error::Error::Parse(_))));
    }

    #[tokio::test]
    async fn check_returns_output_even_on_problems() {
        let (_d, vpn) = vpn_with_script("#!/bin/sh\necho 'ПРОБЛЕМЫ'\nexit 1\n");
        let out = vpn.check().await.unwrap();
        assert!(out.contains("ПРОБЛЕМЫ"));
    }
}
