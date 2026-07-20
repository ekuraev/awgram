pub mod model;
pub mod runner;
pub mod validate;
pub mod wire;

use std::path::PathBuf;

use crate::config::Config;
use crate::error::Result;
use model::{AddResult, Client};
use runner::{run, RunSpec};

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
        RunSpec {
            script: &self.script,
            sudo_prefix: &self.sudo_prefix,
            timeout_secs: self.timeout_secs,
            extra_env: &[],
        }
    }

    pub async fn list(&self) -> Result<Vec<Client>> {
        let (out, code) = run(&self.spec(), &["list", "--json"]).await?;
        // list печатает голый JSON-массив — нет status-конверта. Ненулевой exit
        // всегда означает ошибку выполнения; вывод уходит в stderr-контекст.
        if code != 0 {
            return Err(crate::error::Error::ScriptFailed {
                code: Some(code),
                stderr: out,
            });
        }
        model::parse_client_list(&out).map_err(|e| crate::error::Error::Parse(e.to_string()))
    }

    pub async fn stats(&self) -> Result<Vec<Client>> {
        let (out, code) = run(&self.spec(), &["stats", "--json"]).await?;
        // stats печатает голый JSON-массив — нет status-конверта. Ненулевой exit
        // всегда означает ошибку выполнения; вывод уходит в stderr-контекст.
        if code != 0 {
            return Err(crate::error::Error::ScriptFailed {
                code: Some(code),
                stderr: out,
            });
        }
        model::parse_client_list(&out).map_err(|e| crate::error::Error::Parse(e.to_string()))
    }

    /// Проверяет, существует ли клиент с таким именем (через `list --json`).
    /// Авторитетно: отражает реальное состояние WireGuard, а не только файлы на диске.
    pub async fn exists(&self, name: &str) -> Result<bool> {
        let name =
            validate::validate_name(name).map_err(|e| crate::error::Error::Parse(e.to_string()))?;
        let clients = self.list().await?;
        Ok(clients.iter().any(|c| c.name == name))
    }

    pub async fn add(&self, name: &str, expires: Option<&str>, psk: bool) -> Result<AddResult> {
        let name =
            validate::validate_name(name).map_err(|e| crate::error::Error::Parse(e.to_string()))?;
        let mut args: Vec<String> = vec!["add".into(), name.clone(), "--json".into()];
        if let Some(exp) = expires {
            let exp = validate::validate_expiry(exp)
                .map_err(|e| crate::error::Error::Parse(e.to_string()))?;
            args.push(format!("--expires={exp}"));
        }
        if psk {
            args.push("--psk".into());
        }
        let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let (out, code) = run(&self.spec(), &arg_refs).await?;
        // add печатает JSON-конверт ДАЖЕ при exit 1 (status:"exists" и т.п.) —
        // парсим всегда. Но пустой stdout — скрипт упал ДО печати, это не JSON.
        if out.trim().is_empty() {
            return Err(crate::error::Error::ScriptFailed {
                code: Some(code),
                stderr: format!("add: пустой stdout (exit {code})"),
            });
        }
        let parsed =
            wire::parse_add(&out).map_err(|e| crate::error::Error::Parse(e.to_string()))?;
        let entry = parsed
            .results
            .into_iter()
            .next()
            .ok_or_else(|| crate::error::Error::Parse("add: пустой results[]".into()))?;
        match entry.status {
            wire::AddStatus::Created => Ok(AddResult {
                name: entry.name,
                conf_path: entry.conf.unwrap_or_default(),
                qr_path: entry.qr.unwrap_or_default(),
                uri: read_vpnuri_content(&entry.vpnuri.unwrap_or_default()),
            }),
            wire::AddStatus::Exists => Err(crate::error::Error::ClientExists(name)),
            wire::AddStatus::InvalidName => {
                Err(crate::error::Error::Parse("add: невалидное имя".into()))
            }
            wire::AddStatus::Error | wire::AddStatus::Unknown => Err(crate::error::Error::Parse(
                "add: ошибка создания клиента".into(),
            )),
        }
    }

    pub async fn remove(&self, name: &str) -> Result<()> {
        let name =
            validate::validate_name(name).map_err(|e| crate::error::Error::Parse(e.to_string()))?;
        let spec = RunSpec {
            script: &self.script,
            sudo_prefix: &self.sudo_prefix,
            timeout_secs: self.timeout_secs,
            extra_env: &[("AWG_STRICT_CONFIRM", "1")],
        };
        let args = ["remove", &name, "--json", "--yes"];
        let (out, code) = run(&spec, &args).await?;
        // remove печатает JSON-конверт при exit 1 (status:"not_found") — парсим всегда.
        if out.trim().is_empty() {
            return Err(crate::error::Error::ScriptFailed {
                code: Some(code),
                stderr: format!("remove: пустой stdout (exit {code})"),
            });
        }
        let parsed =
            wire::parse_remove(&out).map_err(|e| crate::error::Error::Parse(e.to_string()))?;
        let entry = parsed
            .results
            .into_iter()
            .next()
            .ok_or_else(|| crate::error::Error::Parse("remove: пустой results[]".into()))?;
        match entry.status {
            wire::RemoveStatus::Removed => Ok(()),
            wire::RemoveStatus::NotFound => Err(crate::error::Error::ClientNotFound(name)),
            _ => Err(crate::error::Error::Parse("remove: ошибка".into())),
        }
    }

    /// Перевыпускает файлы одного клиента (`regen <name>`): ключи и IP
    /// сохраняются, `.conf`/QR/URI создаются заново. Пути берутся из JSON-конверта
    /// v5.21.0 (`results[0].conf/qr/vpnuri`), а не угадываются по имени.
    pub async fn regen_client(&self, name: &str) -> Result<AddResult> {
        let name =
            validate::validate_name(name).map_err(|e| crate::error::Error::Parse(e.to_string()))?;
        let args = ["regen", &name, "--json"];
        let (out, code) = run(&self.spec(), &args).await?;
        // regen печатает JSON-конверт при exit 1 (status:"not_found") — парсим всегда.
        if out.trim().is_empty() {
            return Err(crate::error::Error::ScriptFailed {
                code: Some(code),
                stderr: format!("regen: пустой stdout (exit {code})"),
            });
        }
        let parsed =
            wire::parse_regen(&out).map_err(|e| crate::error::Error::Parse(e.to_string()))?;
        let entry = parsed
            .results
            .into_iter()
            .next()
            .ok_or_else(|| crate::error::Error::Parse("regen: пустой results[]".into()))?;
        match entry.status {
            wire::RegenStatus::Regenerated => Ok(AddResult {
                name: entry.name,
                conf_path: entry.conf.unwrap_or_default(),
                qr_path: entry.qr.unwrap_or_default(),
                uri: read_vpnuri_content(&entry.vpnuri.unwrap_or_default()),
            }),
            wire::RegenStatus::NotFound => Err(crate::error::Error::ClientNotFound(name)),
            _ => Err(crate::error::Error::Parse("regen: ошибка".into())),
        }
    }

    /// Перевыпускает файлы всех клиентов. Различает «нет клиентов» (no-op),
    /// полный успех и частичный провал — UI показывает разные сообщения.
    /// Таймаут ×3 — массовый regen пропорционален числу клиентов.
    pub async fn regen_all(&self, reset_routes: bool) -> Result<RegenAllOutcome> {
        let spec = RunSpec {
            script: &self.script,
            sudo_prefix: &self.sudo_prefix,
            timeout_secs: self.timeout_secs * 3,
            extra_env: &[("AWG_STRICT_CONFIRM", "1")],
        };
        let mut args: Vec<&str> = vec!["regen"];
        if reset_routes {
            args.push("--reset-routes");
        }
        args.push("--json");
        args.push("--yes");
        let (out, code) = run(&spec, &args).await?;
        // regen_all печатает JSON при partial-failure (ok:false, failed>0) — exit 1,
        // но regenerated/failed авторитетны. Парсим всегда.
        if out.trim().is_empty() {
            return Err(crate::error::Error::ScriptFailed {
                code: Some(code),
                stderr: format!("regen: пустой stdout (exit {code})"),
            });
        }
        let parsed =
            wire::parse_regen(&out).map_err(|e| crate::error::Error::Parse(e.to_string()))?;
        if parsed.regenerated == 0 && parsed.failed == 0 {
            Ok(RegenAllOutcome::NoClients)
        } else if parsed.failed == 0 {
            Ok(RegenAllOutcome::Done(parsed.regenerated))
        } else {
            Ok(RegenAllOutcome::Partial {
                ok: parsed.regenerated,
                failed: parsed.failed,
            })
        }
    }

    /// Повторная выдача уже созданных файлов клиента из `clients_dir` (для кнопки «📄 Конфиг»
    /// и как последний шаг `add`). Только `.conf` обязателен — QR (`.png`) и ссылка
    /// (`.vpnuri`) создаются скриптом условно (например, если `qrencode` не установлен).
    pub fn existing_files(&self, name: &str) -> Result<AddResult> {
        let name =
            validate::validate_name(name).map_err(|e| crate::error::Error::Parse(e.to_string()))?;
        let conf = self.clients_dir.join(format!("{name}.conf"));
        let qr = self.clients_dir.join(format!("{name}.png"));
        let uri_path = self.clients_dir.join(format!("{name}.vpnuri"));
        if !conf.exists() {
            return Err(crate::error::Error::Parse(
                "файлы клиента не найдены".into(),
            ));
        }
        let uri = std::fs::read_to_string(&uri_path)
            .unwrap_or_default()
            .trim()
            .to_string();
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
                if !meta.is_file() {
                    // Директория с именем вида "x.tar.gz" не должна попадать в список
                    // бэкапов — только обычные файлы.
                    continue;
                }
                let mtime = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                out.push(BackupFile {
                    name,
                    path,
                    size: meta.len(),
                    mtime,
                });
            }
        }
        out.sort_by_key(|b| std::cmp::Reverse(b.mtime));
        Ok(out)
    }

    /// Запускает `backup` и возвращает свежесозданный архив.
    /// Путь берётся из JSON-конверта v5.21.0 (`BackupOut.path`), а не
    /// угадывается как «новейший .tar.gz по mtime».
    pub async fn backup(&self) -> Result<BackupFile> {
        let (out, code) = run(&self.spec(), &["backup", "--json"]).await?;
        if out.trim().is_empty() {
            return Err(crate::error::Error::ScriptFailed {
                code: Some(code),
                stderr: format!("backup: пустой stdout (exit {code})"),
            });
        }
        let parsed =
            wire::parse_backup(&out).map_err(|e| crate::error::Error::Parse(e.to_string()))?;
        let path = std::path::PathBuf::from(&parsed.path);
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| parsed.path.clone());
        let meta = std::fs::metadata(&path)
            .map_err(|e| crate::error::Error::Parse(format!("backup stat: {e}")))?;
        let size = parsed.size_bytes.unwrap_or(meta.len());
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        Ok(BackupFile {
            name,
            path,
            size,
            mtime,
        })
    }

    /// Восстанавливает из бэкапа по индексу в списке `list_backups()` (0 = самый новый).
    /// Парсит JSON-конверт: `rolled_back:true → RestoreRolledBack`,
    /// `ok:false && !rolled_back → ScriptFailed`. `AWG_STRICT_CONFIRM=1` + `--yes`.
    pub async fn restore(&self, index: usize) -> Result<()> {
        let backups = self.list_backups()?;
        let bf = backups
            .get(index)
            .ok_or_else(|| crate::error::Error::Parse("бэкап не найден".into()))?;
        if bf.name.contains('/')
            || !bf.name.starts_with("awg_backup_")
            || !bf.name.ends_with(".tar.gz")
        {
            return Err(crate::error::Error::Parse("некорректное имя бэкапа".into()));
        }
        let spec = RunSpec {
            script: &self.script,
            sudo_prefix: &self.sudo_prefix,
            timeout_secs: self.timeout_secs,
            extra_env: &[("AWG_STRICT_CONFIRM", "1")],
        };
        let path = bf.path.to_string_lossy().into_owned();
        let (out, code) = run(&spec, &["restore", &path, "--json", "--yes"]).await?;
        // restore печатает {"rolled_back":true} при exit 1 — парсим всегда.
        if out.trim().is_empty() {
            return Err(crate::error::Error::ScriptFailed {
                code: Some(code),
                stderr: format!("restore: пустой stdout (exit {code})"),
            });
        }
        let parsed =
            wire::parse_restore(&out).map_err(|e| crate::error::Error::Parse(e.to_string()))?;
        if parsed.ok == Some(true) {
            Ok(())
        } else if parsed.rolled_back {
            Err(crate::error::Error::RestoreRolledBack)
        } else {
            tracing::error!(
                error = ?parsed.error,
                "restore провалился без отката"
            );
            Err(crate::error::Error::ScriptFailed {
                code: None,
                stderr: parsed.error.unwrap_or_else(|| "restore failed".into()),
            })
        }
    }

    /// Запускает `check --json` и возвращает структурированный отчёт v5.21.0.
    /// check печатает JSON в stdout даже при ok:false (обнаружены проблемы);
    /// run() возвращает stdout независимо от exit code — парсим конверт всегда
    /// (ok:false — это «проблемы найдены», а не ошибка выполнения).
    /// НО: при фатальной ошибке (die до check_server) инсталлер возвращает
    /// аварийный конверт {"ok":false,"error":"...","rc":N} без полей отчёта.
    /// Все блоки CheckReport имеют defaults → без этой проверки такой конверт
    /// десериализуется в фиктивный отчёт (неактивный сервис, ноль клиентов),
    /// и бот радостно покажет пользователю «всё сломано, но это норма».
    pub async fn check(&self) -> Result<wire::CheckReport> {
        let (out, _code) = run(&self.spec(), &["check", "--json"]).await?;
        if let Some(env) = wire::try_error_envelope(&out) {
            tracing::error!(error = ?env.error, rc = env.rc, "check: аварийный конверт");
            return Err(crate::error::Error::ScriptFailed {
                code: Some(env.rc),
                stderr: env.error,
            });
        }
        wire::parse_check(&out).map_err(|e| crate::error::Error::Parse(e.to_string()))
    }

    /// Запускает `diagnose` и возвращает stdout независимо от кода выхода
    /// (как `check`: ненулевой код — «найдены проблемы», а не ошибка).
    /// Пустой вывод — ошибка: диагностика всегда что-то печатает.
    pub async fn diagnose(&self) -> Result<String> {
        let (out, _code) = run(&self.spec(), &["diagnose"]).await?;
        if out.trim().is_empty() {
            return Err(crate::error::Error::Parse("пустой вывод diagnose".into()));
        }
        Ok(out)
    }

    /// Меняет один параметр клиента (`modify <name> <param> <value> --json`).
    /// Имя и param валидируются локально; CLI-имя param даёт `modify_param_cli`.
    pub async fn modify(
        &self,
        name: &str,
        param: validate::ModifyParam,
        value: &str,
    ) -> Result<wire::ModifyOut> {
        let name =
            validate::validate_name(name).map_err(|e| crate::error::Error::Parse(e.to_string()))?;
        let param_cli = validate::modify_param_cli(param);
        let args = ["modify", &name, param_cli, value, "--json"];
        let (out, code) = run(&self.spec(), &args).await?;
        if out.trim().is_empty() {
            return Err(crate::error::Error::ScriptFailed {
                code: Some(code),
                stderr: format!("modify: пустой stdout (exit {code})"),
            });
        }
        wire::parse_modify(&out).map_err(|e| crate::error::Error::Parse(e.to_string()))
    }

    /// Перезапускает юнит awg-quick (`restart --json --yes`). В v5.21.0 restart
    /// вызывает confirm_action — ставим AWG_STRICT_CONFIRM=1 и флаг --yes.
    pub async fn restart(&self) -> Result<wire::RestartOut> {
        let spec = RunSpec {
            script: &self.script,
            sudo_prefix: &self.sudo_prefix,
            timeout_secs: self.timeout_secs,
            extra_env: &[("AWG_STRICT_CONFIRM", "1")],
        };
        let (out, code) = run(&spec, &["restart", "--json", "--yes"]).await?;
        if out.trim().is_empty() {
            return Err(crate::error::Error::ScriptFailed {
                code: Some(code),
                stderr: format!("restart: пустой stdout (exit {code})"),
            });
        }
        wire::parse_restart(&out).map_err(|e| crate::error::Error::Parse(e.to_string()))
    }

    /// Чинит модуль ядра amneziawg (`repair-module --json`). Не деструктивно —
    /// без AWG_STRICT_CONFIRM. Возвращает код завершения ремонта (0 = чисто).
    /// P2.3: отдельный timeout 300с — DKMS rebuild + установка kernel headers
    /// заявлены инсталлером как операция до 5 минут (manage.sh: «может занять
    /// до 5 минут — DKMS rebuild»). Общий timeout 60с обрывал бы восстановление
    /// посреди apt-установки headers.
    pub async fn repair_module(&self) -> Result<wire::RepairOut> {
        let spec = RunSpec {
            script: &self.script,
            sudo_prefix: &self.sudo_prefix,
            timeout_secs: 300,
            extra_env: &[],
        };
        let (out, code) = run(&spec, &["repair-module", "--json"]).await?;
        // repair-module печатает JSON с rc:1/2 при exit 1 — парсим всегда.
        if out.trim().is_empty() {
            return Err(crate::error::Error::ScriptFailed {
                code: Some(code),
                stderr: format!("repair-module: пустой stdout (exit {code})"),
            });
        }
        wire::parse_repair(&out).map_err(|e| crate::error::Error::Parse(e.to_string()))
    }
}

