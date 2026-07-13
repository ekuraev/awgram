# Паритет с manage: regen, diagnose, метка срока — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Открыть в Telegram-боте три возможности manage-скрипта: метку срока действия в списке клиентов, диагностику сервера (`diagnose`) и перевыпуск конфигов (`regen` одного клиента и всех, с опцией `--reset-routes`).

**Architecture:** Три независимых вертикальных среза по существующим паттернам: чистая функция в `vpn/model.rs` → метод в `vpn/mod.rs` (через `run`/`run_capture`) → строки в `i18n.rs` (RU/EN) → клавиатуры в `bot/menu.rs` → `Action` + обработчик в `bot/handlers.rs`. Спека: `docs/superpowers/specs/2026-07-14-manage-parity-design.md`.

**Tech Stack:** Rust, teloxide, tokio; тесты — стандартные `#[test]`/`#[tokio::test]` со стаб-скриптами (`vpn_with_script`).

## Global Constraints

- TDD строго: тест пишется и наблюдается падающим до реализации.
- Все пользовательские строки — через `i18n.rs`, RU и EN; имена клиентов в текстах сообщений — через `html_escape` (в кнопках экранирование не нужно — Telegram не рендерит HTML в кнопках).
- Вывод скрипта в чат не отдаётся при ошибках (секрет-гигиена) — только локализованные тексты `error_text`.
- Прогон перед каждым коммитом: `cargo test --lib` зелёный, `cargo clippy --all-targets` без новых предупреждений.
- Коммиты в стиле репозитория: `feat(vpn): …`, `feat(bot): …`, `feat(i18n): …`, `docs(readme): …`.

---

### Task 1: `format_expiry_badge` в `vpn/model.rs`

**Files:**
- Modify: `src/vpn/model.rs` (функция после `format_expiry`, тесты в конец `mod tests`)

**Interfaces:**
- Consumes: `Lang` (уже импортирован в model.rs).
- Produces: `pub fn format_expiry_badge(lang: Lang, now: i64, exp: Option<i64>) -> Option<String>` — компактная метка для кнопки списка: `None` для бессрочных; `Some("⏳ истёк")`/`Some("⏳ expired")` для истёкших; `Some("⏳ 6д")`/`Some("⏳ 6d")` (дни), `Some("⏳ 5ч")`/`Some("⏳ 5h")` (часы), `Some("⏳ <1ч")`/`Some("⏳ <1h")`.

- [ ] **Step 1: Write the failing tests**

В конец `mod tests` в `src/vpn/model.rs`:

```rust
    #[test]
    fn expiry_badge_none_for_permanent() {
        assert_eq!(format_expiry_badge(Lang::Ru, 1_700_000_000, None), None);
    }

    #[test]
    fn expiry_badge_days() {
        let now = 1_700_000_000;
        assert_eq!(
            format_expiry_badge(Lang::Ru, now, Some(now + 6 * 86400)),
            Some("⏳ 6д".to_string())
        );
        assert_eq!(
            format_expiry_badge(Lang::En, now, Some(now + 6 * 86400)),
            Some("⏳ 6d".to_string())
        );
    }

    #[test]
    fn expiry_badge_hours() {
        let now = 1_700_000_000;
        assert_eq!(
            format_expiry_badge(Lang::Ru, now, Some(now + 5 * 3600)),
            Some("⏳ 5ч".to_string())
        );
        assert_eq!(
            format_expiry_badge(Lang::En, now, Some(now + 5 * 3600)),
            Some("⏳ 5h".to_string())
        );
    }

    #[test]
    fn expiry_badge_under_hour() {
        let now = 1_700_000_000;
        assert_eq!(
            format_expiry_badge(Lang::Ru, now, Some(now + 600)),
            Some("⏳ <1ч".to_string())
        );
        assert_eq!(
            format_expiry_badge(Lang::En, now, Some(now + 600)),
            Some("⏳ <1h".to_string())
        );
    }

    #[test]
    fn expiry_badge_expired() {
        let now = 1_700_000_000;
        assert_eq!(
            format_expiry_badge(Lang::Ru, now, Some(now)),
            Some("⏳ истёк".to_string())
        );
        assert_eq!(
            format_expiry_badge(Lang::En, now, Some(now - 1)),
            Some("⏳ expired".to_string())
        );
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib expiry_badge`
Expected: ошибка компиляции `cannot find function format_expiry_badge`.

- [ ] **Step 3: Write minimal implementation**

В `src/vpn/model.rs` сразу после `format_expiry` (после строки 109):

