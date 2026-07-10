# AmneziaWG Bot — фичи v2 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Добавить в бот язык RU/EN (per-admin, персистентно), опциональный PresharedKey, backup/restore и check/status, и перевести форматирование сообщений на HTML.

**Architecture:** Аддитивно к v1. Две новые опорные части: `SettingsStore` (персистентный `state.json`: глобальный `psk_default` + язык по `user_id`) и `i18n` (`Lang` + локализованные сообщения). Слой `vpn/` получает `backup/list_backups/restore/check` и `run_capture`; слой `bot/` локализуется, переходит на HTML parse mode и получает новые экраны. Порядок задач: сначала аддитивные основы (компилируются независимо), затем локализация-свип существующего бота, затем фичи по одной.

**Tech Stack:** Rust 2021, teloxide 0.17 (rustls), tokio 1.39, serde/serde_json, thiserror, tracing, regex. Без новых зависимостей.

## Global Constraints

- Rust edition 2021; teloxide 0.17 (rustls, HTML parse mode). Никаких новых зависимостей.
- Язык — **per-admin** (`langs: HashMap<i64, Lang>`); дефолт PSK — **глобальный** (`psk_default: bool`). Оба в одном `state.json`.
- Все вызовы скрипта — только через `runner` (`Command`, без шелла); пользовательские аргументы валидируются (`validate_name`/`validate_expiry`) до передачи.
- Бэкапы адресуются **по индексу** свежего `list_backups()` (не по имени в callback-data); имя файла валидируется перед `restore`.
- `check` использует `run_capture` — ненулевой код выхода (найдены проблемы) **не** ошибка, вывод показывается.
- HTML: любые динамические/серверные данные (имена, IP, URI, вывод скрипта) экранируются `html_escape` перед вставкой. `.conf`/QR/архивы уходят документами.
- Каждый хендлер возвращает `Result` и не роняет диспетчер; ошибки пользователю — через `i18n::error_text(lang, &e)`.
- Секреты и содержимое `.conf`/QR никогда не логируются. Запись `state.json` атомарная (tmp+rename), ошибка записи логируется, не роняет бота.
- Коммиты — Conventional Commits, по задаче.

---

## File Structure

```
src/
  i18n.rs        — НОВЫЙ: enum Lang (serde), html_escape, локализованные сообщения, error_text
  settings.rs    — НОВЫЙ: BotState, SettingsStore (персистентный state.json)
  lib.rs         — +pub mod i18n; +pub mod settings;
  config.rs      — +поле state_file
  vpn/runner.rs  — +run_capture
  vpn/mod.rs     — +backup/list_backups/restore/check; BackupFile; add(psk)
  bot/mod.rs     — State: +AwaitingPsk
  bot/menu.rs    — все клавиатуры принимают lang; +language_select/settings/backup_menu/backup_card/psk_step/confirm_restore
  bot/render.rs  — HTML + lang + использование i18n
  bot/handlers.rs— язык-гейт, настройки, PSK-шаг, backup/restore/check; новые Action/parse_callback; локализованные ошибки; +settings dep
  main.rs        — загрузка SettingsStore, регистрация в deps!
README.md        — разделы про язык/PSK/backup/restore/check + поле state_file
```

---

## Task 1: i18n-основа — Lang и html_escape

**Files:**
- Create: `src/i18n.rs`
- Modify: `src/lib.rs` (добавить `pub mod i18n;`)
- Test: юнит-тесты в `src/i18n.rs`

**Interfaces:**
- Produces: `pub enum Lang { Ru, En }` (Default=Ru, `Clone,Copy,PartialEq,Eq,Serialize,Deserialize`), `pub fn html_escape(s: &str) -> String`, `pub fn parse_lang(code: &str) -> Option<Lang>` (`"ru"`/`"en"`), `pub fn lang_code(l: Lang) -> &'static str`.

- [ ] **Step 1: Написать `src/i18n.rs` с падающими тестами**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Lang {
    #[default]
    Ru,
    En,
}

pub fn parse_lang(code: &str) -> Option<Lang> {
    match code {
        "ru" => Some(Lang::Ru),
        "en" => Some(Lang::En),
        _ => None,
    }
}

pub fn lang_code(l: Lang) -> &'static str {
    match l {
        Lang::Ru => "ru",
        Lang::En => "en",
    }
}

pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_html_specials() {
        assert_eq!(html_escape("a<b>&c"), "a&lt;b&gt;&amp;c");
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("plain"), "plain");
    }

    #[test]
    fn amp_escaped_first() {
        // & должен экранироваться до < и >, иначе получим двойное экранирование
        assert_eq!(html_escape("<"), "&lt;");
        assert!(!html_escape("a & b").contains("&amp;amp;"));
    }

    #[test]
    fn lang_roundtrip() {
        assert_eq!(parse_lang("ru"), Some(Lang::Ru));
        assert_eq!(parse_lang("en"), Some(Lang::En));
        assert_eq!(parse_lang("xx"), None);
        assert_eq!(lang_code(Lang::Ru), "ru");
        assert_eq!(lang_code(Lang::En), "en");
        assert_eq!(Lang::default(), Lang::Ru);
    }
}
```

- [ ] **Step 2: Добавить `pub mod i18n;` в `src/lib.rs`** (в алфавитном порядке: после `error`, перед `settings`/`vpn` — пока просто добавить строку).

- [ ] **Step 3: Запустить тесты** — `cargo test --lib i18n` → PASS (3 теста). Проверить `cargo build`.

- [ ] **Step 4: Commit** — `git add src/i18n.rs src/lib.rs && git commit -m "feat(i18n): Lang и html_escape"`

---

## Task 2: SettingsStore и config.state_file

**Files:**
- Create: `src/settings.rs`
- Modify: `src/lib.rs` (`pub mod settings;`), `src/config.rs` (поле `state_file`)
- Test: юнит-тесты в `src/settings.rs` и в `src/config.rs`

**Interfaces:**
- Consumes: `crate::i18n::Lang`.
- Produces:
  - `pub struct BotState { pub psk_default: bool, pub langs: HashMap<i64, Lang> }` (serde, Default).
  - `pub struct SettingsStore { path: PathBuf, state: Mutex<BotState> }`.
  - `impl SettingsStore { pub fn load(path: PathBuf) -> Self; pub fn lang(&self, uid: i64) -> Lang; pub fn has_lang(&self, uid: i64) -> bool; pub fn set_lang(&self, uid: i64, lang: Lang); pub fn psk_default(&self) -> bool; pub fn set_psk_default(&self, v: bool); }`
  - `Config` получает поле `pub state_file: PathBuf` (дефолт `/etc/awg-bot/state.json`).

- [ ] **Step 1: Написать `src/settings.rs` с падающими тестами**

```rust
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::i18n::Lang;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct BotState {
    #[serde(default)]
    pub psk_default: bool,
    #[serde(default)]
    pub langs: HashMap<i64, Lang>,
}