/// Читает содержимое `.vpnuri`-файла (готовую ссылку `vpn://…`) по пути из
/// JSON-конверта v5.21.0. Поле `vpnuri` в конверте — это ПУТЬ к файлу, а не
/// сама ссылка; `send_client_files` показывает `AddResult.uri` как ссылку,
/// поэтому без чтения файла пользователь получал серверный путь вместо vpn://.
/// Пустой путь / отсутствующий файл → пустая строка (qr/vpnuri опциональны).
fn read_vpnuri_content(path: &str) -> String {
    if path.is_empty() {
        return String::new();
    }
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .trim()
        .to_string()
}

#[derive(Debug, Clone, PartialEq)]
pub struct BackupFile {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub mtime: i64,
}

/// Результат массового regen. Различает «нет клиентов» (no-op), полный успех
/// и частичный провал — UI показывает разные сообщения.
#[derive(Debug, Clone, PartialEq)]
pub enum RegenAllOutcome {
    /// Клиентов нет — скрипт завершился как empty no-op.
    NoClients,
    /// Все N клиентов перевыпущены успешно.
    Done(u32),
    /// Часть перевыпущена, часть провалилась (скрипт вернул ok:false).
    Partial { ok: u32, failed: u32 },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
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
    #[serial] // гонка ETXTBSY: параллельный fork удерживает write-fd чужого fake-скрипта до execve
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
    #[serial] // гонка ETXTBSY: параллельный fork удерживает write-fd чужого fake-скрипта до execve
    async fn exists_returns_true_for_existing_client() {
        let (_d, vpn) = vpn_with_script(
            "#!/bin/sh\necho '[{\"name\":\"alice\",\"status_code\":\"active\"}]'\n",
        );
        assert!(vpn.exists("alice").await.unwrap());
    }