```rust
/// Компактная метка срока для кнопки списка клиентов. None → бессрочный
/// клиент (метка не показывается). Пороги — как у `format_expiry`.
pub fn format_expiry_badge(lang: Lang, now: i64, exp: Option<i64>) -> Option<String> {
    let e = exp?;
    let d = e - now;
    let text = if d <= 0 {
        match lang { Lang::Ru => "⏳ истёк".to_string(), Lang::En => "⏳ expired".to_string() }
    } else if d >= 86400 {
        match lang {
            Lang::Ru => format!("⏳ {}д", d / 86400),
            Lang::En => format!("⏳ {}d", d / 86400),
        }
    } else if d >= 3600 {
        match lang {
            Lang::Ru => format!("⏳ {}ч", d / 3600),
            Lang::En => format!("⏳ {}h", d / 3600),
        }
    } else {
        match lang { Lang::Ru => "⏳ <1ч".to_string(), Lang::En => "⏳ <1h".to_string() }
    };
    Some(text)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib expiry_badge`
Expected: 5 passed. Затем `cargo test --lib` — все зелёные.

- [ ] **Step 5: Commit**

```bash
git add src/vpn/model.rs
git commit -m "feat(model): format_expiry_badge — компактная метка срока для списка"
```

---

### Task 2: метка ⏳ в кнопках списка клиентов

**Files:**
- Modify: `src/bot/menu.rs` (функция `clients_list`, её тесты)
- Modify: `src/bot/handlers.rs` (арм `Action::List` ~строка 322, `Action::Page` ~строка 339, `Action::ShowClient` ~строка 361; новый хелпер `now_epoch`)

**Interfaces:**
- Consumes: `format_expiry_badge(lang, now, exp)` из Task 1; `vpn.client_expiry(&name) -> Option<i64>` (существует).
- Produces: новая сигнатура `pub fn clients_list(lang: Lang, clients: &[Client], expiries: &[Option<i64>], now: i64, page: usize, per_page: usize) -> InlineKeyboardMarkup` (`expiries[i]` соответствует `clients[i]`; отсутствующий индекс = без метки); `fn now_epoch() -> i64` в `handlers.rs`.

- [ ] **Step 1: Write the failing test**

В `mod tests` в `src/bot/menu.rs` (рядом с `clients_list_one_button_per_client`). Понадобится хелпер для текстов кнопок — добавить рядом с `all_callback_data`:

```rust
    fn all_button_texts(kb: &InlineKeyboardMarkup) -> Vec<String> {
        kb.inline_keyboard.iter().flatten().map(|b| b.text.clone()).collect()
    }

    #[test]
    fn clients_list_shows_expiry_badge() {
        let clients = vec![
            Client { name: "temp".into(), ip: String::new(), client_ipv6: String::new(), status: String::new(), status_code: "active".into(), rx: 0, tx: 0, last_handshake: None },
            Client { name: "perm".into(), ip: String::new(), client_ipv6: String::new(), status: String::new(), status_code: "active".into(), rx: 0, tx: 0, last_handshake: None },
        ];
        let now = 1_700_000_000;
        let expiries = vec![Some(now + 6 * 86400), None];
        let texts = all_button_texts(&clients_list(Lang::Ru, &clients, &expiries, now, 0, 10));
        assert!(texts.iter().any(|t| t.contains("temp") && t.contains("⏳ 6д")), "temp должен иметь метку: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("perm") && !t.contains("⏳")), "perm должен быть без метки: {texts:?}");
    }
```