pub struct SettingsStore {
    path: PathBuf,
    state: Mutex<BotState>,
}

impl SettingsStore {
    pub fn load(path: PathBuf) -> Self {
        let state = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<BotState>(&s).ok())
            .unwrap_or_default();
        SettingsStore { path, state: Mutex::new(state) }
    }

    fn persist(&self, state: &BotState) {
        let tmp = self.path.with_extension("json.tmp");
        match serde_json::to_string_pretty(state) {
            Ok(json) => {
                if std::fs::write(&tmp, json).and_then(|_| std::fs::rename(&tmp, &self.path)).is_err() {
                    tracing::error!(path = %self.path.display(), "не удалось сохранить state.json");
                }
            }
            Err(e) => tracing::error!(error = %e, "сериализация state.json"),
        }
    }

    pub fn lang(&self, uid: i64) -> Lang {
        self.state.lock().unwrap().langs.get(&uid).copied().unwrap_or_default()
    }

    pub fn has_lang(&self, uid: i64) -> bool {
        self.state.lock().unwrap().langs.contains_key(&uid)
    }

    pub fn set_lang(&self, uid: i64, lang: Lang) {
        let mut s = self.state.lock().unwrap();
        s.langs.insert(uid, lang);
        let snapshot = s.clone();
        drop(s);
        self.persist(&snapshot);
    }

    pub fn psk_default(&self) -> bool {
        self.state.lock().unwrap().psk_default
    }

    pub fn set_psk_default(&self, v: bool) {
        let mut s = self.state.lock().unwrap();
        s.psk_default = v;
        let snapshot = s.clone();
        drop(s);
        self.persist(&snapshot);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, SettingsStore) {
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::load(dir.path().join("state.json"));
        (dir, store)
    }

    #[test]
    fn defaults_when_empty() {
        let (_d, s) = store();
        assert_eq!(s.lang(1), Lang::Ru);
        assert!(!s.has_lang(1));
        assert!(!s.psk_default());
    }

    #[test]
    fn per_user_lang_and_global_psk() {
        let (_d, s) = store();
        s.set_lang(1, Lang::En);
        s.set_lang(2, Lang::Ru);
        s.set_psk_default(true);
        assert_eq!(s.lang(1), Lang::En);
        assert!(s.has_lang(1));
        assert_eq!(s.lang(2), Lang::Ru);
        assert_eq!(s.lang(3), Lang::Ru); // не задан → дефолт
        assert!(s.psk_default());
    }

    #[test]
    fn persists_across_reload() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        {
            let s = SettingsStore::load(path.clone());
            s.set_lang(42, Lang::En);
            s.set_psk_default(true);
        }
        let s2 = SettingsStore::load(path);
        assert_eq!(s2.lang(42), Lang::En);
        assert!(s2.psk_default());
    }
}
```

- [ ] **Step 2: Добавить `pub mod settings;` в `src/lib.rs`.**

- [ ] **Step 3: Добавить поле `state_file` в `Config`** (`src/config.rs`):
  - В `struct Config` добавить `pub state_file: PathBuf,`.
  - В `impl Debug` добавить `.field("state_file", &self.state_file)`.
  - В `struct Raw` добавить `#[serde(default = "default_state_file")] state_file: PathBuf,` и функцию:
    ```rust
    fn default_state_file() -> PathBuf { PathBuf::from("/etc/awg-bot/state.json") }
    ```
  - В конструкторе `Config { ... }` в `load` добавить `state_file: raw.state_file,`.
  - Добавить тест в `config.rs`: конфиг без `state_file` → `cfg.state_file == PathBuf::from("/etc/awg-bot/state.json")`; с явным значением → оно используется. (Дополнить существующий `loads_valid_config` или новый тест `state_file_defaults`.)

- [ ] **Step 4: Запустить тесты** — `cargo test --lib settings` и `cargo test --lib config` → PASS. `cargo build`.

- [ ] **Step 5: Commit** — `git commit -m "feat(settings): персистентный SettingsStore + config.state_file"`

---

## Task 3: i18n — каталог сообщений и error_text

**Files:**
- Modify: `src/i18n.rs` (добавить сообщения + `error_text`)
- Test: юнит-тесты в `src/i18n.rs`

**Interfaces:**
- Consumes: `crate::error::Error`.
- Produces: набор `pub fn <msg>(lang: Lang, args…) -> String` + `pub fn error_text(lang: Lang, err: &Error) -> String`. Все динамические строковые аргументы экранируются `html_escape` внутри функций.

Реализовать функции ниже **дословно** (RU/EN тексты — часть спецификации). Паттерн для каждой: `match lang { Lang::Ru => …, Lang::En => … }`. Где есть динамические данные — оборачивать в `html_escape(...)`.

- [ ] **Step 1: Добавить тесты (RED)** в конец `#[cfg(test)] mod tests`:

```rust
#[test]
fn all_messages_nonempty_both_langs() {
    for l in [Lang::Ru, Lang::En] {
        assert!(!menu_title(l).is_empty());
        assert!(!access_denied(l).is_empty());
        assert!(!ask_client_name(l).is_empty());
        assert!(!ask_expiry(l).is_empty());
        assert!(!settings_title(l, true).is_empty());
        assert!(!backups_empty(l).is_empty());
        assert!(!restore_done(l).is_empty());
        // карточка: имя экранируется
        let card = client_card(l, "a<b>", "Активен", "10.0.0.2", "1 KB", "0 B", "никогда", "бессрочно");
        assert!(card.contains("a&lt;b&gt;"));
        assert!(!card.contains("a<b>"));
    }
}

#[test]
fn error_text_covers_variants() {
    use crate::error::Error;
    for l in [Lang::Ru, Lang::En] {
        for e in [
            Error::Timeout,
            Error::Parse("x".into()),
            Error::ScriptFailed { code: Some(1), stderr: "secret".into() },
            Error::Telegram("x".into()),
        ] {
            let t = error_text(l, &e);
            assert!(!t.is_empty());
            assert!(!t.contains("secret")); // stderr не утекает
        }
    }
}
```

- [ ] **Step 2: Реализовать сообщения (GREEN).** Полный набор (labels кнопок, экраны, PSK, backup, restore, check, settings). Пример нескольких — остальные по тому же шаблону:

