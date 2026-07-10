# AmneziaWG Telegram Bot — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rust-бинарник `awg-bot`, который позволяет админам через Telegram (inline-меню) добавлять/удалять клиентов AmneziaWG и смотреть список и статистику, вызывая штатный `manage_amneziawg.sh --json`.

**Architecture:** Один бинарник на `tokio` + `teloxide` (long polling) на том же VPS, что и VPN. Слой `vpn/` инкапсулирует вызов bash-скрипта и парсинг JSON и ничего не знает о Telegram; слой `bot/` строит меню и рендерит ответы и ничего не знает о деталях скрипта. Граница между ними — типизированные структуры из `vpn/model.rs`.

**Tech Stack:** Rust (edition 2021), `teloxide = 0.17` (feature `macros`), `tokio = 1.39` (`rt-multi-thread`, `macros`, `process`, `time`), `serde`/`serde_json`, `toml`, `thiserror`, `tracing` + `tracing-subscriber`, `regex`.

## Global Constraints

- Rust edition **2021**; MSRV не ниже требуемого `teloxide 0.17`.
- Telegram-библиотека — **только `teloxide` 0.17** (feature `macros`); режим — **long polling** (`Dispatcher` + `enable_ctrlc_handler`). Никаких webhook.
- Все вызовы скрипта — **только через `tokio::process::Command`** с аргументами по одному в `.arg()`. **Запрещено** строить команду через строку/шелл (`sh -c`, форматирование строки команды).
- Аргументы, приходящие от пользователя (имя клиента, срок), **обязаны** пройти валидацию из `vpn/validate.rs` **до** передачи в `Command`.
- Секреты (`bot_token`, содержимое `.conf`/QR) **никогда** не логируются.
- Каждый обработчик Telegram-апдейта возвращает `Result` и **не имеет права уронить процесс**; ошибка операции → дружелюбное сообщение пользователю + запись в лог.
- Дефолтные пути: `manage_script = "/root/awg/manage_amneziawg.sh"`, `clients_dir = "/root/awg"`.
- Коммиты — частые, по одному на задачу (или чаще), в стиле Conventional Commits.

---

## File Structure

```
Cargo.toml
.gitignore                         (уже есть)
src/
  main.rs         — точка входа: логи, конфиг, teloxide dispatcher, DI
  config.rs       — Config: чтение TOML + env-override, валидация (fail-fast)
  error.rs        — enum Error (thiserror), общий Result-алиас
  auth.rs         — is_admin(user_id, &[i64]) -> bool
  vpn/
    mod.rs        — Vpn: фасад add/remove/list/stats; re-export model
    runner.rs     — run(script, sudo_prefix, args, timeout) -> RunOutput
    model.rs      — Client, TrafficStats, AddResult, парсинг --json
    validate.rs   — validate_name, validate_expiry
  bot/
    mod.rs        — сборка dptree-схемы (schema()) + тип State
    menu.rs       — inline-клавиатуры (main_menu, expiry_menu, client_card, confirm_delete, clients_list)
    render.rs     — отправка .conf/QR/URI, форматирование list/stats
    handlers.rs   — эндпоинты команд/callback/диалогов
deploy/
  awg-bot.service — systemd unit (шаблон)
  config.example.toml
README.md
tests/
  runner_integration.rs — тест runner на скрипте-заглушке
```

Файлы группируются по ответственности (`vpn/` vs `bot/`), а не по техническому слою. Каждый файл — одна зона ответственности и помещается в контекст целиком.

---

## Task 1: Скелет проекта и модуль конфигурации

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs` (временный заглушечный `fn main`)
- Create: `src/config.rs`
- Test: юнит-тесты внутри `src/config.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: ничего.
- Produces:
  - `pub struct Config { pub bot_token: String, pub admin_ids: Vec<i64>, pub manage_script: PathBuf, pub clients_dir: PathBuf, pub sudo_prefix: String, pub op_timeout_secs: u64 }`
  - `impl Config { pub fn load(path: &Path) -> Result<Config, ConfigError>; }`
  - `pub enum ConfigError` (варианты: `Read(io::Error)`, `Parse(String)`, `MissingToken`, `NoAdmins`, `ScriptNotFound(PathBuf)`).
  - Env-override: если задан `AWG_BOT_TOKEN`, он перекрывает `bot_token` из файла.

- [ ] **Step 1: Создать `Cargo.toml`**

```toml
[package]
name = "awg-bot"
version = "0.1.0"
edition = "2021"

[dependencies]
teloxide = { version = "0.17", features = ["macros"] }
tokio = { version = "1.39", features = ["rt-multi-thread", "macros", "process", "time"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
thiserror = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
regex = "1"

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: Временный `src/main.rs`, чтобы проект собирался**

```rust
mod config;

fn main() {
    println!("awg-bot skeleton");
}
```

- [ ] **Step 3: Написать падающий тест конфига в `src/config.rs`**

Начать файл с типов и заглушки `load`, затем тесты. Сначала — только тесты и минимальные сигнатуры, чтобы тест компилировался, но падал.

```rust
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

