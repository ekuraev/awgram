pub mod model;
pub mod runner;
pub mod validate;

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
        let mut args: Vec<String> = vec!["add".into(), name];
        if let Some(exp) = expires {
            let exp = validate::validate_expiry(exp).map_err(|e| crate::error::Error::Parse(e.to_string()))?;
            args.push(format!("--expires={exp}"));
        }
        args.push("--json".into());
        let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let out = run(&self.spec(), &arg_refs).await?;
        model::parse_add_result(&out).map_err(|e| crate::error::Error::Parse(e.to_string()))
    }

    pub async fn remove(&self, name: &str) -> Result<()> {
        let name = validate::validate_name(name).map_err(|e| crate::error::Error::Parse(e.to_string()))?;
        run(&self.spec(), &["remove", &name, "--json"]).await?;
        Ok(())
    }

    /// Повторная выдача уже созданных файлов клиента из `clients_dir` (для кнопки «📄 Конфиг»).
    pub fn existing_files(&self, name: &str) -> Result<AddResult> {
        let name = validate::validate_name(name).map_err(|e| crate::error::Error::Parse(e.to_string()))?;
        let conf = self.clients_dir.join(format!("{name}.conf"));
        let qr = self.clients_dir.join(format!("{name}.png"));
        let uri_path = self.clients_dir.join(format!("{name}.vpnuri"));
        if !conf.exists() || !qr.exists() {
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
            "#!/bin/sh\necho '[{\"name\":\"alice\",\"active\":true}]'\n",
        );
        let clients = vpn.list().await.unwrap();
        assert_eq!(clients.len(), 1);
        assert_eq!(clients[0].name, "alice");
    }

    #[tokio::test]
    async fn add_rejects_bad_name_before_running() {
        let (_d, vpn) = vpn_with_script("#!/bin/sh\necho should-not-run 1>&2\nexit 1\n");
        let err = vpn.add("bad name;rm", None).await.unwrap_err();
        // Ошибка валидации, а не запуска скрипта.
        assert!(matches!(err, crate::error::Error::Parse(_)));
    }

    #[test]
    fn existing_files_returns_paths_when_conf_present() {
        let (dir, vpn) = vpn_with_script("#!/bin/sh\n");
        std::fs::write(dir.path().join("alice.conf"), "conf").unwrap();
        std::fs::write(dir.path().join("alice.png"), "png").unwrap();
        std::fs::write(dir.path().join("alice.vpnuri"), "vpn://x\n").unwrap();
        let res = vpn.existing_files("alice").unwrap();
        assert!(res.conf_path.ends_with("alice.conf"));
        assert!(res.qr_path.ends_with("alice.png"));
        assert_eq!(res.uri, "vpn://x");
    }

    #[test]
    fn existing_files_errors_when_qr_missing() {
        let (dir, vpn) = vpn_with_script("#!/bin/sh\n");
        std::fs::write(dir.path().join("alice.conf"), "conf").unwrap();
        assert!(matches!(vpn.existing_files("alice"), Err(crate::error::Error::Parse(_))));
    }

    #[test]
    fn existing_files_errors_when_missing() {
        let (_d, vpn) = vpn_with_script("#!/bin/sh\n");
        assert!(matches!(vpn.existing_files("ghost"), Err(crate::error::Error::Parse(_))));
    }
}