```rust
use crate::error::Error;

// --- экран выбора языка (без lang: показывает оба варианта) ---
pub fn choose_language() -> String {
    "🌐 Выберите язык / Choose language:".to_string()
}

// --- меню ---
pub fn menu_title(lang: Lang) -> String {
    match lang { Lang::Ru => "🔐 <b>AmneziaWG</b>", Lang::En => "🔐 <b>AmneziaWG</b>" }.to_string()
}
pub fn btn_clients(lang: Lang) -> String { match lang { Lang::Ru => "👥 Клиенты", Lang::En => "👥 Clients" }.to_string() }
pub fn btn_add(lang: Lang) -> String { match lang { Lang::Ru => "➕ Добавить", Lang::En => "➕ Add" }.to_string() }
pub fn btn_stats(lang: Lang) -> String { match lang { Lang::Ru => "📊 Статистика", Lang::En => "📊 Stats" }.to_string() }
pub fn btn_backup(lang: Lang) -> String { match lang { Lang::Ru => "💾 Бэкап", Lang::En => "💾 Backup" }.to_string() }
pub fn btn_check(lang: Lang) -> String { match lang { Lang::Ru => "🩺 Проверка", Lang::En => "🩺 Check" }.to_string() }
pub fn btn_settings(lang: Lang) -> String { match lang { Lang::Ru => "⚙️ Настройки", Lang::En => "⚙️ Settings" }.to_string() }
pub fn btn_back(lang: Lang) -> String { match lang { Lang::Ru => "⬅️ В меню", Lang::En => "⬅️ Menu" }.to_string() }

pub fn access_denied(lang: Lang) -> String {
    match lang { Lang::Ru => "⛔ Доступ запрещён.", Lang::En => "⛔ Access denied." }.to_string()
}

// --- add-диалог ---
pub fn ask_client_name(lang: Lang) -> String {
    match lang { Lang::Ru => "Введите имя клиента:", Lang::En => "Enter client name:" }.to_string()
}
pub fn bad_name(lang: Lang) -> String {
    match lang { Lang::Ru => "⚠️ Некорректное имя (латиница/цифры/-/_, 1–32). Введите ещё раз:", Lang::En => "⚠️ Invalid name (a-z0-9-_, 1–32). Try again:" }.to_string()
}
pub fn ask_expiry(lang: Lang) -> String {
    match lang { Lang::Ru => "Выберите срок действия:", Lang::En => "Choose expiry:" }.to_string()
}
pub fn ask_custom_expiry(lang: Lang) -> String {
    match lang { Lang::Ru => "Введите срок (например 10d, 12h, 3w):", Lang::En => "Enter duration (e.g. 10d, 12h, 3w):" }.to_string()
}
pub fn bad_expiry(lang: Lang) -> String {
    match lang { Lang::Ru => "⚠️ Формат срока: Nh/Nd/Nw (например 10d).", Lang::En => "⚠️ Duration format: Nh/Nd/Nw (e.g. 10d)." }.to_string()
}
pub fn psk_step(lang: Lang, default_on: bool) -> String {
    let d = if default_on { "вкл/on" } else { "выкл/off" };
    match lang {
        Lang::Ru => format!("PresharedKey (по умолчанию: {d}). Создать клиента:"),
        Lang::En => format!("PresharedKey (default: {d}). Create client:"),
    }
}
pub fn btn_create_with_psk(lang: Lang) -> String { match lang { Lang::Ru => "🔐 С PSK", Lang::En => "🔐 With PSK" }.to_string() }
pub fn btn_create_no_psk(lang: Lang) -> String { match lang { Lang::Ru => "🔓 Без PSK", Lang::En => "🔓 No PSK" }.to_string() }
pub fn creating(lang: Lang) -> String { match lang { Lang::Ru => "⏳ Создаю клиента…", Lang::En => "⏳ Creating client…" }.to_string() }
pub fn import_link(lang: Lang, uri: &str) -> String {
    let u = html_escape(uri);
    match lang {
        Lang::Ru => format!("🔗 Ссылка для импорта:\n<code>{u}</code>"),
        Lang::En => format!("🔗 Import link:\n<code>{u}</code>"),
    }
}

// --- карточка/статистика (динамика экранируется) ---
#[allow(clippy::too_many_arguments)]
pub fn client_card(lang: Lang, name: &str, status: &str, ip: &str, rx: &str, tx: &str, handshake: &str, expires: &str) -> String {
    let (name, status, ip) = (html_escape(name), html_escape(status), html_escape(ip));
    let ip_line = if ip.is_empty() { String::new() } else { match lang { Lang::Ru => format!("IP: {ip}\n"), Lang::En => format!("IP: {ip}\n") } };
    match lang {
        Lang::Ru => format!("👤 <b>{name}</b>\nСтатус: {status}\n{ip_line}Трафик:  ↓ {rx}   ↑ {tx}\nРукопожатие: {handshake}\nДействует: {expires}"),
        Lang::En => format!("👤 <b>{name}</b>\nStatus: {status}\n{ip_line}Traffic:  ↓ {rx}   ↑ {tx}\nHandshake: {handshake}\nExpires: {expires}"),
    }
}
pub fn stats_summary(lang: Lang, total: usize, active: usize, rx: &str, tx: &str) -> String {
    match lang {
        Lang::Ru => format!("📊 <b>Статистика</b>\nВсего клиентов: {total}\nАктивных: {active}\nТрафик суммарно: ↓ {rx}  ↑ {tx}"),
        Lang::En => format!("📊 <b>Stats</b>\nTotal clients: {total}\nActive: {active}\nTotal traffic: ↓ {rx}  ↑ {tx}"),
    }
}
pub fn clients_empty(lang: Lang) -> String { match lang { Lang::Ru => "Пока нет клиентов.", Lang::En => "No clients yet." }.to_string() }
pub fn clients_title(lang: Lang) -> String { match lang { Lang::Ru => "👥 <b>Клиенты</b>:", Lang::En => "👥 <b>Clients</b>:" }.to_string() }
pub fn not_found(lang: Lang) -> String { match lang { Lang::Ru => "Клиент не найден.", Lang::En => "Client not found." }.to_string() }
pub fn confirm_delete(lang: Lang, name: &str) -> String {
    let n = html_escape(name);
    match lang { Lang::Ru => format!("Точно удалить <b>{n}</b>?"), Lang::En => format!("Delete <b>{n}</b>?") }
}
pub fn deleted(lang: Lang, name: &str) -> String {
    let n = html_escape(name);
    match lang { Lang::Ru => format!("🗑 Клиент {n} удалён."), Lang::En => format!("🗑 Client {n} removed.") }
}
pub fn done(lang: Lang) -> String { match lang { Lang::Ru => "Готово.", Lang::En => "Done." }.to_string() }

// --- настройки ---
pub fn settings_title(lang: Lang, psk_default: bool) -> String {
    let psk = if psk_default { "вкл/on" } else { "выкл/off" };
    match lang {
        Lang::Ru => format!("⚙️ <b>Настройки</b>\nЯзык: русский\nPSK по умолчанию: {psk}"),
        Lang::En => format!("⚙️ <b>Settings</b>\nLanguage: English\nDefault PSK: {psk}"),
    }
}
pub fn btn_lang_ru(lang: Lang) -> String { let _ = lang; "🇷🇺 Русский".to_string() }
pub fn btn_lang_en(lang: Lang) -> String { let _ = lang; "🇬🇧 English".to_string() }
pub fn btn_psk_toggle(lang: Lang, on: bool) -> String {
    match (lang, on) {
        (Lang::Ru, true) => "PSK: вкл ✅", (Lang::Ru, false) => "PSK: выкл ⬜",
        (Lang::En, true) => "PSK: on ✅", (Lang::En, false) => "PSK: off ⬜",
    }.to_string()
}

// --- backup / restore ---
pub fn btn_backup_new(lang: Lang) -> String { match lang { Lang::Ru => "➕ Создать бэкап", Lang::En => "➕ Create backup" }.to_string() }
pub fn btn_backup_list(lang: Lang) -> String { match lang { Lang::Ru => "📃 Список бэкапов", Lang::En => "📃 List backups" }.to_string() }
pub fn backup_menu_title(lang: Lang) -> String { match lang { Lang::Ru => "💾 <b>Бэкап</b>", Lang::En => "💾 <b>Backup</b>" }.to_string() }
pub fn backup_creating(lang: Lang) -> String { match lang { Lang::Ru => "⏳ Создаю бэкап…", Lang::En => "⏳ Creating backup…" }.to_string() }
pub fn backup_done(lang: Lang, filename: &str) -> String {
    let f = html_escape(filename);
    match lang { Lang::Ru => format!("✅ Бэкап создан:\n<code>{f}</code>"), Lang::En => format!("✅ Backup created:\n<code>{f}</code>") }
}
pub fn backups_empty(lang: Lang) -> String { match lang { Lang::Ru => "Бэкапов пока нет.", Lang::En => "No backups yet." }.to_string() }
pub fn backups_list_title(lang: Lang) -> String { match lang { Lang::Ru => "📃 <b>Бэкапы</b>:", Lang::En => "📃 <b>Backups</b>:" }.to_string() }
pub fn btn_download(lang: Lang) -> String { match lang { Lang::Ru => "📥 Скачать", Lang::En => "📥 Download" }.to_string() }
pub fn btn_restore(lang: Lang) -> String { match lang { Lang::Ru => "♻️ Восстановить", Lang::En => "♻️ Restore" }.to_string() }
pub fn confirm_restore(lang: Lang, filename: &str) -> String {
    let f = html_escape(filename);
    match lang { Lang::Ru => format!("♻️ Восстановить из <code>{f}</code>? Текущее состояние будет заменено."), Lang::En => format!("♻️ Restore from <code>{f}</code>? Current state will be replaced.") }
}
pub fn btn_confirm(lang: Lang) -> String { match lang { Lang::Ru => "✅ Да", Lang::En => "✅ Yes" }.to_string() }
pub fn restoring(lang: Lang) -> String { match lang { Lang::Ru => "⏳ Восстанавливаю…", Lang::En => "⏳ Restoring…" }.to_string() }
pub fn restore_done(lang: Lang) -> String { match lang { Lang::Ru => "✅ Восстановление завершено.", Lang::En => "✅ Restore complete." }.to_string() }

// --- check ---
pub fn check_running(lang: Lang) -> String { match lang { Lang::Ru => "⏳ Проверяю сервер…", Lang::En => "⏳ Checking server…" }.to_string() }
pub fn check_result(lang: Lang, body: &str) -> String {
    let b = html_escape(body);
    match lang { Lang::Ru => format!("🩺 <b>Проверка</b>\n<pre>{b}</pre>"), Lang::En => format!("🩺 <b>Check</b>\n<pre>{b}</pre>") }
}

// --- ошибки (локализованные, без утечки stderr) ---
pub fn error_text(lang: Lang, err: &Error) -> String {
    match (lang, err) {
        (Lang::Ru, Error::Timeout) => "⏳ Превышено время ожидания. Попробуйте позже.",
        (Lang::En, Error::Timeout) => "⏳ Operation timed out. Try later.",
        (Lang::Ru, Error::ScriptFailed { .. }) => "❌ Операция не удалась. Попробуйте ещё раз.",
        (Lang::En, Error::ScriptFailed { .. }) => "❌ Operation failed. Try again.",
        (Lang::Ru, Error::Parse(_)) => "❌ Не удалось разобрать ответ сервера.",
        (Lang::En, Error::Parse(_)) => "❌ Failed to parse server response.",
        (Lang::Ru, _) => "❌ Ошибка выполнения операции.",
        (Lang::En, _) => "❌ Operation error.",
    }.to_string()
}
```
Реализовать ВСЕ перечисленные функции. Динамика (`name`, `ip`, `uri`, `filename`, `body`, `status`) — только через `html_escape`. `rx/tx/handshake/expires` приходят уже как готовые строки от `render` (там числа/пути форматируются и, где нужно, экранируются — см. Task 5).