    #[tokio::test]
    #[serial] // гонка ETXTBSY: параллельный fork удерживает write-fd чужого fake-скрипта до execve
    async fn exists_returns_false_for_missing_client() {
        let (_d, vpn) = vpn_with_script(
            "#!/bin/sh\necho '[{\"name\":\"alice\",\"status_code\":\"active\"}]'\n",
        );
        assert!(!vpn.exists("bob").await.unwrap());
    }

    #[tokio::test]
    #[serial] // гонка ETXTBSY: параллельный fork удерживает write-fd чужого fake-скрипта до execve
    async fn exists_rejects_bad_name() {
        let (_d, vpn) = vpn_with_script("#!/bin/sh\necho '[]'\n");
        assert!(vpn.exists("bad name;rm").await.is_err());
    }

    #[tokio::test]
    #[serial] // гонка ETXTBSY: параллельный fork удерживает write-fd чужого fake-скрипта до execve
    async fn exists_propagates_script_failure() {
        let (_d, vpn) = vpn_with_script("#!/bin/sh\nexit 1\n");
        assert!(vpn.exists("alice").await.is_err());
    }

    #[tokio::test]
    #[serial] // гонка ETXTBSY: параллельный fork удерживает write-fd чужого fake-скрипта до execve
    async fn add_rejects_bad_name_before_running() {
        let (_d, vpn) = vpn_with_script("#!/bin/sh\necho should-not-run 1>&2\nexit 1\n");
        let err = vpn.add("bad name;rm", None, false).await.unwrap_err();
        // Ошибка валидации, а не запуска скрипта.
        assert!(matches!(err, crate::error::Error::Parse(_)));
    }

