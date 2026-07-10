use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Config {
    pub bot_token: String,
    pub admin_ids: Vec<i64>,
    pub manage_script: PathBuf,
    pub clients_dir: PathBuf,
    pub sudo_prefix: String,
    pub op_timeout_secs: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("не удалось прочитать конфиг: {0}")]
    Read(#[from] std::io::Error),
    #[error("ошибка разбора TOML: {0}")]
    Parse(String),
    #[error("bot_token не задан (ни в файле, ни в AWG_BOT_TOKEN)")]
    MissingToken,
    #[error("admin_ids пуст — некому управлять ботом")]
    NoAdmins,
    #[error("manage_script не найден: {0}")]
    ScriptNotFound(PathBuf),
}

#[derive(serde::Deserialize)]
struct Raw {
    bot_token: Option<String>,
    admin_ids: Vec<i64>,
    manage_script: PathBuf,
    clients_dir: PathBuf,
    #[serde(default)]
    sudo_prefix: String,
    #[serde(default = "default_timeout")]
    op_timeout_secs: u64,
}

fn default_timeout() -> u64 {
    60
}

impl Config {
    pub fn load(path: &Path) -> Result<Config, ConfigError> {
        let text = std::fs::read_to_string(path)?;
        let raw: Raw = toml::from_str(&text).map_err(|e| ConfigError::Parse(e.to_string()))?;

        let bot_token = std::env::var("AWG_BOT_TOKEN")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| raw.bot_token.filter(|s| !s.is_empty()))
            .ok_or(ConfigError::MissingToken)?;

        if raw.admin_ids.is_empty() {
            return Err(ConfigError::NoAdmins);
        }
        if !raw.manage_script.exists() {
            return Err(ConfigError::ScriptNotFound(raw.manage_script));
        }

        Ok(Config {
            bot_token,
            admin_ids: raw.admin_ids,
            manage_script: raw.manage_script,
            clients_dir: raw.clients_dir,
            sudo_prefix: raw.sudo_prefix,
            op_timeout_secs: raw.op_timeout_secs,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write(dir: &tempfile::TempDir, name: &str, body: &str) -> PathBuf {
        let p = dir.path().join(name);
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(body.as_bytes()).unwrap();
        p
    }

    // All tests in this module are #[serial]: `Config::load` unconditionally reads
    // `AWG_BOT_TOKEN` (a process-global env var), and `env_overrides_token` sets/removes
    // it. Without serialization, tests racing in parallel threads can observe each
    // other's env var state (observed flake in `cargo test`, not just theoretical).
    #[test]
    #[serial_test::serial]
    fn loads_valid_config() {
        let dir = tempfile::tempdir().unwrap();
        let script = write(&dir, "manage.sh", "#!/bin/sh\n");
        let cfg_path = write(
            &dir,
            "config.toml",
            &format!(
                "bot_token = \"t\"\nadmin_ids = [1, 2]\nmanage_script = \"{}\"\nclients_dir = \"{}\"\nsudo_prefix = \"sudo\"\nop_timeout_secs = 60\n",
                script.display(),
                dir.path().display()
            ),
        );
        let cfg = Config::load(&cfg_path).unwrap();
        assert_eq!(cfg.bot_token, "t");
        assert_eq!(cfg.admin_ids, vec![1, 2]);
        assert_eq!(cfg.sudo_prefix, "sudo");
        assert_eq!(cfg.op_timeout_secs, 60);
    }

    #[test]
    #[serial_test::serial]
    fn rejects_empty_admins() {
        let dir = tempfile::tempdir().unwrap();
        let script = write(&dir, "manage.sh", "#!/bin/sh\n");
        let cfg_path = write(
            &dir,
            "config.toml",
            &format!(
                "bot_token = \"t\"\nadmin_ids = []\nmanage_script = \"{}\"\nclients_dir = \"{}\"\n",
                script.display(),
                dir.path().display()
            ),
        );
        assert!(matches!(Config::load(&cfg_path), Err(ConfigError::NoAdmins)));
    }

    #[test]
    #[serial_test::serial]
    fn rejects_missing_script() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_path = write(
            &dir,
            "config.toml",
            "bot_token = \"t\"\nadmin_ids = [1]\nmanage_script = \"/no/such/script.sh\"\nclients_dir = \"/tmp\"\n",
        );
        assert!(matches!(
            Config::load(&cfg_path),
            Err(ConfigError::ScriptNotFound(_))
        ));
    }

    #[test]
    #[serial_test::serial]
    fn env_overrides_token() {
        let dir = tempfile::tempdir().unwrap();
        let script = write(&dir, "manage.sh", "#!/bin/sh\n");
        let cfg_path = write(
            &dir,
            "config.toml",
            &format!(
                "bot_token = \"file-token\"\nadmin_ids = [1]\nmanage_script = \"{}\"\nclients_dir = \"{}\"\n",
                script.display(),
                dir.path().display()
            ),
        );
        std::env::set_var("AWG_BOT_TOKEN", "env-token");
        let cfg = Config::load(&cfg_path).unwrap();
        std::env::remove_var("AWG_BOT_TOKEN");
        assert_eq!(cfg.bot_token, "env-token");
    }
}