- [ ] **Step 3: Запустить тесты** — `cargo test --lib i18n` → PASS. `cargo build`.

- [ ] **Step 4: Commit** — `git commit -m "feat(i18n): каталог сообщений RU/EN и error_text"`

---

## Task 4: Vpn — run_capture, backup, list_backups, restore, check

**Files:**
- Modify: `src/vpn/runner.rs` (+`run_capture`), `src/vpn/mod.rs` (+методы, `BackupFile`)
- Test: юнит/интеграционные тесты со скриптами-заглушками

**Interfaces:**
- Consumes: `runner::{run, run_capture, RunSpec}`, `error::{Error, Result}`.
- Produces:
  - `runner::run_capture(spec, args) -> Result<(String, i32)>` — stdout и код выхода; тайм-аут → `Error::Timeout`; ненулевой код НЕ ошибка.
  - `pub struct BackupFile { pub name: String, pub path: PathBuf, pub size: u64, pub mtime: i64 }`
  - `impl Vpn { pub async fn backup(&self) -> Result<BackupFile>; pub fn list_backups(&self) -> Result<Vec<BackupFile>>; pub async fn restore(&self, index: usize) -> Result<()>; pub async fn check(&self) -> Result<String>; }`
  - `list_backups`: читает `clients_dir/backups/`, только `*.tar.gz`, сортировка по `mtime` убыв.