    #[tokio::test]
    #[serial]
    async fn add_success_takes_paths_from_json() {
        // Stub эмитит add-конверт с путями — add() берёт их из JSON.
        // P2.1: vpnuri в конверте — ПУТЬ к файлу; add() читает его содержимое
        // (готовую ссылку vpn://…), а не отдаёт путь как ссылку.
        let dir = tempfile::tempdir().unwrap();
        let vpnuri_path = dir.path().join("alice.vpnuri");
        let vpnuri_str = vpnuri_path.to_string_lossy().to_string();
        std::fs::write(&vpnuri_path, "vpn://amnezia/x?k=secret\n").unwrap();
        let stub = format!(
            r#"#!/bin/sh
[ "$1" = add ] || exit 1
echo '{{"command":"add","ok":true,"added":1,"failed":0,"applied":true,"results":[{{"name":"alice","status":"created","conf":"/tmp/zz/alice.conf","qr":"/tmp/zz/alice.png","vpnuri":"{vpnuri_str}","expires_at":null}}]}}'
"#,
        );
        let script_path = dir.path().join("stub.sh");
        std::fs::write(&script_path, &stub).unwrap();
        let mut perm = std::fs::metadata(&script_path).unwrap().permissions();
        perm.set_mode(0o755);
        std::fs::set_permissions(&script_path, perm).unwrap();
        let vpn = Vpn {
            script: script_path,
            sudo_prefix: String::new(),
            timeout_secs: 5,
            clients_dir: dir.path().to_path_buf(),
        };
        let res = vpn.add("alice", None, false).await.unwrap();
        assert_eq!(res.conf_path, "/tmp/zz/alice.conf");
        assert_eq!(res.qr_path, "/tmp/zz/alice.png");
        // uri — СОДЕРЖИМОЕ .vpnuri-файла, а не путь к нему.
        assert_eq!(res.uri, "vpn://amnezia/x?k=secret");
    }