Существующие тесты `clients_list_one_button_per_client` и `clients_list_zero_per_page_no_panic` обновить под новую сигнатуру — в обоих вызовах `clients_list(Lang::Ru, &clients, ...)` заменить на `clients_list(Lang::Ru, &clients, &[], 0, <page>, <per_page>)` (пустой срез expiries = без меток; `now` = 0 достаточно).

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib clients_list`
Expected: ошибка компиляции (сигнатура не совпадает — «this function takes 4 arguments»).

- [ ] **Step 3: Write implementation**

В `src/bot/menu.rs` заменить `clients_list` (строки 84–111) на:

```rust
pub fn clients_list(
    lang: Lang,
    clients: &[Client],
    expiries: &[Option<i64>],
    now: i64,
    page: usize,
    per_page: usize,
) -> InlineKeyboardMarkup {
    if per_page == 0 {
        return InlineKeyboardMarkup::new(vec![vec![cb(&i18n::btn_back(lang), "menu")]]);
    }

    let start = page * per_page;
    let mut rows: Vec<Vec<InlineKeyboardButton>> = clients
        .iter()
        .enumerate()
        .skip(start)
        .take(per_page)
        .map(|(i, c)| {
            let mark = if c.active() { "🟢" } else { "🔴" };
            let exp = expiries.get(i).copied().flatten();
            let label = match crate::vpn::model::format_expiry_badge(lang, now, exp) {
                Some(badge) => format!("{mark} {} {badge}", c.name),
                None => format!("{mark} {}", c.name),
            };
            vec![cb(&label, &format!("client:{}", c.name))]
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
    rows.push(vec![cb(&i18n::btn_back(lang), "menu")]);
    InlineKeyboardMarkup::new(rows)
}
```

В `src/bot/handlers.rs` добавить хелпер (рядом с `user_id_of_cb`, ~строка 116):

```rust
fn now_epoch() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
```

Арм `Action::List` (ветка `Ok(clients)`, ~строка 328) заменить на:

```rust
            Ok(clients) => {
                let expiries: Vec<Option<i64>> =
                    clients.iter().map(|c| vpn.client_expiry(&c.name)).collect();
                bot.send_message(chat, i18n::clients_title(lang))
                    .reply_markup(menu::clients_list(lang, &clients, &expiries, now_epoch(), 0, 8))
                    .parse_mode(ParseMode::Html)
                    .await?;
            }
```

Арм `Action::Page(p)` (ветка `Ok(clients)`, ~строка 340) — аналогично:

```rust
            Ok(clients) => {
                let expiries: Vec<Option<i64>> =
                    clients.iter().map(|c| vpn.client_expiry(&c.name)).collect();
                bot.send_message(chat, i18n::clients_title(lang))
                    .reply_markup(menu::clients_list(lang, &clients, &expiries, now_epoch(), p, 8))
                    .parse_mode(ParseMode::Html)
                    .await?;
            }
```

В арме `Action::ShowClient` (~строки 364–367) заменить инлайновый блок `let now = std::time::SystemTime::now()...unwrap_or(0);` на `let now = now_epoch();` (попутный DRY, поведение не меняется).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib`
Expected: все зелёные, включая `clients_list_shows_expiry_badge`.

- [ ] **Step 5: Commit**

```bash
git add src/bot/menu.rs src/bot/handlers.rs
git commit -m "feat(bot): метка ⏳ срока действия в списке клиентов"
```

---

### Task 3: `Vpn::diagnose`

**Files:**
- Modify: `src/vpn/mod.rs` (метод после `check()`, тесты в `mod tests`)

**Interfaces:**
- Consumes: `run_capture(&self.spec(), args) -> Result<(String, i32)>` (существует).
- Produces: `pub async fn diagnose(&self) -> Result<String>` — вывод `manage diagnose` независимо от кода выхода; пустой вывод → `Err(Error::Parse)`.

- [ ] **Step 1: Write the failing tests**

В `mod tests` в `src/vpn/mod.rs` (рядом с `check_returns_output_even_on_problems`):

```rust
    #[tokio::test]
    async fn diagnose_returns_output_even_on_problems() {
        let (_d, vpn) = vpn_with_script("#!/bin/sh\necho 'DIAG REPORT'\nexit 1\n");
        let out = vpn.diagnose().await.unwrap();
        assert!(out.contains("DIAG REPORT"));
    }

    #[tokio::test]
    async fn diagnose_errors_on_empty_output() {
        let (_d, vpn) = vpn_with_script("#!/bin/sh\nexit 0\n");
        assert!(matches!(vpn.diagnose().await, Err(crate::error::Error::Parse(_))));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib diagnose`
Expected: ошибка компиляции `no method named diagnose`.

- [ ] **Step 3: Write minimal implementation**

В `impl Vpn` в `src/vpn/mod.rs`, после метода `check()`:

```rust
    /// Запускает `diagnose` и возвращает stdout независимо от кода выхода
    /// (как `check`: ненулевой код — «найдены проблемы», а не ошибка).
    /// Пустой вывод — ошибка: диагностика всегда что-то печатает.
    pub async fn diagnose(&self) -> Result<String> {
        let (out, _code) = run_capture(&self.spec(), &["diagnose"]).await?;
        if out.trim().is_empty() {
            return Err(crate::error::Error::Parse("пустой вывод diagnose".into()));
        }
        Ok(out)
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib diagnose`
Expected: 2 passed.

- [ ] **Step 5: Commit**

```bash
git add src/vpn/mod.rs
git commit -m "feat(vpn): diagnose() — расширенная диагностика через run_capture"
```

---

### Task 4: диагностика в UI (кнопка, handler, общий хелпер обрезки)

**Files:**
- Modify: `src/i18n.rs` (строки + тесты)
- Modify: `src/bot/menu.rs` (`main_menu`, тест `main_menu_has_expected_actions`)
- Modify: `src/bot/handlers.rs` (`Action::Diagnose`, `parse_callback`, обработчик, хелпер `truncate_for_message`, рефакторинг арма `Action::Check`, тесты)

**Interfaces:**
- Consumes: `vpn.diagnose() -> Result<String>` из Task 3.
- Produces: `i18n::btn_diagnose(lang)`, `i18n::diagnose_running(lang)`, `i18n::diagnose_result(lang, body)`; callback `"diagnose"` → `Action::Diagnose`; `fn truncate_for_message(body: String) -> String` в `handlers.rs`.

- [ ] **Step 1: Write the failing tests**

В `mod tests` в `src/i18n.rs`:

```rust
    #[test]
    fn diagnose_strings_nonempty_both_langs() {
        for l in [Lang::Ru, Lang::En] {
            assert!(!btn_diagnose(l).is_empty());
            assert!(!diagnose_running(l).is_empty());
            let r = diagnose_result(l, "body <x>");
            assert!(r.contains("<pre>"));
            assert!(r.contains("&lt;x&gt;")); // вывод экранируется
        }
    }
```

В `mod tests` в `src/bot/menu.rs` — обновить `main_menu_has_expected_actions`, добавив `"diagnose"` в перечень:

```rust
        for expected in ["list", "add", "stats", "backup", "check", "diagnose", "settings"] {
```

В `mod tests` в `src/bot/handlers.rs` (рядом с тестами `parse_callback`):

```rust
    #[test]
    fn parse_callback_diagnose() {
        assert_eq!(parse_callback("diagnose"), Action::Diagnose);
    }

    #[test]
    fn truncate_for_message_respects_char_boundary() {
        // Трёхбайтовый символ: 3500 не кратно 3 → индекс попадает внутрь
        // символа, обрезка должна откатиться к границе, а не паниковать.
        let long = "€".repeat(1500); // 4500 байт
        let cut = truncate_for_message(long);
        assert!(cut.ends_with('…'));
        assert!(cut.len() <= 3504); // ≤3500 (до границы символа) + "\n…" (4 байта)
        let short = "ok".to_string();
        assert_eq!(truncate_for_message(short), "ok");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib`
Expected: ошибки компиляции (`btn_diagnose` не найден, вариант `Diagnose` не найден, `truncate_for_message` не найден).

- [ ] **Step 3: Write implementation**

`src/i18n.rs` — рядом с блоком `// --- check ---`:

```rust
pub fn btn_diagnose(lang: Lang) -> String {
    match lang { Lang::Ru => "🔬 Диагностика", Lang::En => "🔬 Diagnostics" }.to_string()
}
pub fn diagnose_running(lang: Lang) -> String {
    match lang { Lang::Ru => "⏳ Диагностирую…", Lang::En => "⏳ Running diagnostics…" }.to_string()
}
pub fn diagnose_result(lang: Lang, body: &str) -> String {
    let b = html_escape(body);
    match lang {
        Lang::Ru => format!("🔬 <b>Диагностика</b>\n<pre>{b}</pre>"),
        Lang::En => format!("🔬 <b>Diagnostics</b>\n<pre>{b}</pre>"),
    }
}
```

`src/bot/menu.rs` — в `main_menu` заменить строку `vec![cb(&i18n::btn_check(lang), "check")],` на:

```rust
        vec![cb(&i18n::btn_check(lang), "check"), cb(&i18n::btn_diagnose(lang), "diagnose")],
```

`src/bot/handlers.rs`:

1. В enum `Action` после `Check,` добавить `Diagnose,`.
2. В `parse_callback`, в блок точных совпадений после `"check" => Action::Check,` добавить:

```rust
        "diagnose" => Action::Diagnose,
```

3. Хелпер рядом с `now_epoch` (вынос существующей инлайн-логики из `Action::Check`):

```rust
/// Обрезает вывод скрипта до лимита Telegram-сообщения (3500 байт, с запасом
/// на HTML-обёртку), округляя вниз до границы UTF-8-символа — байтовый индекс
/// может попасть внутрь многобайтового символа (кириллица в выводе скрипта).
fn truncate_for_message(body: String) -> String {
    if body.len() <= 3500 {
        return body;
    }
    let mut cut = 3500;
    while !body.is_char_boundary(cut) {
        cut -= 1;
    }
    format!("{}\n…", &body[..cut])
}
```

4. В арме `Action::Check` (~строки 608–623) заменить инлайновый блок обрезки (`let body = if body.len() > 3500 { ... };`) на `let body = truncate_for_message(body);` (комментарий про границу UTF-8 переезжает к хелперу).
5. Новый арм после `Action::Check` (по его же образцу):

```rust
        Action::Diagnose => {
            let waiting = bot.send_message(chat, i18n::diagnose_running(lang)).await.ok();
            match vpn.diagnose().await {
                Ok(body) => {
                    let body = truncate_for_message(body);
                    bot.send_message(chat, i18n::diagnose_result(lang, &body))
                        .reply_markup(menu::main_menu(lang))
                        .parse_mode(ParseMode::Html)
                        .await?;
                }
                Err(e) => {
                    tracing::error!(error = %e, "diagnose провалился");
                    bot.send_message(chat, i18n::error_text(lang, &e)).await?;
                }
            }
            if let Some(m) = waiting {
                let _ = bot.delete_message(chat, m.id).await;
            }
        }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib`
Expected: все зелёные, включая новые.

- [ ] **Step 5: Commit**

```bash
git add src/i18n.rs src/bot/menu.rs src/bot/handlers.rs
git commit -m "feat(bot): кнопка 🔬 Диагностика — manage diagnose из главного меню"
```

---

### Task 5: `Vpn::regen_client` и `Vpn::regen_all`

**Files:**
- Modify: `src/vpn/mod.rs` (методы после `remove()`, тесты)

**Interfaces:**
- Consumes: `run`, `run_capture`, `RunSpec`, `existing_files` (существуют).
- Produces: `pub async fn regen_client(&self, name: &str) -> Result<AddResult>`; `pub async fn regen_all(&self, reset_routes: bool) -> Result<bool>` (`Ok(true)` — rc 0, `Ok(false)` — rc ≠ 0, частичные ошибки; таймаут ×3).

- [ ] **Step 1: Write the failing tests**

В `mod tests` в `src/vpn/mod.rs`:

```rust
    #[tokio::test]
    async fn regen_client_runs_script_and_reads_files() {
        // Стаб создаёт conf только при argv "regen <name>" — проверяем и команду, и чтение файлов.
        let (dir, vpn) = vpn_with_script(
            "#!/bin/sh\n[ \"$1\" = regen ] || exit 1\necho conf > \"$(dirname \"$0\")/$2.conf\"\nexit 0\n",
        );
        let res = vpn.regen_client("alice").await.unwrap();
        assert!(res.conf_path.ends_with("alice.conf"));
        drop(dir);
    }

    #[tokio::test]
    async fn regen_client_rejects_bad_name() {
        let (_d, vpn) = vpn_with_script("#!/bin/sh\nexit 0\n");
        assert!(vpn.regen_client("bad name;rm").await.is_err());
    }

    #[tokio::test]
    async fn regen_all_true_on_success_false_on_partial() {
        let (_d, vpn) = vpn_with_script("#!/bin/sh\nexit 0\n");
        assert!(vpn.regen_all(false).await.unwrap());

        let (_d2, vpn2) = vpn_with_script("#!/bin/sh\necho warn\nexit 1\n");
        assert!(!vpn2.regen_all(false).await.unwrap());
    }

    #[tokio::test]
    async fn regen_all_passes_reset_routes_flag() {
        // Стаб успешен ТОЛЬКО при наличии --reset-routes среди аргументов.
        const STUB: &str = "#!/bin/sh\nfor a in \"$@\"; do\n  [ \"$a\" = \"--reset-routes\" ] && exit 0\ndone\nexit 1\n";
        let (_d, vpn) = vpn_with_script(STUB);
        assert!(vpn.regen_all(true).await.unwrap(), "с reset_routes=true флаг должен дойти до скрипта");
        assert!(!vpn.regen_all(false).await.unwrap(), "без reset_routes флага быть не должно");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib regen`
Expected: ошибка компиляции `no method named regen_client` / `regen_all`.

- [ ] **Step 3: Write minimal implementation**

В `impl Vpn` в `src/vpn/mod.rs`, после метода `remove()`:

```rust
    /// Перевыпускает файлы одного клиента (`regen <name>`): ключи и IP
    /// сохраняются, `.conf`/QR/URI создаются заново и читаются с диска.
    pub async fn regen_client(&self, name: &str) -> Result<AddResult> {
        let name = validate::validate_name(name)
            .map_err(|e| crate::error::Error::Parse(e.to_string()))?;
        run(&self.spec(), &["regen", &name]).await?;
        self.existing_files(&name)
    }

    /// Перевыпускает файлы всех клиентов. `Ok(false)` — скрипт завершился с
    /// rc ≠ 0: часть клиентов могла быть перевыпущена («завершено с
    /// предупреждениями»), а не отказ операции. Таймаут ×3 — массовый regen
    /// пропорционален числу клиентов.
    pub async fn regen_all(&self, reset_routes: bool) -> Result<bool> {
        let spec = RunSpec {
            script: &self.script,
            sudo_prefix: &self.sudo_prefix,
            timeout_secs: self.timeout_secs * 3,
        };
        let args: &[&str] = if reset_routes { &["regen", "--reset-routes"] } else { &["regen"] };
        let (_out, code) = run_capture(&spec, args).await?;
        Ok(code == 0)
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib regen`
Expected: 4 passed. Затем `cargo test --lib` — все зелёные.

- [ ] **Step 5: Commit**

```bash
git add src/vpn/mod.rs
git commit -m "feat(vpn): regen_client + regen_all(--reset-routes) — перевыпуск конфигов"
```

---

### Task 6: перевыпуск одного клиента из карточки

**Files:**
- Modify: `src/i18n.rs` (строки + тест)
- Modify: `src/bot/menu.rs` (`client_card`, тест `client_card_encodes_name`)
- Modify: `src/bot/handlers.rs` (`Action::Regen(String)`, `parse_callback`, обработчик, тест)

**Interfaces:**
- Consumes: `vpn.regen_client(&name) -> Result<AddResult>` из Task 5; `render::send_client_files` (существует).
- Produces: `i18n::btn_regen(lang)`, `i18n::regen_running(lang)`; callback `regen:<name>` → `Action::Regen(String)`.

- [ ] **Step 1: Write the failing tests**

`src/i18n.rs`, `mod tests`:

```rust
    #[test]
    fn regen_strings_nonempty_both_langs() {
        for l in [Lang::Ru, Lang::En] {
            assert!(!btn_regen(l).is_empty());
            assert!(!regen_running(l).is_empty());
        }
    }
```

`src/bot/menu.rs`, обновить тест `client_card_encodes_name`:

```rust
    #[test]
    fn client_card_encodes_name() {
        let data = all_callback_data(&client_card(Lang::Ru, "alice"));
        assert!(data.contains(&"conf:alice".to_string()));
        assert!(data.contains(&"del:alice".to_string()));
        assert!(data.contains(&"regen:alice".to_string()));
    }
```

`src/bot/handlers.rs`, `mod tests`:

```rust
    #[test]
    fn parse_callback_regen_client() {
        assert_eq!(parse_callback("regen:alice"), Action::Regen("alice".into()));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib`
Expected: ошибки компиляции (`btn_regen` не найден, вариант `Regen` не найден); тест `client_card_encodes_name` упал бы по assert.

- [ ] **Step 3: Write implementation**

`src/i18n.rs` — рядом с `deleted`/`done`:

```rust
pub fn btn_regen(lang: Lang) -> String {
    match lang { Lang::Ru => "🔄 Перевыпустить", Lang::En => "🔄 Reissue" }.to_string()
}
pub fn regen_running(lang: Lang) -> String {
    match lang { Lang::Ru => "⏳ Перевыпускаю…", Lang::En => "⏳ Reissuing…" }.to_string()
}
```

`src/bot/menu.rs` — в `client_card` добавить ряд с regen между рядом действий и «назад»:

```rust
pub fn client_card(lang: Lang, name: &str) -> InlineKeyboardMarkup {
    let conf_txt = match lang { Lang::Ru => "📄 Конфиг", Lang::En => "📄 Config" };
    let del_txt = match lang { Lang::Ru => "🗑 Удалить", Lang::En => "🗑 Delete" };
    InlineKeyboardMarkup::new(vec![
        vec![cb(conf_txt, &format!("conf:{name}")), cb(del_txt, &format!("del:{name}"))],
        vec![cb(&i18n::btn_regen(lang), &format!("regen:{name}"))],
        vec![cb(&i18n::btn_back(lang), "menu")],
    ])
}
```

`src/bot/handlers.rs`:

1. В enum `Action` после `Recreate(String),` добавить `Regen(String),`.
2. В `parse_callback` в цепочку префиксов, сразу после ветки `recreate:`:

```rust
            } else if let Some(v) = data.strip_prefix("regen:") {
                Action::Regen(v.to_string())
```

3. Новый арм в `callback_handler` (после арма `Action::Recreate`):

```rust
        Action::Regen(name) => {
            let waiting = bot.send_message(chat, i18n::regen_running(lang)).await.ok();
            match vpn.regen_client(&name).await {
                Ok(res) => {
                    if let Err(e) = render::send_client_files(&bot, chat, lang, &res).await {
                        tracing::error!(error = %e, "не удалось отправить файлы после regen");
                        bot.send_message(chat, i18n::error_text(lang, &e)).await?;
                    } else {
                        bot.send_message(chat, i18n::done(lang))
                            .reply_markup(menu::main_menu(lang))
                            .parse_mode(ParseMode::Html)
                            .await?;
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, "regen провалился");
                    bot.send_message(chat, i18n::error_text(lang, &e)).await?;
                }
            }
            if let Some(m) = waiting {
                let _ = bot.delete_message(chat, m.id).await;
            }
        }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib`
Expected: все зелёные.

- [ ] **Step 5: Commit**

```bash
git add src/i18n.rs src/bot/menu.rs src/bot/handlers.rs
git commit -m "feat(bot): 🔄 Перевыпустить в карточке клиента — regen + свежие файлы"
```

---

### Task 7: массовый перевыпуск с подтверждением

**Files:**
- Modify: `src/i18n.rs` (строки + тест)
- Modify: `src/bot/menu.rs` (кнопка в `clients_list`, новая `confirm_regen_all`, тесты)
- Modify: `src/bot/handlers.rs` (`Action::RegenAll`, `Action::RegenAllRun(bool)`, `parse_callback`, обработчики, тесты)

**Interfaces:**
- Consumes: `vpn.regen_all(reset_routes) -> Result<bool>` из Task 5.
- Produces: callbacks `regen_all` → `Action::RegenAll`, `regen_all_go` → `Action::RegenAllRun(false)`, `regen_all_routes` → `Action::RegenAllRun(true)`; `menu::confirm_regen_all(lang)`; строки `i18n::btn_regen_all`, `confirm_regen_all`, `btn_regen_all_go`, `btn_regen_all_routes`, `regen_all_running`, `regen_all_done`, `regen_all_partial`.

- [ ] **Step 1: Write the failing tests**

`src/i18n.rs`, `mod tests`:

```rust
    #[test]
    fn regen_all_strings_nonempty_both_langs() {
        for l in [Lang::Ru, Lang::En] {
            assert!(!btn_regen_all(l).is_empty());
            assert!(!confirm_regen_all(l).is_empty());
            assert!(!btn_regen_all_go(l).is_empty());
            assert!(!btn_regen_all_routes(l).is_empty());
            assert!(!regen_all_running(l).is_empty());
            assert!(!regen_all_done(l).is_empty());
            assert!(!regen_all_partial(l).is_empty());
        }
    }
```

`src/bot/menu.rs`, `mod tests`:

```rust
    #[test]
    fn clients_list_has_regen_all_button() {
        let clients = vec![
            Client { name: "a".into(), ip: String::new(), client_ipv6: String::new(), status: String::new(), status_code: "active".into(), rx: 0, tx: 0, last_handshake: None },
        ];
        let data = all_callback_data(&clients_list(Lang::Ru, &clients, &[], 0, 0, 10));
        assert!(data.contains(&"regen_all".to_string()));
    }

    #[test]
    fn confirm_regen_all_has_three_actions() {
        let data = all_callback_data(&confirm_regen_all(Lang::Ru));
        assert!(data.contains(&"regen_all_go".to_string()));
        assert!(data.contains(&"regen_all_routes".to_string()));
        assert!(data.contains(&"menu".to_string()));
    }
```

`src/bot/handlers.rs`, `mod tests`:

```rust
    #[test]
    fn parse_callback_regen_all_variants() {
        assert_eq!(parse_callback("regen_all"), Action::RegenAll);
        assert_eq!(parse_callback("regen_all_go"), Action::RegenAllRun(false));
        assert_eq!(parse_callback("regen_all_routes"), Action::RegenAllRun(true));
        // "regen_all…" не должен съедаться префиксом "regen:" (там двоеточие).
        assert_eq!(parse_callback("regen:alice"), Action::Regen("alice".into()));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib`
Expected: ошибки компиляции (нет `btn_regen_all`, `confirm_regen_all`, вариантов `RegenAll`/`RegenAllRun`).

- [ ] **Step 3: Write implementation**

`src/i18n.rs` — рядом со строками regen из Task 6:

```rust
pub fn btn_regen_all(lang: Lang) -> String {
    match lang { Lang::Ru => "🔄 Перевыпустить всех", Lang::En => "🔄 Reissue all" }.to_string()
}
pub fn confirm_regen_all(lang: Lang) -> String {
    match lang {
        Lang::Ru => "🔄 <b>Перевыпустить конфиги всех клиентов?</b>\nФайлы и QR будут перегенерированы, ключи и IP сохранятся — существующие подключения продолжат работать.\n\n🔀 <b>+ сброс маршрутов</b>: дополнительно заменит индивидуальные AllowedIPs клиентов глобальным режимом маршрутизации сервера (нужно после смены режима).",
        Lang::En => "🔄 <b>Reissue configs for all clients?</b>\nFiles and QR codes will be regenerated; keys and IPs are preserved — existing connections keep working.\n\n🔀 <b>+ reset routes</b>: additionally replaces per-client AllowedIPs with the server's global routing mode (needed after a mode change).",
    }.to_string()
}
pub fn btn_regen_all_go(lang: Lang) -> String {
    match lang { Lang::Ru => "✅ Перевыпустить", Lang::En => "✅ Reissue" }.to_string()
}
pub fn btn_regen_all_routes(lang: Lang) -> String {
    match lang { Lang::Ru => "🔀 + сброс маршрутов", Lang::En => "🔀 + reset routes" }.to_string()
}
pub fn regen_all_running(lang: Lang) -> String {
    match lang { Lang::Ru => "⏳ Перевыпускаю всех…", Lang::En => "⏳ Reissuing all…" }.to_string()
}
pub fn regen_all_done(lang: Lang) -> String {
    match lang { Lang::Ru => "✅ Все конфиги перевыпущены.", Lang::En => "✅ All client configs reissued." }.to_string()
}
pub fn regen_all_partial(lang: Lang) -> String {
    match lang {
        Lang::Ru => "⚠️ Завершено, но с ошибками у части клиентов — проверьте логи сервера.",
        Lang::En => "⚠️ Completed, but with errors for some clients — check the server logs.",
    }.to_string()
}
```

`src/bot/menu.rs`:

1. В `clients_list`, перед строкой `rows.push(vec![cb(&i18n::btn_back(lang), "menu")]);`:

```rust
    rows.push(vec![cb(&i18n::btn_regen_all(lang), "regen_all")]);
```

Примечание: тест `clients_list_zero_per_page_no_panic` проверяет ветку `per_page == 0` строгим `assert_eq!(data, vec!["menu"])` — ветку раннего выхода не трогаем (там только «назад»), поэтому тест остаётся валидным.

2. Новая клавиатура после `confirm_recreate`:

```rust
pub fn confirm_regen_all(lang: Lang) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![cb(&i18n::btn_regen_all_go(lang), "regen_all_go")],
        vec![cb(&i18n::btn_regen_all_routes(lang), "regen_all_routes")],
        vec![cb(&i18n::btn_back(lang), "menu")],
    ])
}
```

`src/bot/handlers.rs`:

1. В enum `Action` после `Regen(String),`:

```rust
    RegenAll,
    RegenAllRun(bool), // true = --reset-routes
```

2. В `parse_callback`, в блок точных совпадений (рядом с `"check"`):

```rust
        "regen_all" => Action::RegenAll,
        "regen_all_go" => Action::RegenAllRun(false),
        "regen_all_routes" => Action::RegenAllRun(true),
```

3. Армы после `Action::Regen`:

```rust
        Action::RegenAll => {
            bot.send_message(chat, i18n::confirm_regen_all(lang))
                .reply_markup(menu::confirm_regen_all(lang))
                .parse_mode(ParseMode::Html)
                .await?;
        }
        Action::RegenAllRun(reset_routes) => {
            let waiting = bot.send_message(chat, i18n::regen_all_running(lang)).await.ok();
            match vpn.regen_all(reset_routes).await {
                Ok(true) => {
                    bot.send_message(chat, i18n::regen_all_done(lang))
                        .reply_markup(menu::main_menu(lang))
                        .parse_mode(ParseMode::Html)
                        .await?;
                }
                Ok(false) => {
                    bot.send_message(chat, i18n::regen_all_partial(lang))
                        .reply_markup(menu::main_menu(lang))
                        .parse_mode(ParseMode::Html)
                        .await?;
                }
                Err(e) => {
                    tracing::error!(error = %e, "массовый regen провалился");
                    bot.send_message(chat, i18n::error_text(lang, &e)).await?;
                }
            }
            if let Some(m) = waiting {
                let _ = bot.delete_message(chat, m.id).await;
            }
        }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib`
Expected: все зелёные.

- [ ] **Step 5: Commit**

```bash
git add src/i18n.rs src/bot/menu.rs src/bot/handlers.rs
git commit -m "feat(bot): массовый перевыпуск с подтверждением и опцией --reset-routes"
```

---

### Task 8: документация и финальный прогон

**Files:**
- Modify: `README.md` (раздел с описанием функций бота — рядом с пунктами «Список клиентов», «Проверка»)

**Interfaces:**
- Consumes: всё реализованное в Tasks 1–7.
- Produces: задокументированные фичи; зелёный полный прогон.

- [ ] **Step 1: Update README**

В README.md (раздел сценариев, рядом с пунктом 4 «Список клиентов» и пунктом про проверку) добавить описания:

```markdown
   **Метка срока в списке.** Временные клиенты помечаются в списке меткой
   «⏳ 6д» (дни/часы до истечения); истёкший, но ещё не удалённый cron'ом —
   «⏳ истёк». Бессрочные клиенты показываются без метки. Срок читается из
   `clients_dir/expiry/<имя>` (тот же источник, что и карточка клиента).

   **Перевыпуск конфигов (regen).** Кнопка «🔄 Перевыпустить» в карточке
   клиента перегенерирует `.conf`/QR/URI (ключи и IP сохраняются) и присылает
   свежие файлы. Кнопка «🔄 Перевыпустить всех» внизу списка клиентов — после
   подтверждения запускает `regen` для всех; вариант «🔀 + сброс маршрутов»
   добавляет `--reset-routes` (замена индивидуальных AllowedIPs глобальным
   режимом сервера — нужно после смены режима маршрутизации). Массовый
   перевыпуск выполняется с утроенным таймаутом; при частичных ошибках бот
   сообщает «завершено с ошибками», детали — в логах сервера.

   **Диагностика.** Кнопка «🔬 Диагностика» в главном меню запускает
   `manage diagnose` (публичный IP, состояние модуля, известные операторские
   проблемы) и показывает отчёт; длинный вывод обрезается до лимита Telegram.
```

- [ ] **Step 2: Full verification run**

Run: `cargo test 2>&1 | grep "test result"` и `cargo clippy --all-targets 2>&1 | grep -E "^(warning|error)"`
Expected: все test result `ok`, clippy без вывода.

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs(readme): regen, diagnose и метка срока в списке клиентов"
```