- [ ] **Step 1: `run_capture` в `src/vpn/runner.rs`** (рядом с `run`):

```rust
/// Как `run`, но возвращает stdout и код выхода независимо от успеха.
/// Тайм-аут по-прежнему → Error::Timeout. Нужен для `check` (код 1 = «проблемы», не ошибка).
pub async fn run_capture(spec: &RunSpec<'_>, args: &[&str]) -> Result<(String, i32)> {
    let mut cmd = if spec.sudo_prefix.is_empty() {
        let mut c = Command::new(spec.script); c.args(args); c
    } else {
        let mut c = Command::new(spec.sudo_prefix); c.arg(spec.script); c.args(args); c
    };
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
    let child = cmd.spawn()?;
    let dur = Duration::from_secs(spec.timeout_secs);
    let output = match timeout(dur, child.wait_with_output()).await {
        Ok(res) => res?,
        Err(_) => return Err(Error::Timeout),
    };
    let mut out = String::from_utf8_lossy(&output.stdout).into_owned();
    if out.is_empty() {
        out = String::from_utf8_lossy(&output.stderr).into_owned();
    }
    Ok((out, output.status.code().unwrap_or(-1)))
}
```

- [ ] **Step 2: Интеграционный тест `run_capture`** в `tests/runner_integration.rs`:

```rust
#[tokio::test]
async fn run_capture_returns_output_on_nonzero() {
    let (_d, script) = make_script("#!/bin/sh\necho diag\nexit 1\n");
    let spec = RunSpec { script: &script, sudo_prefix: "", timeout_secs: 5 };
    let (out, code) = awg_bot::vpn::runner::run_capture(&spec, &["check"]).await.unwrap();
    assert!(out.contains("diag"));
    assert_eq!(code, 1);
}
```
Run: `cargo test --test runner_integration run_capture` — сначала FAIL (нет функции), после Step 1 — PASS.

- [ ] **Step 3: `BackupFile` + методы в `src/vpn/mod.rs`.** Добавить импорт `use crate::vpn::runner::run_capture;` и:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct BackupFile {
    pub name: String,
    pub path: std::path::PathBuf,
    pub size: u64,
    pub mtime: i64,
}

impl Vpn {
    fn backups_dir(&self) -> std::path::PathBuf {
        self.clients_dir.join("backups")
    }

    pub fn list_backups(&self) -> Result<Vec<BackupFile>> {
        let dir = self.backups_dir();
        let mut out = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for e in entries.flatten() {
                let path = e.path();
                let name = e.file_name().to_string_lossy().into_owned();
                if !name.ends_with(".tar.gz") { continue; }
                let meta = match e.metadata() { Ok(m) => m, Err(_) => continue };
                let mtime = meta.modified().ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                out.push(BackupFile { name, path, size: meta.len(), mtime });
            }
        }
        out.sort_by(|a, b| b.mtime.cmp(&a.mtime));
        Ok(out)
    }

    pub async fn backup(&self) -> Result<BackupFile> {
        run(&self.spec(), &["backup"]).await?;
        self.list_backups()?
            .into_iter()
            .next()
            .ok_or_else(|| crate::error::Error::Parse("бэкап не найден после создания".into()))
    }

    pub async fn restore(&self, index: usize) -> Result<()> {
        let backups = self.list_backups()?;
        let bf = backups.get(index)
            .ok_or_else(|| crate::error::Error::Parse("бэкап не найден".into()))?;
        // basename-валидация: имя без разделителей пути и по шаблону
        if bf.name.contains('/') || !bf.name.starts_with("awg_backup_") || !bf.name.ends_with(".tar.gz") {
            return Err(crate::error::Error::Parse("некорректное имя бэкапа".into()));
        }
        let path = bf.path.to_string_lossy().into_owned();
        run(&self.spec(), &["restore", &path]).await?;
        Ok(())
    }

    pub async fn check(&self) -> Result<String> {
        let (out, _code) = run_capture(&self.spec(), &["check"]).await?;
        Ok(out)
    }
}
```

- [ ] **Step 4: Тесты фасада** в `#[cfg(test)] mod tests` (`src/vpn/mod.rs`), используя существующий хелпер `vpn_with_script`:

```rust
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
```
> Примечание: `vpn_with_script` кладёт скрипт в temp `clients_dir`; путь `backups` — `clients_dir/backups`. Если хелпер устроен иначе, тесты создают каталог `backups` вручную (как выше).

- [ ] **Step 5: Запустить** — `cargo test` (все зелёные), `cargo build`.

- [ ] **Step 6: Commit** — `git commit -m "feat(vpn): backup/list_backups/restore/check + run_capture"`

---

## Task 5: Локализация + HTML существующего бота, язык-гейт, экран настроек (язык)

> Самая крупная задача: связный «свип» слоя `bot/`. Все клавиатуры получают `lang`, все сообщения — через `i18n`, parse_mode → HTML, добавляется выбор языка при старте и экран настроек со сменой языка. Новые фичи (PSK/backup/check) — в следующих задачах. Проверка — компиляция + существующие сценарии; юнит-тест — `parse_callback` (новые action-ы языка/настроек).

**Files:**
- Modify: `src/bot/menu.rs`, `src/bot/render.rs`, `src/bot/handlers.rs`, `src/main.rs`
- Test: юнит-тесты `parse_callback` в `handlers.rs`; round-trip menu↔parse_callback обновить

**Interfaces:**
- `menu.rs`: все функции получают первым аргументом `lang: Lang`. Новые: `pub fn language_select() -> InlineKeyboardMarkup` (data `lang:ru`/`lang:en`), `pub fn settings_menu(lang, psk_default: bool) -> InlineKeyboardMarkup` (data `set:lang:ru`/`set:lang:en`/`set:psk:on`/`set:psk:off`/`menu`). `main_menu(lang)` расширяется кнопками backup/check/settings (data `backup`,`check`,`settings`).
- `render.rs`: `format_client_card`/`format_stats` получают `lang` и возвращают HTML через `i18n` (числа форматируются здесь, статус/имя/ip передаются в i18n, где экранируются). `send_client_files(bot, chat, lang, res)` — URI через `i18n::import_link` + `.parse_mode(Html)`.
- `handlers.rs`: сигнатуры хендлеров получают `settings: Arc<SettingsStore>`. Везде `lang = settings.lang(uid)`. Ошибки — `i18n::error_text(lang, &e)`.
- `main.rs`: создать `Arc<SettingsStore>`, зарегистрировать в `deps!`.