    #[tokio::test]
    #[serial]
    async fn add_uri_empty_when_vpnuri_file_missing() {
        // qr/vpnuri опциональны в конверте (qrencode может отсутствовать).
        // Если vpnuri-путь указан, но файла нет → uri = "" (не путь, не ошибка).
        let dir = tempfile::tempdir().unwrap();
        let stub = r#"#!/bin/sh
[ "$1" = add ] || exit 1
echo '{"command":"add","ok":true,"added":1,"failed":0,"applied":true,"results":[{"name":"alice","status":"created","conf":"/tmp/zz/alice.conf","qr":null,"vpnuri":"/nonexistent/alice.vpnuri","expires_at":null}]}'
"#;
        let script_path = dir.path().join("stub.sh");
        std::fs::write(&script_path, stub).unwrap();
        let mut perm = std::fs::metadata(&script_path).unwrap().permissions();
        perm.set_mode(0o755);
        std::fs::set_permissions(&script_path, perm).unwrap();
        let vpn = Vpn {
            script: script_path,
            sudo_prefix: String::new(),
            timeout_secs: 5,
            clients_dir: dir.path().to_path_buf(),
        };
        let res = vpn.add("alice", None, false).await.unwrap();
        assert_eq!(res.uri, "");
    }

    #[tokio::test]
    #[serial]
    async fn add_returns_client_exists_when_script_exits_nonzero() {
        // Реальный инсталлер v5.21.0 печатает {"status":"exists"} в stdout и
        // ЗАТЕМ выходит с rc=1 (_cmd_rc=1). add() должен распарсить JSON
        // независимо от exit code и вернуть Error::ClientExists.
        let stub = r#"#!/bin/sh
echo '{"command":"add","ok":true,"added":0,"failed":1,"applied":false,"results":[{"name":"alice","status":"exists"}]}'
exit 1
"#;
        let (_d, vpn) = vpn_with_script(stub);
        let err = vpn.add("alice", None, false).await.unwrap_err();
        assert!(
            matches!(err, crate::error::Error::ClientExists(_)),
            "got {err:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn add_error_envelope_becomes_parse_failure() {
        // ok:false с rc:1 — envelope означает ошибку создания; нет статуса exists,
        // нет пустого stdout → должна вернуть Parse (нет конкретной status-ветки).
        let stub = r#"#!/bin/sh
echo '{"command":"add","ok":false,"error":"boom","rc":1}'
exit 1
"#;
        let (_d, vpn) = vpn_with_script(stub);
        let err = vpn.add("alice", None, false).await.unwrap_err();
        // status:Unknown → Parse (не ScriptFailed: stdout есть, конверт распарсен).
        assert!(
            matches!(err, crate::error::Error::Parse(_)),
            "got {err:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn add_errors_when_script_prints_nothing_and_exits_nonzero() {
        // Скрипт упал ДО печати чего-либо (например, kill -9, OOM, либо
        // bash-ошибка до любого echo). run() возвращает пустой stdout →
        // защита от пустого stdout: ScriptFailed с diagnostic-контекстом.
        // (Если скрипт печатает в stderr, runner мержит его в out — тогда
        // сработает Parse, что тоже корректно: был вывод, но не JSON.)
        let stub = "#!/bin/sh\nexit 1\n";
        let (_d, vpn) = vpn_with_script(stub);
        let err = vpn.add("alice", None, false).await.unwrap_err();
        assert!(
            matches!(
                err,
                crate::error::Error::ScriptFailed { code: Some(1), .. }
            ),
            "got {err:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn add_passes_psk_and_expires_flags() {
        // argv: add <name> [--expires=..] [--psk] --json
        const STUB: &str = r#"#!/bin/sh
[ "$1" = add ] || exit 1
ok=1
for a in "$@"; do
  case "$a" in
    --psk) psk=1 ;;
    --expires=*) exp="$a" ;;
  esac
done
if [ -n "${psk:-}" ] && [ -n "${exp:-}" ]; then
  echo '{"ok":true,"results":[{"name":"a","status":"created"}]}'
elif [ -z "${psk:-}${exp:-}" ]; then
  echo '{"ok":true,"results":[{"name":"a","status":"created"}]}'
else
  exit 1
fi
"#;
        let (_d, vpn) = vpn_with_script(STUB);
        assert!(vpn.add("alice", Some("30d"), true).await.is_ok());
        assert!(vpn.add("alice", None, false).await.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn remove_returns_not_found_when_script_exits_nonzero() {
        // Реальный инсталлер: {"status":"not_found"} → exit 1.
        let stub = r#"#!/bin/sh
echo '{"command":"remove","ok":true,"removed":0,"failed":1,"results":[{"name":"ghost","status":"not_found"}]}'
exit 1
"#;
        let (_d, vpn) = vpn_with_script(stub);
        let err = vpn.remove("ghost").await.unwrap_err();
        assert!(
            matches!(err, crate::error::Error::ClientNotFound(_)),
            "got {err:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn remove_success() {
        let stub = r#"#!/bin/sh
echo '{"command":"remove","ok":true,"removed":1,"failed":0,"results":[{"name":"alice","status":"removed"}]}'
"#;
        let (_d, vpn) = vpn_with_script(stub);
        vpn.remove("alice").await.unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn regen_client_success_takes_paths_from_json() {
        let stub = r#"#!/bin/sh
[ "$1" = regen ] || exit 1
echo '{"command":"regen","ok":true,"regenerated":1,"failed":0,"results":[{"name":"alice","status":"regenerated","conf":"/x/alice.conf","qr":null,"vpnuri":null}]}'
"#;
        let (_d, vpn) = vpn_with_script(stub);
        let res = vpn.regen_client("alice").await.unwrap();
        assert_eq!(res.conf_path, "/x/alice.conf");
    }

    #[tokio::test]
    #[serial]
    async fn regen_client_not_found_becomes_client_not_found() {
        // Реальный инсталлер: {"status":"not_found"} → exit 1.
        let stub = r#"#!/bin/sh
echo '{"command":"regen","ok":true,"regenerated":0,"failed":1,"results":[{"name":"ghost","status":"not_found"}]}'
exit 1
"#;
        let (_d, vpn) = vpn_with_script(stub);
        let err = vpn.regen_client("ghost").await.unwrap_err();
        assert!(
            matches!(err, crate::error::Error::ClientNotFound(_)),
            "got {err:?}"
        );
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
        assert!(matches!(
            vpn.existing_files("ghost"),
            Err(crate::error::Error::Parse(_))
        ));
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
    #[serial]
    async fn backup_takes_path_from_json() {
        // Stub создаёт реальный файл по пути из JSON (BackupFile делает stat).
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("awg_backup_x.tar.gz");
        std::fs::write(&target, b"archive").unwrap();
        let target_str = target.to_string_lossy().to_string();
        let stub = format!(
            r#"#!/bin/sh
[ "$1" = backup ] || exit 1
echo '{{"command":"backup","ok":true,"path":"{}","size_bytes":7}}'
"#,
            target_str
        );
        let (dir2, vpn) = vpn_with_script(&stub);
        // BackupFile ищет файл по path из JSON; clients_dir тут dir2.path(),
        // но backup() использует путь из ответа, а не clients_dir.
        let _ = dir2;
        let bf = vpn.backup().await.unwrap();
        assert_eq!(bf.name, "awg_backup_x.tar.gz");
        assert_eq!(bf.size, 7);
        assert!(bf.path.ends_with("awg_backup_x.tar.gz"));
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
    #[serial] // гонка ETXTBSY: параллельный fork удерживает write-fd чужого fake-скрипта до execve
    async fn restore_rejects_out_of_range() {
        let (_d, vpn) = vpn_with_script("#!/bin/sh\n");
        assert!(matches!(
            vpn.restore(999).await,
            Err(crate::error::Error::Parse(_))
        ));
    }

    #[tokio::test]
    #[serial]
    async fn check_returns_report_with_problems() {
        // check с ok:false (обнаружены проблемы) НЕ ошибка выполнения — возвращаем отчёт.
        let stub = r#"#!/bin/sh
echo '{"command":"check","ok":false,"service":{"unit":"awg-quick@awg0","active":false},"interface":{"name":"awg0","present":false},"port":{"number":0,"listening":false},"module":{"loaded":false},"clients":{"total":0},"firewall":{"ufw_active":false,"port_allowed":false}}'
"#;
        let (_d, vpn) = vpn_with_script(stub);
        let report = vpn.check().await.unwrap();
        assert!(!report.ok);
        assert!(!report.service.active);
    }

    #[tokio::test]
    #[serial]
    async fn check_success_report() {
        let stub = r#"#!/bin/sh
echo '{"command":"check","ok":true,"service":{"unit":"awg-quick@awg0","active":true},"interface":{"name":"awg0","present":true,"mtu":1280,"addresses":["10.9.9.1/24"]},"port":{"number":39743,"listening":true},"module":{"loaded":true},"clients":{"total":5},"firewall":{"ufw_active":true,"port_allowed":true}}'
"#;
        let (_d, vpn) = vpn_with_script(stub);
        let report = vpn.check().await.unwrap();
        assert!(report.ok);
        assert_eq!(report.clients.total, 5);
    }

    #[tokio::test]
    #[serial]
    async fn check_returns_error_on_fatal_envelope() {
        // P2.2: при фатальной ошибке (die до check_server) инсталлер возвращает
        // аварийный конверт {"ok":false,"error":...,"rc":N} без полей отчёта.
        // Без try_error_envelope он десериализуется в фиктивный отчёт (все defaults)
        // → бот показывает «сервис неактивен, 0 клиентов» как нормальный результат.
        let stub = r#"#!/bin/sh
echo '{"command":"check","ok":false,"error":"config not found","rc":1}'
exit 1
"#;
        let (_d, vpn) = vpn_with_script(stub);
        let err = vpn.check().await.unwrap_err();
        assert!(
            matches!(err, crate::error::Error::ScriptFailed { code: Some(1), .. }),
            "expected ScriptFailed for fatal envelope, got {err:?}"
        );
    }

    #[tokio::test]
    #[serial] // гонка ETXTBSY: параллельный fork удерживает write-fd чужого fake-скрипта до execve
    async fn diagnose_returns_output_even_on_problems() {
        let (_d, vpn) = vpn_with_script("#!/bin/sh\necho 'DIAG REPORT'\nexit 1\n");
        let out = vpn.diagnose().await.unwrap();
        assert!(out.contains("DIAG REPORT"));
    }

    #[tokio::test]
    #[serial] // гонка ETXTBSY: параллельный fork удерживает write-fd чужого fake-скрипта до execve
    async fn diagnose_errors_on_empty_output() {
        let (_d, vpn) = vpn_with_script("#!/bin/sh\nexit 0\n");
        assert!(matches!(
            vpn.diagnose().await,
            Err(crate::error::Error::Parse(_))
        ));
    }

    #[tokio::test]
    #[serial] // гонка ETXBSY: параллельный fork удерживает write-fd чужого fake-скрипта до execve
    async fn regen_client_rejects_bad_name() {
        let (_d, vpn) = vpn_with_script("#!/bin/sh\nexit 0\n");
        assert!(vpn.regen_client("bad name;rm").await.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn regen_all_no_clients_is_noop() {
        let stub = r#"#!/bin/sh
[ "$1" = regen ] || exit 1
echo '{"command":"regen","ok":true,"regenerated":0,"failed":0,"reset_routes":false,"results":[]}'
"#;
        let (_d, vpn) = vpn_with_script(stub);
        // Теперь метод должен сигнализировать «нет клиентов» — см. изменение сигнатуры.
        let res = vpn.regen_all(false).await.unwrap();
        assert!(matches!(res, RegenAllOutcome::NoClients));
    }

    #[tokio::test]
    #[serial]
    async fn regen_all_success() {
        let stub = r#"#!/bin/sh
echo '{"command":"regen","ok":true,"regenerated":3,"failed":0,"reset_routes":false,"results":[{"name":"a","status":"regenerated"},{"name":"b","status":"regenerated"}]}'
"#;
        let (_d, vpn) = vpn_with_script(stub);
        let res = vpn.regen_all(false).await.unwrap();
        assert!(matches!(res, RegenAllOutcome::Done(n) if n == 3));
    }

    #[tokio::test]
    #[serial]
    async fn regen_all_partial_failure() {
        // Реальный инсталлер: partial (failed>0) → exit 1.
        let stub = r#"#!/bin/sh
echo '{"command":"regen","ok":false,"regenerated":1,"failed":1,"reset_routes":false,"results":[{"name":"a","status":"regenerated"},{"name":"b","status":"error"}]}'
exit 1
"#;
        let (_d, vpn) = vpn_with_script(stub);
        let res = vpn.regen_all(false).await.unwrap();
        assert!(matches!(res, RegenAllOutcome::Partial { ok: 1, failed: 1 }));
    }

    #[tokio::test]
    #[serial]
    async fn regen_all_success_via_json() {
        let (_d, vpn) = vpn_with_script(
            r#"#!/bin/sh
echo '{"command":"regen","ok":true,"regenerated":2,"failed":0,"reset_routes":false,"results":[]}'
"#,
        );
        assert!(matches!(
            vpn.regen_all(false).await.unwrap(),
            RegenAllOutcome::Done(2)
        ));
    }

    #[tokio::test]
    #[serial]
    async fn regen_all_passes_reset_routes_flag() {
        const STUB2: &str = r#"#!/bin/sh
for a in "$@"; do
  if [ "$a" = "--reset-routes" ]; then
    echo '{"command":"regen","ok":true,"regenerated":1,"failed":0,"reset_routes":true,"results":[]}'
    exit 0
  fi
done
exit 1
"#;
        let (_d2, vpn2) = vpn_with_script(STUB2);
        assert!(matches!(
            vpn2.regen_all(true).await.unwrap(),
            RegenAllOutcome::Done(1)
        ));
    }

    #[tokio::test]
    #[serial]
    async fn restore_success() {
        let stub = r#"#!/bin/sh
echo '{"command":"restore","ok":true,"source":"/x.tar.gz","applied":true,"rolled_back":false,"restored":{"server_conf":true,"clients":3,"keys":true}}'
"#;
        let (dir, vpn) = vpn_with_script(stub);
        // restore требует list_backups для индекса — подготовим один.
        let bdir = dir.path().join("backups");
        std::fs::create_dir_all(&bdir).unwrap();
        std::fs::write(bdir.join("awg_backup_x.tar.gz"), b"x").unwrap();
        vpn.restore(0).await.unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn restore_rolled_back_becomes_error() {
        // list_backups возвращает 1 запись; stub эмитит rolled_back=true → exit 1.
        let (dir, vpn) = vpn_with_script(
            r#"#!/bin/sh
echo '{"command":"restore","ok":false,"error":"boom","source":"/x.tar.gz","applied":false,"rolled_back":true}'
exit 1
"#,
        );
        let bdir = dir.path().join("backups");
        std::fs::create_dir_all(&bdir).unwrap();
        std::fs::write(bdir.join("awg_backup_x.tar.gz"), b"x").unwrap();
        let err = vpn.restore(0).await.unwrap_err();
        assert!(
            matches!(err, crate::error::Error::RestoreRolledBack),
            "got {err:?}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn modify_passes_param_and_value() {
        // Stub проверяет argv: modify <name> <param> <value> --json
        const STUB: &str = r#"#!/bin/sh
[ "$1" = modify ] || exit 1
[ "$2" = alice ] && [ "$3" = PersistentKeepalive ] && [ "$4" = 25 ] || exit 1
echo '{"command":"modify","ok":true,"name":"alice","param":"PersistentKeepalive","value":"25"}'
"#;
        let (_d, vpn) = vpn_with_script(STUB);
        let out = vpn
            .modify("alice", validate::ModifyParam::Keepalive, "25")
            .await
            .unwrap();
        assert_eq!(out.param, "PersistentKeepalive");
        assert_eq!(out.value, "25");
    }

    #[tokio::test]
    #[serial]
    async fn modify_rejects_bad_name() {
        let (_d, vpn) = vpn_with_script("#!/bin/sh\nexit 1\n");
        assert!(vpn
            .modify("bad name;rm", validate::ModifyParam::Dns, "1.1.1.1")
            .await
            .is_err());
    }

    #[tokio::test]
    #[serial]
    async fn restart_returns_active_true() {
        let stub = r#"#!/bin/sh
[ "$1" = restart ] || exit 1
echo '{"command":"restart","ok":true,"unit":"awg-quick@awg0","active":true}'
"#;
        let (_d, vpn) = vpn_with_script(stub);
        let out = vpn.restart().await.unwrap();
        assert!(out.active);
    }

    #[tokio::test]
    #[serial]
    async fn repair_module_returns_rc_code() {
        // Реальный инсталлер: rc=2 (требуется перезагрузка) → exit 1,
        // но JSON печатается в stdout и парсится независимо от exit code.
        let stub = r#"#!/bin/sh
[ "$1" = repair-module ] || exit 1
echo '{"command":"repair-module","ok":true,"module_loaded":true,"service_active":true,"rc":2}'
exit 1
"#;
        let (_d, vpn) = vpn_with_script(stub);
        let out = vpn.repair_module().await.unwrap();
        assert_eq!(out.rc, 2);
    }
}