impl Config {
    pub fn load(_path: &Path) -> Result<Config, ConfigError> {
        unimplemented!()
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

    #[test]
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
```

- [ ] **Step 4: Запустить тесты — убедиться, что падают**

Run: `cargo test config`
Expected: FAIL (паника `not implemented` в `load`).

- [ ] **Step 5: Реализовать `Config::load`**

Заменить тело `impl Config`:

```rust
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
```

- [ ] **Step 6: Запустить тесты — убедиться, что проходят**

Run: `cargo test config`
Expected: PASS (4 теста).

Примечание: тест `env_overrides_token` трогает переменную окружения процесса. Если тесты гоняются параллельно и появится флейк, добавить `#[serial_test::serial]` (dev-dep `serial-test`) на два теста, читающих env. Пока оставляем как есть.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock src/main.rs src/config.rs
git commit -m "feat: скелет проекта и загрузка конфига с валидацией"
```

---

## Task 2: Единый тип ошибки

**Files:**
- Create: `src/error.rs`
- Modify: `src/main.rs` (добавить `mod error;`)
- Test: юнит-тест в `src/error.rs`

**Interfaces:**
- Consumes: `ConfigError` из Task 1.
- Produces:
  - `pub enum Error` с вариантами: `Config(ConfigError)`, `ScriptFailed { code: Option<i32>, stderr: String }`, `Timeout`, `Parse(String)`, `Io(std::io::Error)`, `Telegram(String)`.
  - `pub type Result<T> = std::result::Result<T, Error>;`
  - `impl Error { pub fn user_message(&self) -> &'static str; }` — текст для отправки пользователю (без технических деталей/секретов).

- [ ] **Step 1: Падающий тест в `src/error.rs`**

```rust
use crate::config::ConfigError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("ошибка конфигурации: {0}")]
    Config(#[from] ConfigError),
    #[error("скрипт завершился с ошибкой (код {code:?})")]
    ScriptFailed { code: Option<i32>, stderr: String },
    #[error("превышено время ожидания операции")]
    Timeout,
    #[error("не удалось разобрать ответ скрипта: {0}")]
    Parse(String),
    #[error("ошибка ввода-вывода: {0}")]
    Io(#[from] std::io::Error),
    #[error("ошибка Telegram: {0}")]
    Telegram(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn user_message(&self) -> &'static str {
        match self {
            Error::Config(_) => "Внутренняя ошибка конфигурации.",
            Error::ScriptFailed { .. } => "❌ Операция не удалась. Попробуйте ещё раз.",
            Error::Timeout => "⏳ Превышено время ожидания. Попробуйте позже.",
            Error::Parse(_) => "Не удалось разобрать ответ сервера.",
            Error::Io(_) => "❌ Ошибка выполнения операции.",
            Error::Telegram(_) => "❌ Ошибка отправки сообщения.",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_message_hides_stderr() {
        let e = Error::ScriptFailed { code: Some(1), stderr: "secret-key-leak".into() };
        assert!(!e.user_message().contains("secret"));
        assert_eq!(e.user_message(), "❌ Операция не удалась. Попробуйте ещё раз.");
    }
}
```

- [ ] **Step 2: Добавить `mod error;` в `src/main.rs`**

```rust
mod config;
mod error;

fn main() {
    println!("awg-bot skeleton");
}
```

- [ ] **Step 3: Запустить тест**

Run: `cargo test error`
Expected: PASS (1 тест). Также убедиться, что `cargo build` компилируется.

- [ ] **Step 4: Commit**

```bash
git add src/error.rs src/main.rs
git commit -m "feat: единый тип ошибки с безопасным user_message"
```

---

## Task 3: Валидация имени и срока

**Files:**
- Create: `src/vpn/validate.rs`
- Create: `src/vpn/mod.rs` (пока только `pub mod validate;`)
- Modify: `src/main.rs` (добавить `mod vpn;`)
- Test: юнит-тесты в `src/vpn/validate.rs`

**Interfaces:**
- Consumes: ничего.
- Produces:
  - `pub fn validate_name(input: &str) -> Result<String, ValidateError>` — trim, проверка по regex `^[A-Za-z0-9_-]{1,32}$`, возвращает нормализованное имя.
  - `pub fn validate_expiry(input: &str) -> Result<String, ValidateError>` — формат `^[0-9]{1,4}[hdw]$` (например `12h`, `10d`, `3w`), возвращает строку как есть (для `--expires=<v>`).
  - `pub enum ValidateError { BadName, BadExpiry }` (реализует `std::fmt::Display`).

- [ ] **Step 1: Падающий тест в `src/vpn/validate.rs`**

```rust
use std::sync::OnceLock;
use regex::Regex;

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum ValidateError {
    #[error("имя должно содержать 1–32 символа: латиница, цифры, дефис, подчёркивание")]
    BadName,
    #[error("срок должен быть в формате Nh/Nd/Nw, например 12h, 10d, 3w")]
    BadExpiry,
}

pub fn validate_name(_input: &str) -> Result<String, ValidateError> {
    unimplemented!()
}

pub fn validate_expiry(_input: &str) -> Result<String, ValidateError> {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_good_names() {
        assert_eq!(validate_name("alice").unwrap(), "alice");
        assert_eq!(validate_name("  bob_1-2  ").unwrap(), "bob_1-2");
    }

    #[test]
    fn rejects_injection_and_bad_names() {
        for bad in ["", "a b", "a;rm -rf /", "../etc", "имя", "a".repeat(33).as_str(), "--flag", "a/b"] {
            assert_eq!(validate_name(bad), Err(ValidateError::BadName), "should reject {bad:?}");
        }
    }

    #[test]
    fn accepts_good_expiry() {
        for good in ["12h", "10d", "3w", "1d", "9999h"] {
            assert!(validate_expiry(good).is_ok(), "should accept {good}");
        }
    }

    #[test]
    fn rejects_bad_expiry() {
        for bad in ["", "10", "d10", "10x", "1.5d", "10 d", "-5d", "10d;ls"] {
            assert_eq!(validate_expiry(bad), Err(ValidateError::BadExpiry), "should reject {bad:?}");
        }
    }
}
```

- [ ] **Step 2: Создать `src/vpn/mod.rs` и подключить модуль в `main.rs`**

`src/vpn/mod.rs`:
```rust
pub mod validate;
```

`src/main.rs`:
```rust
mod config;
mod error;
mod vpn;

fn main() {
    println!("awg-bot skeleton");
}
```

- [ ] **Step 3: Запустить тест — убедиться, что падает**

Run: `cargo test validate`
Expected: FAIL (`not implemented`).

- [ ] **Step 4: Реализовать функции**

Заменить заглушки:
```rust
fn name_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[A-Za-z0-9_-]{1,32}$").unwrap())
}

fn expiry_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[0-9]{1,4}[hdw]$").unwrap())
}

pub fn validate_name(input: &str) -> Result<String, ValidateError> {
    let name = input.trim();
    if name_re().is_match(name) {
        Ok(name.to_string())
    } else {
        Err(ValidateError::BadName)
    }
}

pub fn validate_expiry(input: &str) -> Result<String, ValidateError> {
    let v = input.trim();
    if expiry_re().is_match(v) {
        Ok(v.to_string())
    } else {
        Err(ValidateError::BadExpiry)
    }
}
```

- [ ] **Step 5: Запустить тест — убедиться, что проходит**

Run: `cargo test validate`
Expected: PASS (4 теста).

- [ ] **Step 6: Commit**

```bash
git add src/vpn/mod.rs src/vpn/validate.rs src/main.rs
git commit -m "feat: валидация имени клиента и срока действия (защита от инъекций)"
```

---

## Task 4: Модель данных и парсинг `--json`

> ВАЖНО: реальная схема `--json` установщика заранее не зафиксирована (см. спеку §10). **Первый шаг задачи — захватить реальный вывод** и подогнать под него структуры и тестовые фикстуры. Ниже задана правдоподобная схема как отправная точка.

**Files:**
- Create: `src/vpn/model.rs`
- Modify: `src/vpn/mod.rs` (добавить `pub mod model;`)
- Test: юнит-тесты в `src/vpn/model.rs`

**Interfaces:**
- Consumes: ничего.
- Produces:
  - `pub struct Client { pub name: String, pub active: bool, pub expires_at: Option<String>, pub rx_bytes: u64, pub tx_bytes: u64, pub last_handshake: Option<String> }`
  - `pub struct AddResult { pub name: String, pub conf_path: String, pub qr_path: String, pub uri: String }`
  - `pub fn parse_client_list(json: &str) -> Result<Vec<Client>, serde_json::Error>`
  - `pub fn parse_add_result(json: &str) -> Result<AddResult, serde_json::Error>`
  - `pub fn human_bytes(n: u64) -> String` — форматирование трафика (`1.2 GB`).

- [ ] **Step 0: Зафиксировать реальную схему (ручной шаг, если есть доступ к серверу)**

Если доступен тестовый сервер с установщиком, выполнить и сохранить вывод:
```bash
/root/awg/manage_amneziawg.sh list --json
/root/awg/manage_amneziawg.sh add plan-probe --json   # затем remove plan-probe
```
Сверить поля с `serde`-структурами ниже. При расхождении — обновить `#[serde(rename = ...)]`, состав полей и JSON в тестах **синхронно**. Если доступа нет — реализовать по схеме ниже и пометить как требующее сверки на smoke-тесте (Task 11).

- [ ] **Step 1: Падающий тест в `src/vpn/model.rs`**

```rust
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Client {
    pub name: String,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub rx_bytes: u64,
    #[serde(default)]
    pub tx_bytes: u64,
    #[serde(default)]
    pub last_handshake: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AddResult {
    pub name: String,
    pub conf_path: String,
    pub qr_path: String,
    pub uri: String,
}

pub fn parse_client_list(_json: &str) -> Result<Vec<Client>, serde_json::Error> {
    unimplemented!()
}

pub fn parse_add_result(_json: &str) -> Result<AddResult, serde_json::Error> {
    unimplemented!()
}

pub fn human_bytes(_n: u64) -> String {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST_JSON: &str = r#"[
      {"name":"alice","active":true,"expires_at":"2026-08-01","rx_bytes":1288490188,"tx_bytes":356515840,"last_handshake":"2026-07-10T10:00:00Z"},
      {"name":"bob","active":false}
    ]"#;

    const ADD_JSON: &str = r#"{"name":"carol","conf_path":"/root/awg/carol.conf","qr_path":"/root/awg/carol.png","uri":"vpn://example"}"#;

    #[test]
    fn parses_client_list() {
        let clients = parse_client_list(LIST_JSON).unwrap();
        assert_eq!(clients.len(), 2);
        assert_eq!(clients[0].name, "alice");
        assert!(clients[0].active);
        assert_eq!(clients[0].rx_bytes, 1288490188);
        assert_eq!(clients[1].name, "bob");
        assert!(!clients[1].active);
        assert_eq!(clients[1].rx_bytes, 0);
    }

    #[test]
    fn parses_add_result() {
        let r = parse_add_result(ADD_JSON).unwrap();
        assert_eq!(r.name, "carol");
        assert_eq!(r.conf_path, "/root/awg/carol.conf");
        assert_eq!(r.qr_path, "/root/awg/carol.png");
    }

    #[test]
    fn human_bytes_formats() {
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(512), "512 B");
        assert_eq!(human_bytes(1536), "1.5 KB");
        assert_eq!(human_bytes(1288490188), "1.2 GB");
    }
}
```

- [ ] **Step 2: Подключить модуль в `src/vpn/mod.rs`**

```rust
pub mod model;
pub mod validate;
```

- [ ] **Step 3: Запустить тест — убедиться, что падает**

Run: `cargo test --lib model`
Expected: FAIL (`not implemented`).

- [ ] **Step 4: Реализовать функции**

```rust
pub fn parse_client_list(json: &str) -> Result<Vec<Client>, serde_json::Error> {
    serde_json::from_str(json)
}

pub fn parse_add_result(json: &str) -> Result<AddResult, serde_json::Error> {
    serde_json::from_str(json)
}

pub fn human_bytes(n: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    if n < 1024 {
        return format!("{n} B");
    }
    let mut value = n as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}
```

- [ ] **Step 5: Запустить тест — убедиться, что проходит**

Run: `cargo test --lib model`
Expected: PASS (3 теста).

- [ ] **Step 6: Commit**

```bash
git add src/vpn/mod.rs src/vpn/model.rs
git commit -m "feat: модель клиентов и парсинг --json со сверкой схемы"
```

---

## Task 5: Runner — запуск скрипта с тайм-аутом

**Files:**
- Create: `src/vpn/runner.rs`
- Modify: `src/vpn/mod.rs` (добавить `pub mod runner;`)
- Create: `tests/runner_integration.rs`

**Interfaces:**
- Consumes: `crate::error::{Error, Result}`.
- Produces:
  - `pub struct RunSpec<'a> { pub script: &'a Path, pub sudo_prefix: &'a str, pub timeout_secs: u64 }`
  - `pub async fn run(spec: &RunSpec<'_>, args: &[&str]) -> Result<String>` — возвращает stdout при коде 0; при ненулевом коде → `Error::ScriptFailed`; при тайм-ауте → `Error::Timeout` (процесс убивается).
  - Логика префикса: если `sudo_prefix` непустой — программа = `sudo_prefix`, первый аргумент = путь к скрипту, далее `args`; иначе программа = путь к скрипту, аргументы = `args`.

- [ ] **Step 1: Написать интеграционный тест в `tests/runner_integration.rs`**

Тест использует скрипты-заглушки, создаваемые во временной директории (без реального VPN).

```rust
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use awg_bot::error::Error;
use awg_bot::vpn::runner::{run, RunSpec};

fn make_script(body: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fake.sh");
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(body.as_bytes()).unwrap();
    let mut perm = std::fs::metadata(&path).unwrap().permissions();
    perm.set_mode(0o755);
    std::fs::set_permissions(&path, perm).unwrap();
    (dir, path)
}

#[tokio::test]
async fn returns_stdout_on_success() {
    let (_d, script) = make_script("#!/bin/sh\necho \"$1-ok\"\n");
    let spec = RunSpec { script: &script, sudo_prefix: "", timeout_secs: 5 };
    let out = run(&spec, &["list"]).await.unwrap();
    assert_eq!(out.trim(), "list-ok");
}

#[tokio::test]
async fn maps_nonzero_exit_to_script_failed() {
    let (_d, script) = make_script("#!/bin/sh\necho boom 1>&2\nexit 3\n");
    let spec = RunSpec { script: &script, sudo_prefix: "", timeout_secs: 5 };
    let err = run(&spec, &["add"]).await.unwrap_err();
    match err {
        Error::ScriptFailed { code, stderr } => {
            assert_eq!(code, Some(3));
            assert!(stderr.contains("boom"));
        }
        other => panic!("expected ScriptFailed, got {other:?}"),
    }
}

#[tokio::test]
async fn times_out_long_running_script() {
    let (_d, script) = make_script("#!/bin/sh\nsleep 10\n");
    let spec = RunSpec { script: &script, sudo_prefix: "", timeout_secs: 1 };
    let err = run(&spec, &["list"]).await.unwrap_err();
    assert!(matches!(err, Error::Timeout));
}
```

> Для интеграционного теста крейт должен экспортировать модули как библиотека. В Step 2 добавляется `src/lib.rs`.

- [ ] **Step 2: Добавить `src/lib.rs` и подключить runner**

Создать `src/lib.rs` (делает модули доступными и бинарнику, и тестам):
```rust
pub mod auth;
pub mod bot;
pub mod config;
pub mod error;
pub mod vpn;
```

> Модули `auth` и `bot` появятся в Task 6–10. Чтобы `lib.rs` компилировался сейчас, временно закомментировать `pub mod auth;` и `pub mod bot;` и раскомментировать по мере создания. Альтернатива: создать пустые `src/auth.rs` (`pub fn placeholder() {}`) и `src/bot/mod.rs` — но проще коммент. На этом шаге оставить только `config`, `error`, `vpn`.

`src/lib.rs` (на данный момент):
```rust
pub mod config;
pub mod error;
pub mod vpn;
```

`src/vpn/mod.rs`:
```rust
pub mod model;
pub mod runner;
pub mod validate;
```

Обновить `src/main.rs`, чтобы использовать библиотеку (убрать дублирующие `mod`):
```rust
use awg_bot::config;

fn main() {
    let _ = &config::Config::load;
    println!("awg-bot skeleton");
}
```

- [ ] **Step 3: Заглушка `run` в `src/vpn/runner.rs`, чтобы тест компилировался и падал**

```rust
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command;
use tokio::time::timeout;

use crate::error::{Error, Result};

pub struct RunSpec<'a> {
    pub script: &'a Path,
    pub sudo_prefix: &'a str,
    pub timeout_secs: u64,
}

pub async fn run(_spec: &RunSpec<'_>, _args: &[&str]) -> Result<String> {
    unimplemented!()
}
```

- [ ] **Step 4: Запустить тест — убедиться, что падает**

Run: `cargo test --test runner_integration`
Expected: FAIL (`not implemented`).

- [ ] **Step 5: Реализовать `run`**

```rust
pub async fn run(spec: &RunSpec<'_>, args: &[&str]) -> Result<String> {
    let mut cmd = if spec.sudo_prefix.is_empty() {
        let mut c = Command::new(spec.script);
        c.args(args);
        c
    } else {
        let mut c = Command::new(spec.sudo_prefix);
        c.arg(spec.script);
        c.args(args);
        c
    };
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);

    let child = cmd.spawn()?;
    let dur = Duration::from_secs(spec.timeout_secs);

    let output = match timeout(dur, child.wait_with_output()).await {
        Ok(res) => res?,
        Err(_) => return Err(Error::Timeout), // child убивается через kill_on_drop
    };

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(Error::ScriptFailed {
            code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}
```

- [ ] **Step 6: Запустить тест — убедиться, что проходит**

Run: `cargo test --test runner_integration`
Expected: PASS (3 теста).

- [ ] **Step 7: Commit**

```bash
git add src/lib.rs src/main.rs src/vpn/mod.rs src/vpn/runner.rs tests/runner_integration.rs
git commit -m "feat: runner скрипта с тайм-аутом и маппингом ошибок + библиотечный крейт"
```

---

## Task 6: Фасад Vpn и авторизация

**Files:**
- Modify: `src/vpn/mod.rs` (добавить структуру `Vpn` и методы)
- Create: `src/auth.rs`
- Modify: `src/lib.rs` (раскомментировать `pub mod auth;`)
- Test: юнит-тесты в `src/auth.rs` и в `src/vpn/mod.rs`

**Interfaces:**
- Consumes: `runner::{run, RunSpec}`, `model::*`, `validate::*`, `config::Config`, `error::Result`.
- Produces:
  - `pub struct Vpn { script: PathBuf, sudo_prefix: String, timeout_secs: u64, clients_dir: PathBuf }`
  - `impl Vpn { pub fn from_config(cfg: &Config) -> Vpn; pub async fn add(&self, name: &str, expires: Option<&str>) -> Result<AddResult>; pub async fn remove(&self, name: &str) -> Result<()>; pub async fn list(&self) -> Result<Vec<Client>>; pub async fn stats(&self) -> Result<Vec<Client>>; pub fn existing_files(&self, name: &str) -> Result<AddResult>; }`
  - `auth::is_admin(user_id: i64, admins: &[i64]) -> bool`.

> `existing_files` нужен Task 9 (кнопка «📄 Конфиг» — повторно выдать уже созданные файлы клиента из `clients_dir`, без пересоздания). Реализуется здесь же.

> `add`/`remove` вызывают `validate::validate_name` внутри; `add` — `validate::validate_expiry`, если `expires` задан. `stats` переиспользует `list` (те же поля трафика), либо отдельная команда `stats --json`, если её вывод богаче — сверить на Step 0 Task 4.

- [ ] **Step 1: Падающий тест авторизации в `src/auth.rs`**

```rust
pub fn is_admin(user_id: i64, admins: &[i64]) -> bool {
    admins.contains(&user_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admits_listed_and_rejects_others() {
        let admins = [111i64, 222];
        assert!(is_admin(111, &admins));
        assert!(is_admin(222, &admins));
        assert!(!is_admin(333, &admins));
        assert!(!is_admin(0, &admins));
    }
}
```

- [ ] **Step 2: Раскомментировать `pub mod auth;` в `src/lib.rs`**

```rust
pub mod auth;
pub mod config;
pub mod error;
pub mod vpn;
```

- [ ] **Step 3: Запустить тест авторизации**

Run: `cargo test --lib auth`
Expected: PASS (1 тест) — реализация тривиальна, тест и код добавлены вместе.

- [ ] **Step 4: Написать падающий тест фасада `Vpn` через скрипт-заглушку**

Добавить в конец `src/vpn/mod.rs`:
```rust
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
        std::fs::write(dir.path().join("alice.vpnuri"), "vpn://x\n").unwrap();
        let res = vpn.existing_files("alice").unwrap();
        assert!(res.conf_path.ends_with("alice.conf"));
        assert!(res.qr_path.ends_with("alice.png"));
        assert_eq!(res.uri, "vpn://x");
    }

    #[test]
    fn existing_files_errors_when_missing() {
        let (_d, vpn) = vpn_with_script("#!/bin/sh\n");
        assert!(matches!(vpn.existing_files("ghost"), Err(crate::error::Error::Parse(_))));
    }
}
```

- [ ] **Step 5: Запустить тесты — убедиться, что падают, затем проходят**

Реализация уже включена в Step 4 (TDD-петля здесь короткая: тест и минимальная реализация фасада в одном файле). Порядок:
Run: `cargo test --lib` — Expected: PASS (auth + model + vpn::tests).
Если что-то не компилируется — исправить сигнатуры до зелёного.

- [ ] **Step 6: Commit**

```bash
git add src/auth.rs src/lib.rs src/vpn/mod.rs
git commit -m "feat: фасад Vpn (add/remove/list/stats) и авторизация админов"
```

---

## Task 7: Inline-клавиатуры

**Files:**
- Create: `src/bot/mod.rs` (пока `pub mod menu;` + заготовка `State`)
- Create: `src/bot/menu.rs`
- Modify: `src/lib.rs` (раскомментировать `pub mod bot;`)
- Test: юнит-тесты в `src/bot/menu.rs`

**Interfaces:**
- Consumes: `teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup}`, `vpn::model::Client`.
- Produces (callback-data — стабильные строковые контракты, используются в Task 9):
  - `pub fn main_menu() -> InlineKeyboardMarkup` — кнопки с data: `list`, `add`, `stats`.
  - `pub fn expiry_menu() -> InlineKeyboardMarkup` — data: `exp:none`, `exp:1d`, `exp:7d`, `exp:14d`, `exp:30d`, `exp:90d`, `exp:180d`, `exp:365d`, `exp:custom`.
  - `pub fn clients_list(clients: &[Client], page: usize, per_page: usize) -> InlineKeyboardMarkup` — по кнопке на клиента (data `client:<name>`) + навигация `page:<n>` + `menu`.
  - `pub fn client_card(name: &str) -> InlineKeyboardMarkup` — data: `conf:<name>`, `del:<name>`, `menu`.
  - `pub fn confirm_delete(name: &str) -> InlineKeyboardMarkup` — data: `delyes:<name>`, `menu`.

- [ ] **Step 1: Создать `src/bot/mod.rs` с типом `State` и подключить модуль**

`src/bot/mod.rs`:
```rust
pub mod menu;

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Idle,
    AwaitingName,
    AwaitingExpiry { name: String },
    AwaitingCustomExpiry { name: String },
}
```

`src/lib.rs`:
```rust
pub mod auth;
pub mod bot;
pub mod config;
pub mod error;
pub mod vpn;
```

- [ ] **Step 2: Падающий тест в `src/bot/menu.rs`**

```rust
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

use crate::vpn::model::Client;

pub fn main_menu() -> InlineKeyboardMarkup {
    unimplemented!()
}
pub fn expiry_menu() -> InlineKeyboardMarkup {
    unimplemented!()
}
pub fn clients_list(_clients: &[Client], _page: usize, _per_page: usize) -> InlineKeyboardMarkup {
    unimplemented!()
}
pub fn client_card(_name: &str) -> InlineKeyboardMarkup {
    unimplemented!()
}
pub fn confirm_delete(_name: &str) -> InlineKeyboardMarkup {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_callback_data(kb: &InlineKeyboardMarkup) -> Vec<String> {
        kb.inline_keyboard
            .iter()
            .flatten()
            .filter_map(|b| match &b.kind {
                teloxide::types::InlineKeyboardButtonKind::CallbackData(d) => Some(d.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn main_menu_has_expected_actions() {
        let data = all_callback_data(&main_menu());
        for expected in ["list", "add", "stats"] {
            assert!(data.contains(&expected.to_string()), "missing {expected}");
        }
    }

    #[test]
    fn expiry_menu_has_custom_and_presets() {
        let data = all_callback_data(&expiry_menu());
        assert!(data.contains(&"exp:none".to_string()));
        assert!(data.contains(&"exp:30d".to_string()));
        assert!(data.contains(&"exp:custom".to_string()));
    }

    #[test]
    fn client_card_encodes_name() {
        let data = all_callback_data(&client_card("alice"));
        assert!(data.contains(&"conf:alice".to_string()));
        assert!(data.contains(&"del:alice".to_string()));
    }

    #[test]
    fn confirm_delete_encodes_name() {
        let data = all_callback_data(&confirm_delete("bob"));
        assert!(data.contains(&"delyes:bob".to_string()));
    }

    #[test]
    fn clients_list_one_button_per_client() {
        let clients = vec![
            Client { name: "a".into(), active: true, expires_at: None, rx_bytes: 0, tx_bytes: 0, last_handshake: None },
            Client { name: "b".into(), active: false, expires_at: None, rx_bytes: 0, tx_bytes: 0, last_handshake: None },
        ];
        let data = all_callback_data(&clients_list(&clients, 0, 10));
        assert!(data.contains(&"client:a".to_string()));
        assert!(data.contains(&"client:b".to_string()));
    }
}
```

- [ ] **Step 3: Запустить тест — убедиться, что падает**

Run: `cargo test --lib menu`
Expected: FAIL (`not implemented`).

- [ ] **Step 4: Реализовать клавиатуры**

```rust
fn cb(text: &str, data: &str) -> InlineKeyboardButton {
    InlineKeyboardButton::callback(text.to_string(), data.to_string())
}

pub fn main_menu() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![cb("👥 Клиенты", "list")],
        vec![cb("➕ Добавить", "add")],
        vec![cb("📊 Статистика", "stats")],
    ])
}

pub fn expiry_menu() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![cb("Без срока", "exp:none")],
        vec![cb("1д", "exp:1d"), cb("7д", "exp:7d"), cb("14д", "exp:14d")],
        vec![cb("30д", "exp:30d"), cb("90д", "exp:90d"), cb("180д", "exp:180d")],
        vec![cb("365д", "exp:365d"), cb("✏️ Свой", "exp:custom")],
    ])
}

pub fn clients_list(clients: &[Client], page: usize, per_page: usize) -> InlineKeyboardMarkup {
    let start = page * per_page;
    let slice = clients.iter().skip(start).take(per_page);
    let mut rows: Vec<Vec<InlineKeyboardButton>> = slice
        .map(|c| {
            let mark = if c.active { "🟢" } else { "🔴" };
            vec![cb(&format!("{mark} {}", c.name), &format!("client:{}", c.name))]
        })
        .collect();

    let total_pages = clients.len().div_ceil(per_page).max(1);
    let mut nav = Vec::new();
    if page > 0 {
        nav.push(cb("◀️", &format!("page:{}", page - 1)));
    }
    if page + 1 < total_pages {
        nav.push(cb("▶️", &format!("page:{}", page + 1)));
    }
    if !nav.is_empty() {
        rows.push(nav);
    }
    rows.push(vec![cb("⬅️ В меню", "menu")]);
    InlineKeyboardMarkup::new(rows)
}

pub fn client_card(name: &str) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![cb("📄 Конфиг", &format!("conf:{name}")), cb("🗑 Удалить", &format!("del:{name}"))],
        vec![cb("⬅️ В меню", "menu")],
    ])
}

pub fn confirm_delete(name: &str) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![
        cb("✅ Да, удалить", &format!("delyes:{name}")),
        cb("⬅️ Отмена", "menu"),
    ]])
}
```

- [ ] **Step 5: Запустить тест — убедиться, что проходит**

Run: `cargo test --lib menu`
Expected: PASS (5 тестов).

- [ ] **Step 6: Commit**

```bash
git add src/bot/mod.rs src/bot/menu.rs src/lib.rs
git commit -m "feat: inline-клавиатуры меню, списка, карточки клиента и подтверждения"
```

---

## Task 8: Рендер ответов и отправка файлов

**Files:**
- Create: `src/bot/render.rs`
- Modify: `src/bot/mod.rs` (добавить `pub mod render;`)
- Test: юнит-тесты форматирования в `src/bot/render.rs`

**Interfaces:**
- Consumes: `teloxide::{Bot, prelude::*, types::{ChatId, InputFile}}`, `vpn::model::{Client, AddResult, human_bytes}`, `error::Result`.
- Produces:
  - `pub fn format_client_card(c: &Client) -> String` — текст карточки.
  - `pub fn format_stats(clients: &[Client]) -> String` — сводка.
  - `pub async fn send_client_files(bot: &Bot, chat: ChatId, res: &AddResult) -> Result<()>` — шлёт документ `.conf`, фото QR и текст с URI.

> Тестируем только чистые функции форматирования (без сети). Отправка файлов проверяется на smoke-тесте (Task 11).

- [ ] **Step 1: Падающий тест форматирования в `src/bot/render.rs`**

```rust
use teloxide::prelude::*;
use teloxide::types::{ChatId, InputFile};

use crate::error::{Error, Result};
use crate::vpn::model::{human_bytes, AddResult, Client};

pub fn format_client_card(_c: &Client) -> String {
    unimplemented!()
}

pub fn format_stats(_clients: &[Client]) -> String {
    unimplemented!()
}

pub async fn send_client_files(_bot: &Bot, _chat: ChatId, _res: &AddResult) -> Result<()> {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Client {
        Client {
            name: "alice".into(),
            active: true,
            expires_at: Some("2026-08-01".into()),
            rx_bytes: 1288490188,
            tx_bytes: 356515840,
            last_handshake: Some("2026-07-10T10:00:00Z".into()),
        }
    }

    #[test]
    fn card_contains_name_and_traffic() {
        let text = format_client_card(&sample());
        assert!(text.contains("alice"));
        assert!(text.contains("активен"));
        assert!(text.contains("1.2 GB"));
        assert!(text.contains("2026-08-01"));
    }

    #[test]
    fn stats_counts_clients() {
        let clients = vec![sample(), Client { active: false, name: "bob".into(), expires_at: None, rx_bytes: 0, tx_bytes: 0, last_handshake: None }];
        let text = format_stats(&clients);
        assert!(text.contains("2")); // всего клиентов
        assert!(text.contains("1")); // активных
    }
}
```

- [ ] **Step 2: Подключить модуль в `src/bot/mod.rs`**

```rust
pub mod menu;
pub mod render;
```
(плюс существующий `enum State`).

- [ ] **Step 3: Запустить тест — убедиться, что падает**

Run: `cargo test --lib render`
Expected: FAIL (`not implemented`).

- [ ] **Step 4: Реализовать функции**

```rust
pub fn format_client_card(c: &Client) -> String {
    let status = if c.active { "активен" } else { "отключён" };
    let expires = c.expires_at.as_deref().unwrap_or("бессрочно");
    format!(
        "client: {name}\nстатус: {status} · истекает: {expires}\nтрафик: ↓ {rx}  ↑ {tx}",
        name = c.name,
        rx = human_bytes(c.rx_bytes),
        tx = human_bytes(c.tx_bytes),
    )
}

pub fn format_stats(clients: &[Client]) -> String {
    let total = clients.len();
    let active = clients.iter().filter(|c| c.active).count();
    let rx: u64 = clients.iter().map(|c| c.rx_bytes).sum();
    let tx: u64 = clients.iter().map(|c| c.tx_bytes).sum();
    format!(
        "📊 Статистика\nВсего клиентов: {total}\nАктивных: {active}\nТрафик суммарно: ↓ {rx}  ↑ {tx}",
        rx = human_bytes(rx),
        tx = human_bytes(tx),
    )
}

pub async fn send_client_files(bot: &Bot, chat: ChatId, res: &AddResult) -> Result<()> {
    bot.send_document(chat, InputFile::file(&res.conf_path))
        .await
        .map_err(|e| Error::Telegram(e.to_string()))?;
    bot.send_photo(chat, InputFile::file(&res.qr_path))
        .await
        .map_err(|e| Error::Telegram(e.to_string()))?;
    bot.send_message(chat, format!("🔗 Ссылка для импорта:\n`{}`", res.uri))
        .parse_mode(teloxide::types::ParseMode::MarkdownV2)
        .await
        .map_err(|e| Error::Telegram(e.to_string()))?;
    Ok(())
}
```

> Примечание по `MarkdownV2`: URI может содержать спецсимволы. Если при smoke-тесте (Task 11) отправка падает на экранировании — убрать `.parse_mode(...)` и слать URI обычным текстом. Зафиксировать решение там.

- [ ] **Step 5: Запустить тест — убедиться, что проходит**

Run: `cargo test --lib render`
Expected: PASS (2 теста).

- [ ] **Step 6: Commit**

```bash
git add src/bot/mod.rs src/bot/render.rs
git commit -m "feat: рендер карточки/статистики и отправка .conf/QR/URI"
```

---

## Task 9: Обработчики, диалоги и схема dptree

**Files:**
- Create: `src/bot/handlers.rs`
- Modify: `src/bot/mod.rs` (добавить `pub mod handlers;` и `pub fn schema()`)
- Test: компиляционная проверка + ручной smoke (Task 11). Юнит-тестами покрыт разбор callback-data.

**Interfaces:**
- Consumes: `Vpn` (Task 6), `auth::is_admin` (Task 6), `menu::*` (Task 7), `render::*` (Task 8), `State` (Task 7), `config::Config`.
- Produces:
  - `pub fn schema() -> ...` — dptree-хендлер для `Dispatcher`.
  - Внутренняя функция `parse_callback(data: &str) -> Action` + `enum Action` — единая точка разбора callback-data (тестируется).
  - Зависимости через `dptree::deps![...]`: `Arc<Vpn>`, `Arc<Config>`, `InMemStorage<State>`.

- [ ] **Step 1: Падающий тест разбора callback-data в `src/bot/handlers.rs`**

Начать файл с типа `Action`, `parse_callback` (заглушка) и тестов:
```rust
use std::sync::Arc;

use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::dispatching::{HandlerExt, UpdateFilterExt};
use teloxide::prelude::*;
use teloxide::types::{CallbackQuery, MaybeInaccessibleMessage};

use crate::auth::is_admin;
use crate::bot::menu;
use crate::bot::render::{self, format_client_card, format_stats};
use crate::bot::State;
use crate::config::Config;
use crate::vpn::Vpn;

#[derive(Debug, PartialEq)]
pub enum Action {
    Menu,
    List,
    Add,
    Stats,
    Page(usize),
    ShowClient(String),
    SendConf(String),
    AskDelete(String),
    ConfirmDelete(String),
    Expiry(String), // "none" | "1d" | ... | "custom"
    Unknown,
}

fn parse_callback(_data: &str) -> Action {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_actions() {
        assert_eq!(parse_callback("menu"), Action::Menu);
        assert_eq!(parse_callback("list"), Action::List);
        assert_eq!(parse_callback("add"), Action::Add);
        assert_eq!(parse_callback("stats"), Action::Stats);
        assert_eq!(parse_callback("page:3"), Action::Page(3));
        assert_eq!(parse_callback("client:alice"), Action::ShowClient("alice".into()));
        assert_eq!(parse_callback("conf:alice"), Action::SendConf("alice".into()));
        assert_eq!(parse_callback("del:alice"), Action::AskDelete("alice".into()));
        assert_eq!(parse_callback("delyes:alice"), Action::ConfirmDelete("alice".into()));
        assert_eq!(parse_callback("exp:30d"), Action::Expiry("30d".into()));
        assert_eq!(parse_callback("exp:custom"), Action::Expiry("custom".into()));
        assert_eq!(parse_callback("garbage"), Action::Unknown);
    }
}
```

- [ ] **Step 2: Запустить тест — убедиться, что падает**

Run: `cargo test --lib handlers`
Expected: FAIL (`not implemented`).

- [ ] **Step 3: Реализовать `parse_callback`**

```rust
fn parse_callback(data: &str) -> Action {
    match data {
        "menu" => Action::Menu,
        "list" => Action::List,
        "add" => Action::Add,
        "stats" => Action::Stats,
        _ => {
            if let Some(v) = data.strip_prefix("page:") {
                v.parse().map(Action::Page).unwrap_or(Action::Unknown)
            } else if let Some(v) = data.strip_prefix("client:") {
                Action::ShowClient(v.to_string())
            } else if let Some(v) = data.strip_prefix("conf:") {
                Action::SendConf(v.to_string())
            } else if let Some(v) = data.strip_prefix("delyes:") {
                Action::ConfirmDelete(v.to_string())
            } else if let Some(v) = data.strip_prefix("del:") {
                Action::AskDelete(v.to_string())
            } else if let Some(v) = data.strip_prefix("exp:") {
                Action::Expiry(v.to_string())
            } else {
                Action::Unknown
            }
        }
    }
}
```

> Порядок важен: `delyes:` проверяется раньше `del:` (иначе `del:` перехватит префикс).

- [ ] **Step 4: Запустить тест — убедиться, что проходит**

Run: `cargo test --lib handlers`
Expected: PASS (1 тест).

- [ ] **Step 5: Реализовать хендлеры и схему**

Добавить эндпоинты и `schema()`. Логика:
- `/start` и текстовые апдейты → `message_handler`; callback → `callback_handler`.
- Первым делом проверка `is_admin` по `user_id`; если нет — ответ «Доступ запрещён» и `return Ok(())`.
- Диалоговые состояния: `AwaitingName` (ждём имя → показываем `expiry_menu`, переходим в `AwaitingExpiry{name}`), `AwaitingCustomExpiry{name}` (ждём строку срока).

```rust
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
type MyDialogue = Dialogue<State, InMemStorage<State>>;

fn user_id_of_msg(msg: &Message) -> Option<i64> {
    msg.from.as_ref().map(|u| u.id.0 as i64)
}

fn user_id_of_cb(q: &CallbackQuery) -> i64 {
    q.from.id.0 as i64
}

async fn message_handler(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    cfg: Arc<Config>,
    vpn: Arc<Vpn>,
) -> HandlerResult {
    let uid = user_id_of_msg(&msg).unwrap_or(0);
    if !is_admin(uid, &cfg.admin_ids) {
        tracing::warn!(user_id = uid, "отклонён доступ (message)");
        bot.send_message(msg.chat.id, "⛔ Доступ запрещён.").await?;
        return Ok(());
    }

    let state = dialogue.get().await?.unwrap_or_default();
    match state {
        State::AwaitingName => {
            let name = msg.text().unwrap_or_default().to_string();
            match crate::vpn::validate::validate_name(&name) {
                Ok(valid) => {
                    bot.send_message(msg.chat.id, format!("Клиент: {valid}\nВыберите срок действия:"))
                        .reply_markup(menu::expiry_menu())
                        .await?;
                    dialogue.update(State::AwaitingExpiry { name: valid }).await?;
                }
                Err(e) => {
                    bot.send_message(msg.chat.id, format!("⚠️ {e}\nВведите имя ещё раз:")).await?;
                }
            }
        }
        State::AwaitingCustomExpiry { name } => {
            let raw = msg.text().unwrap_or_default().to_string();
            match crate::vpn::validate::validate_expiry(&raw) {
                Ok(exp) => {
                    finish_add(&bot, msg.chat.id, &vpn, &name, Some(&exp)).await;
                    dialogue.exit().await?;
                }
                Err(e) => {
                    bot.send_message(msg.chat.id, format!("⚠️ {e}")).await?;
                }
            }
        }
        _ => {
            // /start и всё прочее — показать меню
            bot.send_message(msg.chat.id, "🔐 AmneziaWG — управление VPN")
                .reply_markup(menu::main_menu())
                .await?;
            dialogue.update(State::Idle).await?;
        }
    }
    Ok(())
}

async fn finish_add(bot: &Bot, chat: ChatId, vpn: &Vpn, name: &str, expires: Option<&str>) {
    let waiting = bot.send_message(chat, "⏳ Создаю клиента…").await.ok();
    match vpn.add(name, expires).await {
        Ok(res) => {
            if let Err(e) = render::send_client_files(bot, chat, &res).await {
                tracing::error!(error = %e, "не удалось отправить файлы клиента");
                let _ = bot.send_message(chat, e.user_message()).await;
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "add провалился");
            let _ = bot.send_message(chat, e.user_message()).await;
        }
    }
    if let Some(m) = waiting {
        let _ = bot.delete_message(chat, m.id).await;
    }
    let _ = bot.send_message(chat, "Готово.").reply_markup(menu::main_menu()).await;
}

async fn callback_handler(
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
    cfg: Arc<Config>,
    vpn: Arc<Vpn>,
) -> HandlerResult {
    bot.answer_callback_query(q.id.clone()).await.ok();

    let uid = user_id_of_cb(&q);
    if !is_admin(uid, &cfg.admin_ids) {
        tracing::warn!(user_id = uid, "отклонён доступ (callback)");
        return Ok(());
    }

    let chat = match &q.message {
        Some(MaybeInaccessibleMessage::Regular(m)) => m.chat.id,
        Some(MaybeInaccessibleMessage::Inaccessible(m)) => m.chat.id,
        None => return Ok(()),
    };

    let data = q.data.clone().unwrap_or_default();
    match parse_callback(&data) {
        Action::Menu => {
            dialogue.update(State::Idle).await?;
            bot.send_message(chat, "🔐 AmneziaWG").reply_markup(menu::main_menu()).await?;
        }
        Action::List => match vpn.list().await {
            Ok(clients) if clients.is_empty() => {
                bot.send_message(chat, "Пока нет клиентов.").reply_markup(menu::main_menu()).await?;
            }
            Ok(clients) => {
                bot.send_message(chat, "👥 Клиенты:")
                    .reply_markup(menu::clients_list(&clients, 0, 8))
                    .await?;
            }
            Err(e) => {
                tracing::error!(error = %e, "list провалился");
                bot.send_message(chat, e.user_message()).await?;
            }
        },
        Action::Page(p) => match vpn.list().await {
            Ok(clients) => {
                bot.send_message(chat, "👥 Клиенты:")
                    .reply_markup(menu::clients_list(&clients, p, 8))
                    .await?;
            }
            Err(e) => { bot.send_message(chat, e.user_message()).await?; }
        },
        Action::Stats => match vpn.stats().await {
            Ok(clients) => {
                bot.send_message(chat, format_stats(&clients))
                    .reply_markup(menu::main_menu())
                    .await?;
            }
            Err(e) => { bot.send_message(chat, e.user_message()).await?; }
        },
        Action::ShowClient(name) => match vpn.list().await {
            Ok(clients) => match clients.iter().find(|c| c.name == name) {
                Some(c) => {
                    bot.send_message(chat, format_client_card(c))
                        .reply_markup(menu::client_card(&name))
                        .await?;
                }
                None => { bot.send_message(chat, "Клиент не найден.").await?; }
            },
            Err(e) => { bot.send_message(chat, e.user_message()).await?; }
        },
        Action::SendConf(name) => {
            // Повторная выдача: читаем уже существующие .conf/.png/.vpnuri из clients_dir.
            match vpn.existing_files(&name) {
                Ok(res) => {
                    if let Err(e) = render::send_client_files(&bot, chat, &res).await {
                        bot.send_message(chat, e.user_message()).await?;
                    }
                }
                Err(e) => { bot.send_message(chat, e.user_message()).await?; }
            }
        }
        Action::AskDelete(name) => {
            bot.send_message(chat, format!("Точно удалить {name}?"))
                .reply_markup(menu::confirm_delete(&name))
                .await?;
        }
        Action::ConfirmDelete(name) => match vpn.remove(&name).await {
            Ok(()) => {
                bot.send_message(chat, format!("🗑 Клиент {name} удалён."))
                    .reply_markup(menu::main_menu())
                    .await?;
            }
            Err(e) => {
                tracing::error!(error = %e, "remove провалился");
                bot.send_message(chat, e.user_message()).await?;
            }
        },
        Action::Add => {
            bot.send_message(chat, "Введите имя клиента:").await?;
            dialogue.update(State::AwaitingName).await?;
        }
        Action::Expiry(kind) => {
            let name = match dialogue.get().await?.unwrap_or_default() {
                State::AwaitingExpiry { name } => name,
                _ => {
                    bot.send_message(chat, "Сессия устарела. Начните заново.").reply_markup(menu::main_menu()).await?;
                    return Ok(());
                }
            };
            if kind == "custom" {
                bot.send_message(chat, "Введите срок (например 10d, 12h, 3w):").await?;
                dialogue.update(State::AwaitingCustomExpiry { name }).await?;
            } else {
                let expires = if kind == "none" { None } else { Some(kind.as_str()) };
                finish_add(&bot, chat, &vpn, &name, expires).await;
                dialogue.exit().await?;
            }
        }
        Action::Unknown => {
            bot.send_message(chat, "Неизвестное действие.").await?;
        }
    }
    Ok(())
}

pub fn schema() -> teloxide::dispatching::UpdateHandler<Box<dyn std::error::Error + Send + Sync>> {
    // Зависимости (Arc<Vpn>, Arc<Config>, InMemStorage<State>) регистрируются в main через dptree::deps!.
    dptree::entry()
        .enter_dialogue::<Update, InMemStorage<State>, State>()
        .branch(Update::filter_message().endpoint(message_handler))
        .branch(Update::filter_callback_query().endpoint(callback_handler))
}
```

> `vpn.existing_files(&name)` определён в Task 6 (кнопка «📄 Конфиг» отдаёт уже созданные файлы из `clients_dir`).

- [ ] **Step 6: Проверить компиляцию всего проекта**

Run: `cargo build`
Expected: успешная сборка. Исправить сигнатуры teloxide при несовпадении типов (`MaybeInaccessibleMessage`, `q.from.id.0`, `UpdateHandler`) — сверять с локальной версией `teloxide 0.17` через `cargo doc --open` при необходимости.

- [ ] **Step 7: Прогнать все тесты**

Run: `cargo test`
Expected: PASS (все юнит- и интеграционные тесты).

- [ ] **Step 8: Commit**

```bash
git add src/bot/mod.rs src/bot/handlers.rs src/vpn/mod.rs
git commit -m "feat: обработчики команд/callback, диалоги add и dptree-схема"
```

---

## Task 10: Точка входа main и запуск диспетчера

**Files:**
- Modify: `src/main.rs`

**Interfaces:**
- Consumes: `config::Config`, `vpn::Vpn`, `bot::{schema, State}`, `bot::handlers::schema` (реэкспорт через `bot::schema`).
- Produces: рабочий бинарник.

- [ ] **Step 1: Реализовать `main.rs`**

```rust
use std::path::PathBuf;
use std::sync::Arc;

use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::prelude::*;

use awg_bot::bot::{handlers, State};
use awg_bot::config::Config;
use awg_bot::vpn::Vpn;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();

    let cfg_path = std::env::var("AWG_BOT_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/etc/awg-bot/config.toml"));

    let cfg = match Config::load(&cfg_path) {
        Ok(c) => Arc::new(c),
        Err(e) => {
            tracing::error!(error = %e, path = %cfg_path.display(), "не удалось загрузить конфиг");
            std::process::exit(1);
        }
    };
    tracing::info!(admins = cfg.admin_ids.len(), "конфиг загружен");

    let bot = Bot::new(&cfg.bot_token);
    let vpn = Arc::new(Vpn::from_config(&cfg));

    tracing::info!("запуск long polling");
    Dispatcher::builder(bot, handlers::schema())
        .dependencies(dptree::deps![InMemStorage::<State>::new(), cfg, vpn])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}
```

> `handlers::schema` должен быть `pub`. Убедиться, что в `src/bot/mod.rs` есть `pub mod handlers;`. Тип возвращаемого `schema()` и генерики зависимостей сверить с teloxide 0.17.

- [ ] **Step 2: Собрать проект**

Run: `cargo build --release`
Expected: успешная сборка бинарника `target/release/awg-bot`.

- [ ] **Step 3: Быстрая проверка запуска на невалидном конфиге (fail-fast)**

Run: `AWG_BOT_CONFIG=/no/such.toml ./target/release/awg-bot; echo "exit=$?"`
Expected: лог ошибки конфига и `exit=1`.

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: main — загрузка конфига и запуск teloxide long polling"
```

---

## Task 11: Деплой-артефакты, README и ручной smoke-тест

**Files:**
- Create: `deploy/config.example.toml`
- Create: `deploy/awg-bot.service`
- Create: `README.md`

**Interfaces:**
- Consumes: готовый бинарник (Task 10).
- Produces: инструкции и systemd-юнит; подтверждённая сверка схемы `--json`.

- [ ] **Step 1: `deploy/config.example.toml`**

```toml
# Скопируйте в /etc/awg-bot/config.toml и заполните.
# Токен лучше хранить в /etc/awg-bot/env (AWG_BOT_TOKEN=...), а не здесь.
bot_token     = ""                              # или через env AWG_BOT_TOKEN
admin_ids     = [111111111]                     # ваш Telegram user ID (узнать: @userinfobot)
manage_script = "/root/awg/manage_amneziawg.sh"
clients_dir   = "/root/awg"
sudo_prefix   = ""                              # "" если сервис от root; "sudo" для hardened-режима
op_timeout_secs = 60
```

- [ ] **Step 2: `deploy/awg-bot.service`**

```ini
[Unit]
Description=AmneziaWG Telegram Bot
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
# По умолчанию — root (простой режим). Для hardened: User=awg-bot + sudoers (см. README).
ExecStart=/usr/local/bin/awg-bot
Environment=AWG_BOT_CONFIG=/etc/awg-bot/config.toml
EnvironmentFile=-/etc/awg-bot/env
Restart=on-failure
RestartSec=5
NoNewPrivileges=false

[Install]
WantedBy=multi-user.target
```

- [ ] **Step 3: `README.md`**

Написать README с разделами:
- Что это и как работает (кратко из спеки).
- Сборка: `cargo build --release` (и опционально musl-таргет).
- Установка: копирование бинарника в `/usr/local/bin/`, конфига в `/etc/awg-bot/`, `env` с правами `chmod 600`.
- Настройка Telegram: создать бота у @BotFather, узнать свой user ID у @userinfobot, вписать в `admin_ids`.
- Режимы привилегий:
  - Простой: сервис от root, `sudo_prefix=""`.
  - Hardened: юзер `awg-bot`, `sudo_prefix="sudo"`, строка sudoers:
    `awg-bot ALL=(root) NOPASSWD: /root/awg/manage_amneziawg.sh`
    и `User=awg-bot` в юните (учесть доступ на чтение файлов в `/root/awg` — при необходимости перенести `clients_dir`).
- Запуск: `systemctl enable --now awg-bot`, просмотр логов `journalctl -u awg-bot -f`.
- **Smoke-чеклист** (см. Step 4).

- [ ] **Step 4: Ручной smoke-тест на тестовом сервере**

Выполнить и отметить результат в README/PR:
1. Установить бота, запустить сервис, `journalctl` показывает «конфиг загружен» и «запуск long polling».
2. `/start` от админа → появляется главное меню; от не-админа → «Доступ запрещён» (+ warn в логах).
3. **Сверка схемы `--json`** (закрывает Task 4 Step 0): «Добавить» → ввести имя → выбрать срок → бот прислал `.conf`, QR и URI. Если парсинг упал (`Не удалось разобрать ответ сервера`) — сверить реальный JSON с `model.rs`, поправить структуры/фикстуры, повторить `cargo test` и пересобрать.
4. «Клиенты» → список; карточка показывает трафик; «Конфиг» повторно присылает файлы.
5. «Статистика» → сводка.
6. Удаление → подтверждение → клиент пропал из списка.
7. Проверить, что срок «✏️ Свой» принимает `10d` и отклоняет мусор.

- [ ] **Step 5: Commit**

```bash
git add deploy/ README.md
git commit -m "docs: деплой, systemd-юнит, README и smoke-чеклист"
```

---

## Итоговая проверка

- [ ] `cargo test` — все тесты зелёные.
- [ ] `cargo build --release` — бинарник собран.
- [ ] `cargo clippy -- -D warnings` — предупреждений нет (при наличии clippy).
- [ ] Smoke-чеклист (Task 11 Step 4) пройден на реальном/тестовом сервере, схема `--json` подтверждена.
- [ ] Секреты не логируются; не-админ не может выполнить ни одной операции.