- [ ] **Step 1: `main.rs` — загрузка SettingsStore и регистрация.**
Добавить `use awg_bot::settings::SettingsStore;`. После создания `vpn`:
```rust
let settings = std::sync::Arc::new(SettingsStore::load(cfg.state_file.clone()));
```
и в `deps!`:
```rust
.dependencies(dptree::deps![InMemStorage::<State>::new(), cfg, vpn, settings])
```

- [ ] **Step 2: `menu.rs` — добавить `lang` и экраны.**
Импортировать `use crate::i18n::{self, Lang};`. Изменить сигнатуры на `pub fn main_menu(lang: Lang)`, `expiry_menu(lang)`, `clients_list(lang, clients, page, per_page)`, `client_card(lang, name)`, `confirm_delete(lang, name)`. Тексты кнопок — из `i18n::btn_*`. `main_menu` теперь:
```rust
pub fn main_menu(lang: Lang) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![cb(&i18n::btn_clients(lang), "list")],
        vec![cb(&i18n::btn_add(lang), "add")],
        vec![cb(&i18n::btn_stats(lang), "stats")],
        vec![cb(&i18n::btn_backup(lang), "backup")],
        vec![cb(&i18n::btn_check(lang), "check")],
        vec![cb(&i18n::btn_settings(lang), "settings")],
    ])
}
pub fn language_select() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![
        cb("🇷🇺 Русский", "lang:ru"), cb("🇬🇧 English", "lang:en"),
    ]])
}
pub fn settings_menu(lang: Lang, psk_default: bool) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![cb(&i18n::btn_lang_ru(lang), "set:lang:ru"), cb(&i18n::btn_lang_en(lang), "set:lang:en")],
        vec![cb(&i18n::btn_psk_toggle(lang, psk_default), if psk_default { "set:psk:off" } else { "set:psk:on" })],
        vec![cb(&i18n::btn_back(lang), "menu")],
    ])
}
```
Кнопка «В меню» во всех клавиатурах — `i18n::btn_back(lang)` с data `menu`. Обновить тесты `menu.rs` под новые сигнатуры (передавать `Lang::Ru`); callback-data не меняются для существующих кнопок.

- [ ] **Step 3: `render.rs` — HTML + lang.**
`format_client_card(lang, c, now, expiry)` → форматирует числа (`human_bytes`), строки статуса/handshake/expires, зовёт `i18n::client_card(lang, &c.name, &status, &c.ip, &rx, &tx, &handshake, &expires)`. `format_stats(lang, clients)` → `i18n::stats_summary(...)`. `send_client_files(bot, chat, lang, res)` → документ `.conf`, фото QR (если файл есть), и `bot.send_message(chat, i18n::import_link(lang, &res.uri)).parse_mode(ParseMode::Html)`. Обновить тесты render под новые сигнатуры (проверять, что имя экранируется и присутствует «1.2 GB»).

- [ ] **Step 4: `handlers.rs` — settings dep, язык-гейт, настройки, HTML, локализованные ошибки.**
  - Добавить `settings: Arc<SettingsStore>` в оба хендлера и во все внутренние вызовы, которым нужен `lang`.
  - Хелпер: `let lang = settings.lang(uid);` после auth.
  - `/start` (default-ветка message_handler): если `!settings.has_lang(uid)` → `bot.send_message(chat, i18n::choose_language()).reply_markup(menu::language_select())` (без parse_mode), иначе меню `i18n::menu_title(lang)` + `menu::main_menu(lang)` с `.parse_mode(Html)`.
  - Все `send_message` с разметкой — `.parse_mode(ParseMode::Html)`.
  - Заменить строковые литералы на `i18n::*(lang)`; `e.user_message()` → `i18n::error_text(lang, &e)`.
  - `finish_add` получает `lang` (и пока прежнюю сигнатуру `vpn.add(name, expires)` — PSK в Task 6).
  - Новые Action-ы: `Lang(String)` (для `lang:ru|en`), `Settings`, `SetLang(String)`, `SetPsk(bool)`. Расширить `parse_callback`:
    ```rust
    "settings" => Action::Settings,
    // ... в блоке strip_prefix:
    else if let Some(v) = data.strip_prefix("set:lang:") { Action::SetLang(v.to_string()) }
    else if let Some(v) = data.strip_prefix("set:psk:") { Action::SetPsk(v == "on") }
    else if let Some(v) = data.strip_prefix("lang:") { Action::Lang(v.to_string()) }
    ```
    (Разместить `set:lang:`/`set:psk:` до общего `lang:`/др.; порядок префиксов — как с `delyes:`/`del:`.)
  - Обработчики callback:
    - `Lang(code)`: `if let Some(l) = i18n::parse_lang(&code) { settings.set_lang(uid, l); }` → показать меню на новом языке.
    - `Settings`: `bot.send_message(chat, i18n::settings_title(lang, settings.psk_default())).reply_markup(menu::settings_menu(lang, settings.psk_default())).parse_mode(Html)`.
    - `SetLang(code)`: сохранить, перерисовать экран настроек на новом языке.
    - `SetPsk(on)`: `settings.set_psk_default(on)` → перерисовать настройки.
  - Обновить unit-тест `parses_all_actions` новыми кейсами (`lang:ru`→`Lang("ru")`, `settings`→`Settings`, `set:lang:en`→`SetLang("en")`, `set:psk:on`→`SetPsk(true)`).
  - ПРИМЕЧАНИЕ: кнопки `backup`/`check` уже присутствуют в `main_menu`, но их action-ы (`Backup`/`Check`) добавляются в Task 7/8. До тех пор тап по ним даёт `Action::Unknown` → «неизвестное действие»; это ожидаемо при инкрементальной поставке. Поэтому в `all_menu_callback_data_parse_to_known_actions` временно исключить `backup`/`check` из проверки (или пометить как ожидаемо-Unknown), а в Task 7/8 вернуть их в проверку.

- [ ] **Step 5: Компиляция и тесты** — `cargo build`; `cargo test` (все зелёные, включая обновлённые menu/render/handlers тесты); `cargo clippy --all-targets`.

- [ ] **Step 6: Commit** — `git commit -m "feat(bot): локализация RU/EN, HTML parse mode, выбор языка и настройки"`

---

## Task 6: PSK — глобальный дефолт в настройках и шаг в диалоге add

**Files:**
- Modify: `src/bot/mod.rs` (State), `src/vpn/mod.rs` (add psk), `src/bot/menu.rs` (psk_step), `src/bot/handlers.rs`
- Test: юнит-тесты (`parse_callback`, add-стаб с `--psk`)

**Interfaces:**
- `State += AwaitingPsk { name: String, expires: Option<String> }`.
- `Vpn::add(&self, name, expires: Option<&str>, psk: bool)` — при `psk` добавляет `--psk`.
- `menu::psk_step(lang, default_on) -> InlineKeyboardMarkup` (data `add:psk:on`/`add:psk:off`).
- Action `AddPsk(bool)`.

- [ ] **Step 1: `Vpn::add` — psk-параметр (+ обновить единственный вызов).**
```rust
pub async fn add(&self, name: &str, expires: Option<&str>, psk: bool) -> Result<AddResult> {
    let name = validate::validate_name(name).map_err(|e| crate::error::Error::Parse(e.to_string()))?;
    let mut args: Vec<String> = vec!["add".into(), name.clone()];
    if let Some(exp) = expires {
        let exp = validate::validate_expiry(exp).map_err(|e| crate::error::Error::Parse(e.to_string()))?;
        args.push(format!("--expires={exp}"));
    }
    if psk { args.push("--psk".into()); }
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    run(&self.spec(), &arg_refs).await?;
    self.existing_files(&name)
}
```
Обновить тест `add_rejects_bad_name_before_running` → `vpn.add("bad;", None, false)`. Добавить тест `add_passes_psk_flag`: стаб-скрипт, который при наличии `--psk` создаёт `<name>.conf`, иначе `exit 1`; `vpn.add("alice", None, true)` → Ok. (Проверяет, что флаг реально передан.)

- [ ] **Step 2: `State += AwaitingPsk`** в `src/bot/mod.rs`:
```rust
AwaitingPsk { name: String, expires: Option<String> },
```
Добавить ветку `case![State::AwaitingPsk { .. }]`? Нет — PSK выбирается callback-кнопкой, не текстом; состояние читается в `callback_handler` (как `AwaitingExpiry`). Дополнительных message-веток не нужно.

- [ ] **Step 3: `menu::psk_step`:**
```rust
pub fn psk_step(lang: Lang, default_on: bool) -> InlineKeyboardMarkup {
    let (first, second) = if default_on {
        (cb(&i18n::btn_create_with_psk(lang), "add:psk:on"), cb(&i18n::btn_create_no_psk(lang), "add:psk:off"))
    } else {
        (cb(&i18n::btn_create_no_psk(lang), "add:psk:off"), cb(&i18n::btn_create_with_psk(lang), "add:psk:on"))
    };
    InlineKeyboardMarkup::new(vec![vec![first, second], vec![cb(&i18n::btn_back(lang), "menu")]])
}
```
(Дефолтная опция — первой.)

- [ ] **Step 4: Диалог add в `handlers.rs`.**
  - Изменить `Action::Expiry` и `AwaitingCustomExpiry`: вместо немедленного `finish_add` — переход в `AwaitingPsk { name, expires }` и показ `menu::psk_step(lang, settings.psk_default())` с текстом `i18n::psk_step(lang, settings.psk_default())`.
    - `exp:none` → `expires=None`; пресеты → `Some(kind)`; `custom` → как раньше (`AwaitingCustomExpiry`), после ввода срока → `AwaitingPsk { name, expires: Some(exp) }`.
  - Новый Action `AddPsk(bool)` (`parse_callback`: `else if let Some(v) = data.strip_prefix("add:psk:") { Action::AddPsk(v == "on") }`, разместить до `add`-точного и общих префиксов).
  - Обработка `AddPsk(psk)`: прочитать состояние `AwaitingPsk { name, expires }` (иначе «сессия устарела»), затем `finish_add(&bot, chat, lang, &vpn, &name, expires.as_deref(), psk)`, `dialogue.exit()`.
  - `finish_add` получает доп. параметр `psk: bool` и зовёт `vpn.add(name, expires, psk)`.
  - Обновить unit-тест `parses_all_actions`: `add:psk:on`→`AddPsk(true)`, `add:psk:off`→`AddPsk(false)`.

- [ ] **Step 5: Компиляция и тесты** — `cargo build`, `cargo test`, `cargo clippy --all-targets`.

- [ ] **Step 6: Commit** — `git commit -m "feat(bot): опциональный PSK — глобальный дефолт и шаг в add"`

---

## Task 7: Backup и Restore в боте

**Files:**
- Modify: `src/bot/menu.rs`, `src/bot/handlers.rs`
- Test: `parse_callback` юнит-тесты

**Interfaces:**
- `menu::backup_menu(lang) -> InlineKeyboardMarkup` (data `bk:new`/`bk:list`/`menu`), `menu::backups_list(lang, &[BackupFile]) -> InlineKeyboardMarkup` (по индексу: кнопка на бэкап data `bk:card:<idx>` + `menu`), `menu::backup_card(lang, idx) -> InlineKeyboardMarkup` (data `bk:dl:<idx>`/`bk:restore:<idx>`/`menu`), `menu::confirm_restore(lang, idx) -> InlineKeyboardMarkup` (data `bk:restore_yes:<idx>`/`menu`).
- Actions: `Backup`, `BackupNew`, `BackupList`, `BackupCard(usize)`, `BackupDownload(usize)`, `Restore(usize)`, `RestoreYes(usize)`.

- [ ] **Step 1: Клавиатуры backup в `menu.rs`** (по шаблону выше; `backups_list` рендерит `bk:card:<idx>` с именем файла как текст — имя экранировать не нужно в тексте кнопки, Telegram кнопки — plain text).

- [ ] **Step 2: `parse_callback` — новые action-ы** (в `handlers.rs`), с корректным порядком префиксов (специфичные `bk:restore_yes:` до `bk:restore:`; `bk:dl:`, `bk:card:`, `bk:new`/`bk:list` точные):
```rust
"backup" => Action::Backup,
"bk:new" => Action::BackupNew,
"bk:list" => Action::BackupList,
// strip_prefix, порядок важен:
else if let Some(v) = data.strip_prefix("bk:restore_yes:") { v.parse().map(Action::RestoreYes).unwrap_or(Action::Unknown) }
else if let Some(v) = data.strip_prefix("bk:restore:") { v.parse().map(Action::Restore).unwrap_or(Action::Unknown) }
else if let Some(v) = data.strip_prefix("bk:card:") { v.parse().map(Action::BackupCard).unwrap_or(Action::Unknown) }
else if let Some(v) = data.strip_prefix("bk:dl:") { v.parse().map(Action::BackupDownload).unwrap_or(Action::Unknown) }
```

- [ ] **Step 3: Обработчики callback:**
  - `Backup`: показать `menu::backup_menu(lang)` + `i18n::backup_menu_title(lang)`.
  - `BackupNew`: «⏳» → `vpn.backup().await`; успех → `i18n::backup_done(lang, &bf.name)` + карточка с «Скачать» (индекс 0, т.к. новейший); ошибка → `error_text`.
  - `BackupList`: `vpn.list_backups()`; пусто → `i18n::backups_empty`; иначе `menu::backups_list(lang, &list)`.
  - `BackupCard(idx)`: показать `menu::backup_card(lang, idx)`.
  - `BackupDownload(idx)`: `vpn.list_backups()` → `[idx]` → `bot.send_document(chat, InputFile::file(&bf.path))` (ошибку слать через `error_text`; вне диапазона → `not_found`/`error`).
  - `Restore(idx)`: подтверждение `i18n::confirm_restore(lang, &name)` + `menu::confirm_restore(lang, idx)`.
  - `RestoreYes(idx)`: «⏳ восстанавливаю» → `vpn.restore(idx).await` → `i18n::restore_done` или `error_text`; вернуть меню.

- [ ] **Step 4: Тесты** — `parse_callback` новыми кейсами (`backup`→`Backup`, `bk:new`→`BackupNew`, `bk:list`→`BackupList`, `bk:restore_yes:2`→`RestoreYes(2)`, `bk:restore:2`→`Restore(2)`, `bk:dl:1`→`BackupDownload(1)`, `bk:card:0`→`BackupCard(0)`); `all_menu_callback_data_parse_to_known_actions` дополнить `backup_menu`/`backup_card`/`confirm_restore`/`backups_list` (с фиктивным `&[BackupFile]`) и вернуть `backup` из `main_menu` в проверку (исключался в Task 5).

- [ ] **Step 5: Компиляция и тесты** — `cargo build`, `cargo test`, `cargo clippy --all-targets`.

- [ ] **Step 6: Commit** — `git commit -m "feat(bot): backup и restore (создание, список, скачивание, восстановление)"`

---

## Task 8: Check/Status в боте

**Files:**
- Modify: `src/bot/handlers.rs`
- Test: `parse_callback` (action `Check` уже добавлен в Task 5 через кнопку меню — если нет, добавить здесь)

**Interfaces:**
- Action `Check` (добавляется здесь; кнопка меню `check` уже есть с Task 5).

- [ ] **Step 1: Action `Check`** — добавить `"check" => Action::Check` в exact-match блок `parse_callback` и вернуть `check` в проверку `all_menu_callback_data_parse_to_known_actions`.

- [ ] **Step 2: Обработчик `Check`:**
```rust
Action::Check => {
    let waiting = bot.send_message(chat, i18n::check_running(lang)).await.ok();
    match vpn.check().await {
        Ok(body) => {
            let body = if body.len() > 3500 { format!("{}\n…", &body[..3500]) } else { body };
            bot.send_message(chat, i18n::check_result(lang, &body))
                .parse_mode(teloxide::types::ParseMode::Html)
                .reply_markup(menu::main_menu(lang))
                .await?;
        }
        Err(e) => { bot.send_message(chat, i18n::error_text(lang, &e)).await?; }
    }
    if let Some(m) = waiting { let _ = bot.delete_message(chat, m.id).await; }
}
```
(Обрезка до ~3500 символов — лимит Telegram 4096; `<pre>` + экранирование в `i18n::check_result`.)

- [ ] **Step 3: Тест `parse_callback`** — `check`→`Action::Check` (если не покрыт в Task 5).

- [ ] **Step 4: Компиляция и тесты** — `cargo build`, `cargo test`, `cargo clippy --all-targets`.

- [ ] **Step 5: Commit** — `git commit -m "feat(bot): check/status — диагностика сервера"`

---

## Task 9: README, сборка, smoke-чеклист

**Files:**
- Modify: `README.md`, `deploy/config.example.toml`

**Interfaces:** документация; артефакт — обновлённый статический бинарник.

- [ ] **Step 1: `deploy/config.example.toml`** — добавить строку `state_file = "/etc/awg-bot/state.json"` с комментарием (per-admin язык + глобальный PSK-дефолт).

- [ ] **Step 2: README** — новые разделы:
  - «Язык» — выбор при `/start`, смена в ⚙️ Настройки (per-admin).
  - «PSK» — глобальный дефолт в Настройках + переопределение при добавлении; флаг `--psk`.
  - «Backup/Restore» — создать/скачать/список/восстановить (из бэкапов на сервере в `clients_dir/backups/`).
  - «Проверка» — `check/status`.
  - «Настройки хранятся в `state_file`» (дефолт `/etc/awg-bot/state.json`).
  - Обновить пример карточки/меню (HTML, новые пункты).

- [ ] **Step 3: Пересборка бинарника** — `./scripts/build-musl.sh` → `dist/awg-bot-linux-amd64`. Убедиться `file dist/...` → `statically linked`. (Требует Docker.)

- [ ] **Step 4: Ручной smoke-чеклист (в README, не выполнять здесь без сервера):**
  1. Первый `/start` → выбор языка → меню на выбранном языке.
  2. ⚙️ Настройки → сменить язык → интерфейс переключился; PSK-дефолт вкл/выкл сохраняется (проверить `cat state.json`).
  3. ➕ Добавить → имя → срок → шаг PSK → создан клиент; при «С PSK» в конфиге присутствует PresharedKey.
  4. 👥 Клиенты → карточка (HTML, трафик/рукопожатие/срок), «Конфиг».
  5. 💾 Бэкап → создать → «Скачать» приходит .tar.gz; список показывает архивы; восстановление из выбранного бэкапа.
  6. 🩺 Проверка → приходит вывод диагностики в `<pre>`.
  7. Рестарт бота → язык и PSK-дефолт сохранились (персистентность state.json).

- [ ] **Step 5: Commit** — `git commit -m "docs: README и config для фич v2; пересборка бинарника"`

---

## Итоговая проверка
- [ ] `cargo test` — все зелёные.
- [ ] `cargo build --release` и `./scripts/build-musl.sh` — собирается; `dist/awg-bot-linux-amd64` статический.
- [ ] `cargo clippy --all-targets` — без предупреждений.
- [ ] state.json: язык per-admin и глобальный psk_default сохраняются и переживают рестарт.
- [ ] HTML-экранирование: имя/URI/вывод check не ломают разметку; секреты/stderr не в чате.
