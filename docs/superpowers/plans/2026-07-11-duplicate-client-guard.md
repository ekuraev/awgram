# Duplicate Client Guard Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prevent silent overwrites when adding a client with an existing name — ask the user to confirm before recreating.

**Architecture:** Check client existence via `vpn.list()` immediately after name validation. If the client exists, show a warning with "Recreate" / "Menu" buttons. On "Recreate", the user re-selects expiry and PSK (same flow as normal add), then `finish_add` runs `vpn.remove(name)` before `vpn.add(...)`.

**Tech Stack:** Rust, teloxide (Telegram bot framework), tokio, tempfile (tests), serial_test.

## Global Constraints

- Name validation regex: `^[A-Za-z0-9_][A-Za-z0-9_-]{0,31}$` (no leading hyphen — prevents CLI flag injection).
- i18n: every user-facing string must exist for both `Lang::Ru` and `Lang::En`.
- HTML parse mode for messages with markup; `html_escape` on all dynamic text injected into HTML.
- Callback data prefix ordering: specific prefixes (e.g. `delyes:`) must be checked before shorter ones they start with (e.g. `del:`). New prefixes must not collide with existing ones.
- Tests use `vpn_with_script(body)` helper in `src/vpn/mod.rs` (stub shell script in a tempdir). Config tests use `#[serial_test::serial]` because `AWG_BOT_TOKEN` env var is process-global.
- No stderr leakage to users: `i18n::error_text` returns a generic localized message, never the raw error.

---

## File Structure

| File | Responsibility | Change |
|---|---|---|
| `src/bot/mod.rs` | Dialogue `State` enum | Add `recreate: bool` to 3 variants |
| `src/vpn/mod.rs` | VPN operations (`add`, `remove`, `list`, `exists`) | Add `exists()` method + tests |
| `src/bot/handlers.rs` | Callback/message handlers, `Action` enum, `parse_callback`, `finish_add` | New `Recreate` action, `AwaitingName` duplicate check, `recreate` propagation, `finish_add` param, tests |
| `src/bot/menu.rs` | Inline keyboards | `confirm_recreate()` keyboard + test |
| `src/i18n.rs` | Localized messages | `client_exists()` message + tests |

---

### Task 1: Add `exists()` to VPN layer

**Files:**
- Modify: `src/vpn/mod.rs` (add method to `impl Vpn`, add tests to `mod tests`)

**Interfaces:**
- Consumes: `self.list()` (returns `Result<Vec<Client>>`), `validate::validate_name` (returns `Result<String, ValidateError>`)
- Produces: `pub async fn exists(&self, name: &str) -> Result<bool>` — returns `Ok(true)` if a client with the given name is in `list --json` output, `Ok(false)` if not, `Err` if the name is invalid or the script fails.

- [ ] **Step 1: Write failing tests**

Add these tests to the `mod tests` block in `src/vpn/mod.rs`, after the existing `list_parses_stub_output` test:

```rust
    #[tokio::test]
    async fn exists_returns_true_for_existing_client() {
        let (_d, vpn) = vpn_with_script(
            "#!/bin/sh\necho '[{\"name\":\"alice\",\"status_code\":\"active\"}]'\n",
        );
        assert_eq!(vpn.exists("alice").await.unwrap(), true);
    }

    #[tokio::test]
    async fn exists_returns_false_for_missing_client() {
        let (_d, vpn) = vpn_with_script(
            "#!/bin/sh\necho '[{\"name\":\"alice\",\"status_code\":\"active\"}]'\n",
        );
        assert_eq!(vpn.exists("bob").await.unwrap(), false);
    }

    #[tokio::test]
    async fn exists_rejects_bad_name() {
        let (_d, vpn) = vpn_with_script("#!/bin/sh\necho '[]'\n");
        assert!(vpn.exists("bad name;rm").await.is_err());
    }

    #[tokio::test]
    async fn exists_propagates_script_failure() {
        let (_d, vpn) = vpn_with_script("#!/bin/sh\nexit 1\n");
        assert!(vpn.exists("alice").await.is_err());
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib vpn::tests::exists 2>&1 | head -20`
Expected: compilation error — `no method named exists` on `Vpn`.

- [ ] **Step 3: Write minimal implementation**

Add this method to `impl Vpn` in `src/vpn/mod.rs`, after the `stats` method (before `add`):

```rust
    /// Проверяет, существует ли клиент с таким именем (через `list --json`).
    /// Авторитетно: отражает реальное состояние WireGuard, а не только файлы на диске.
    pub async fn exists(&self, name: &str) -> Result<bool> {
        let name = validate::validate_name(name)
            .map_err(|e| crate::error::Error::Parse(e.to_string()))?;
        let clients = self.list().await?;
        Ok(clients.iter().any(|c| c.name == name))
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib vpn::tests::exists 2>&1 | tail -10`
Expected: 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/vpn/mod.rs
git commit -m "feat(vpn): exists() — authoritative client existence check via list --json"
```

---

### Task 2: Add `Recreate` action to callback parser

**Files:**
- Modify: `src/bot/handlers.rs` (add variant to `Action` enum, add arm to `parse_callback`, add test assertion)

**Interfaces:**
- Consumes: none
- Produces: `Action::Recreate(String)` variant; `parse_callback("recreate:alice")` returns `Action::Recreate("alice".into())`.

- [ ] **Step 1: Write failing test**

In `src/bot/handlers.rs`, in the `parses_all_actions` test (inside `mod tests`), add this assertion alongside the existing ones (e.g. after the `delyes:alice` assertion):

```rust
        assert_eq!(parse_callback("recreate:alice"), Action::Recreate("alice".into()));
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib bot::handlers::tests::parses_all_actions 2>&1 | head -20`
Expected: compilation error — `Action::Recreate` does not exist.

- [ ] **Step 3: Add the `Recreate` variant**

In `src/bot/handlers.rs`, add `Recreate(String)` to the `Action` enum. Insert it after `ConfirmDelete(String)` and before `Expiry(String)`:

```rust
    ConfirmDelete(String),
    Recreate(String),
    Expiry(String), // "none" | "1d" | ... | "custom"
```

- [ ] **Step 4: Add the `parse_callback` arm**

In `src/bot/handlers.rs`, in `parse_callback`, add a `recreate:` prefix arm. Place it inside the `_ => { ... }` block, after the `del:` arm and before the `exp:` arm:

```rust
            } else if let Some(v) = data.strip_prefix("recreate:") {
                Action::Recreate(v.to_string())
            } else if let Some(v) = data.strip_prefix("exp:") {
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test --lib bot::handlers::tests::parses_all_actions 2>&1 | tail -10`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/bot/handlers.rs
git commit -m "feat(bot): Action::Recreate + parse_callback 'recreate:' prefix"
```

---

### Task 3: Add `confirm_recreate` keyboard

**Files:**
- Modify: `src/bot/menu.rs` (add function, add test)

**Interfaces:**
- Consumes: `cb(text, data)` helper, `i18n::btn_back(lang)`
- Produces: `pub fn confirm_recreate(lang: Lang, name: &str) -> InlineKeyboardMarkup` — two buttons: `recreate:{name}` and `menu`.

- [ ] **Step 1: Write failing test**

In `src/bot/menu.rs`, in `mod tests`, add this test (after `confirm_delete_encodes_name`):

```rust
    #[test]
    fn confirm_recreate_encodes_name() {
        let data = all_callback_data(&confirm_recreate(Lang::Ru, "bob"));
        assert!(data.contains(&"recreate:bob".to_string()));
        assert!(data.contains(&"menu".to_string()));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib bot::menu::tests::confirm_recreate_encodes_name 2>&1 | head -20`
Expected: compilation error — `confirm_recreate` not found.

- [ ] **Step 3: Write implementation**

In `src/bot/menu.rs`, add this function after `confirm_delete` (before `backup_menu`):

```rust
pub fn confirm_recreate(lang: Lang, name: &str) -> InlineKeyboardMarkup {
    let yes_txt = match lang { Lang::Ru => "♻️ Пересоздать", Lang::En => "♻️ Recreate" };
    InlineKeyboardMarkup::new(vec![vec![
        cb(yes_txt, &format!("recreate:{name}")),
        cb(&i18n::btn_back(lang), "menu"),
    ]])
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib bot::menu::tests::confirm_recreate_encodes_name 2>&1 | tail -10`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/bot/menu.rs
git commit -m "feat(menu): confirm_recreate keyboard — ♻️ Recreate / ⬅️ Menu"
```

---

### Task 4: Add `client_exists` i18n message

**Files:**
- Modify: `src/i18n.rs` (add function, add tests)

**Interfaces:**
- Consumes: `html_escape(s: &str) -> String`, `Lang`
- Produces: `pub fn client_exists(lang: Lang, name: &str) -> String` — HTML warning that the client already exists, asking whether to recreate.

- [ ] **Step 1: Write failing tests**

In `src/i18n.rs`, in `mod tests`, add these tests (after `status_label_unknown_code_falls_back_to_raw` or alongside the other message tests):

```rust
    #[test]
    fn client_exists_nonempty_both_langs() {
        for l in [Lang::Ru, Lang::En] {
            let msg = client_exists(l, "alice");
            assert!(!msg.is_empty());
            assert!(msg.contains("alice"));
        }
    }

    #[test]
    fn client_exists_escapes_html() {
        // Имя проходит validate_name (без <>), но html_escape не должен
        // давать двойное экранирование (&amp;amp;).
        let msg = client_exists(Lang::Ru, "alice");
        assert!(!msg.contains("&amp;amp;"));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib i18n::tests::client_exists 2>&1 | head -20`
Expected: compilation error — `client_exists` not found.

- [ ] **Step 3: Write implementation**

In `src/i18n.rs`, add this function in the `// --- add-диалог ---` section, after `done`:

```rust
pub fn client_exists(lang: Lang, name: &str) -> String {
    let n = html_escape(name);
    match lang {
        Lang::Ru => format!("⚠️ Клиент <b>{n}</b> уже существует. Пересоздать? Старый конфиг будет заменён (новые ключи, новый IP)."),
        Lang::En => format!("⚠️ Client <b>{n}</b> already exists. Recreate? The old config will be replaced (new keys, new IP)."),
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib i18n::tests::client_exists 2>&1 | tail -10`
Expected: 2 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/i18n.rs
git commit -m "feat(i18n): client_exists — localized duplicate warning"
```

---

### Task 5: Add `recreate: bool` to `State` variants

**Files:**
- Modify: `src/bot/mod.rs` (add field to 3 enum variants)

**Interfaces:**
- Consumes: none
- Produces: `State::AwaitingExpiry { name: String, recreate: bool }`, `State::AwaitingCustomExpiry { name: String, recreate: bool }`, `State::AwaitingPsk { name: String, expires: Option<String>, recreate: bool }`.

**Note:** This task will break compilation of `handlers.rs` (all match arms that construct/destructure these states). Task 6 fixes them. Do not run `cargo build` between this task and Task 6 — only run tests after Task 6 is complete.

- [ ] **Step 1: Modify the `State` enum**

In `src/bot/mod.rs`, replace the entire `State` enum:

```rust
#[derive(Clone, Default)]
pub enum State {
    #[default]
    Idle,
    AwaitingName,
    AwaitingExpiry { name: String, recreate: bool },
    AwaitingCustomExpiry { name: String, recreate: bool },
    AwaitingPsk { name: String, expires: Option<String>, recreate: bool },
}
```

- [ ] **Step 2: Commit (without building — Task 6 fixes the breakage)**

```bash
git add src/bot/mod.rs
git commit -m "feat(bot): add recreate: bool to add-flow State variants

Compilation will break in handlers.rs — fixed in the next commit."
```

---

### Task 6: Wire `recreate` through handlers and `finish_add`

**Files:**
- Modify: `src/bot/handlers.rs` (multiple branches in `message_handler` and `callback_handler`, `finish_add` signature, contract test)

**Interfaces:**
- Consumes: `vpn.exists()` (Task 1), `Action::Recreate` (Task 2), `menu::confirm_recreate` (Task 3), `i18n::client_exists` (Task 4), `State` with `recreate` field (Task 5)
- Produces: fully functional duplicate guard in the add flow.

- [ ] **Step 1: Update `finish_add` signature and body**

In `src/bot/handlers.rs`, replace the `finish_add` function signature and add the remove-before-add logic. Replace the existing function (lines ~211-233):

```rust
async fn finish_add(bot: &Bot, chat: ChatId, vpn: &Vpn, lang: Lang, name: &str, expires: Option<&str>, psk: bool, recreate: bool) {
    let waiting = bot.send_message(chat, i18n::creating(lang)).await.ok();
    if recreate {
        // Удаляем старого клиента перед созданием нового. Если remove упадёт —
        // не создаём нового, показываем ошибку; старый клиент остаётся.
        if let Err(e) = vpn.remove(name).await {
            tracing::error!(error = %e, "remove перед recreate провалился");
            if let Some(m) = waiting {
                let _ = bot.delete_message(chat, m.id).await;
            }
            let _ = bot.send_message(chat, i18n::error_text(lang, &e)).await;
            return;
        }
    }
    match vpn.add(name, expires, psk).await {
        Ok(res) => {
            if let Err(e) = render::send_client_files(bot, chat, lang, &res).await {
                tracing::error!(error = %e, "не удалось отправить файлы клиента");
                let _ = bot.send_message(chat, i18n::error_text(lang, &e)).await;
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "add провалился");
            let _ = bot.send_message(chat, i18n::error_text(lang, &e)).await;
        }
    }
    if let Some(m) = waiting {
        let _ = bot.delete_message(chat, m.id).await;
    }
    let _ = bot
        .send_message(chat, i18n::done(lang))
        .reply_markup(menu::main_menu(lang))
        .parse_mode(ParseMode::Html)
        .await;
}
```

- [ ] **Step 2: Rename `_vpn` to `vpn` in `message_handler`**

In `src/bot/handlers.rs`, the `message_handler` function signature uses `_vpn: Arc<Vpn>`. Rename it to `vpn` since we now use it. Replace:

```rust
    _vpn: Arc<Vpn>,
```

with:

```rust
    vpn: Arc<Vpn>,
```

- [ ] **Step 3: Update `State::AwaitingName` branch in `message_handler`**

Replace the entire `State::AwaitingName => { ... }` arm in `message_handler` (the one that currently does `validate_name` and transitions to `AwaitingExpiry { name: valid }`):

```rust
        State::AwaitingName => {
            let name = msg.text().unwrap_or_default().to_string();
            match crate::vpn::validate::validate_name(&name) {
                Ok(valid) => {
                    match vpn.exists(&valid).await {
                        Ok(false) => {
                            let confirm_line = match lang {
                                Lang::Ru => format!("Клиент: {valid}"),
                                Lang::En => format!("Client: {valid}"),
                            };
                            bot.send_message(msg.chat.id, format!("{confirm_line}\n{}", i18n::ask_expiry(lang)))
                                .reply_markup(menu::expiry_menu(lang))
                                .await?;
                            dialogue.update(State::AwaitingExpiry { name: valid, recreate: false }).await?;
                        }
                        Ok(true) => {
                            bot.send_message(msg.chat.id, i18n::client_exists(lang, &valid))
                                .reply_markup(menu::confirm_recreate(lang, &valid))
                                .parse_mode(ParseMode::Html)
                                .await?;
                            dialogue.update(State::Idle).await?;
                        }
                        Err(e) => {
                            // list --json упал — не блокируем создание (fail-open).
                            tracing::warn!(error = %e, "exists check failed, proceeding without duplicate guard");
                            bot.send_message(msg.chat.id, i18n::ask_expiry(lang))
                                .reply_markup(menu::expiry_menu(lang))
                                .await?;
                            dialogue.update(State::AwaitingExpiry { name: valid, recreate: false }).await?;
                        }
                    }
                }
                Err(_e) => {
                    bot.send_message(msg.chat.id, i18n::bad_name(lang)).await?;
                }
            }
        }
```

- [ ] **Step 4: Update `State::AwaitingCustomExpiry` branch in `message_handler`**

Replace the `State::AwaitingCustomExpiry { name } => { ... }` arm (destructure `recreate`, propagate into `AwaitingPsk`):

```rust
        State::AwaitingCustomExpiry { name, recreate } => {
            let raw = msg.text().unwrap_or_default().to_string();
            match crate::vpn::validate::validate_expiry(&raw) {
                Ok(exp) => {
                    bot.send_message(msg.chat.id, i18n::psk_step(lang, settings.psk_default()))
                        .reply_markup(menu::psk_step(lang, settings.psk_default()))
                        .parse_mode(ParseMode::Html)
                        .await?;
                    dialogue.update(State::AwaitingPsk { name, expires: Some(exp), recreate }).await?;
                }
                Err(_e) => {
                    bot.send_message(msg.chat.id, i18n::bad_expiry(lang)).await?;
                }
            }
        }
```

- [ ] **Step 5: Add `Action::Recreate(name)` handler arm**

In `callback_handler`, add a new match arm for `Action::Recreate`. Place it after `Action::Add` and before `Action::Expiry(kind)`:

```rust
        Action::Recreate(name) => {
            bot.send_message(chat, i18n::ask_expiry(lang))
                .reply_markup(menu::expiry_menu(lang))
                .await?;
            dialogue.update(State::AwaitingExpiry { name, recreate: true }).await?;
        }
```

- [ ] **Step 6: Update `Action::Expiry(kind)` arm**

Replace the `Action::Expiry(kind) => { ... }` arm to destructure `recreate` and propagate it:

```rust
        Action::Expiry(kind) => {
            let (name, recreate) = match dialogue.get().await?.unwrap_or_default() {
                State::AwaitingExpiry { name, recreate } => (name, recreate),
                _ => {
                    bot.send_message(chat, session_expired_text(lang))
                        .reply_markup(menu::main_menu(lang))
                        .parse_mode(ParseMode::Html)
                        .await?;
                    return Ok(());
                }
            };
            if kind == "custom" {
                bot.send_message(chat, i18n::ask_custom_expiry(lang)).await?;
                dialogue.update(State::AwaitingCustomExpiry { name, recreate }).await?;
            } else {
                let expires = if kind == "none" { None } else { Some(kind.clone()) };
                bot.send_message(chat, i18n::psk_step(lang, settings.psk_default()))
                    .reply_markup(menu::psk_step(lang, settings.psk_default()))
                    .parse_mode(ParseMode::Html)
                    .await?;
                dialogue.update(State::AwaitingPsk { name, expires, recreate }).await?;
            }
        }
```

- [ ] **Step 7: Update `Action::AddPsk(psk)` arm**

Replace the `Action::AddPsk(psk) => { ... }` arm to destructure `recreate` and pass it to `finish_add`:

```rust
        Action::AddPsk(psk) => {
            let (name, expires, recreate) = match dialogue.get().await?.unwrap_or_default() {
                State::AwaitingPsk { name, expires, recreate } => (name, expires, recreate),
                _ => {
                    bot.send_message(chat, session_expired_text(lang))
                        .reply_markup(menu::main_menu(lang))
                        .parse_mode(ParseMode::Html)
                        .await?;
                    return Ok(());
                }
            };
            finish_add(&bot, chat, &vpn, lang, &name, expires.as_deref(), psk, recreate).await;
            dialogue.exit().await?;
        }
```

- [ ] **Step 8: Add `confirm_recreate` to the contract test**

In `src/bot/handlers.rs`, in the `all_menu_callback_data_parse_to_known_actions` test, add `confirm_recreate` to the `keyboards` vector. Add this line after `menu::confirm_delete(Lang::Ru, "bob"),`:

```rust
            menu::confirm_recreate(Lang::Ru, "alice"),
```

- [ ] **Step 9: Build and run all tests**

Run: `cargo test 2>&1 | tail -30`
Expected: all tests pass (no compilation errors, no test failures).

- [ ] **Step 10: Commit**

```bash
git add src/bot/handlers.rs
git commit -m "feat(bot): duplicate client guard — check exists, confirm recreate, remove→add"
```

---

### Task 7: Update README

**Files:**
- Modify: `README.md` (document the duplicate guard in the add flow section)

**Interfaces:**
- Consumes: none
- Produces: documentation of the new behavior.

- [ ] **Step 1: Update the add flow description**

In `README.md`, find the "Добавление клиента" item (around line 363). After the existing description of the add flow, add a note about the duplicate guard. Find this text:

```
3. **Добавление клиента.** `➕ Добавить` → ввести имя клиента → выбрать срок →
   бот присылает `.conf`-файл, QR-код (если он сгенерирован скриптом) и
   текстовое сообщение со ссылкой-URI для импорта. `add` не печатает JSON —
   бот запускает `manage_amneziawg.sh add <имя> [--expires=<срок>]` и затем
   читает созданные файлы из `clients_dir`. Если бот не находит `.conf` —
   проверьте, что `clients_dir` в конфиге указывает на каталог, куда скрипт
   кладёт файлы (по умолчанию `/root/awg`).
```

Replace it with:

```
3. **Добавление клиента.** `➕ Добавить` → ввести имя клиента → выбрать срок →
   бот присылает `.conf`-файл, QR-код (если он сгенерирован скриптом) и
   текстовое сообщение со ссылкой-URI для импорта. `add` не печатает JSON —
   бот запускает `manage_amneziawg.sh add <имя> [--expires=<срок>]` и затем
   читает созданные файлы из `clients_dir`. Если бот не находит `.conf` —
   проверьте, что `clients_dir` в конфиге указывает на каталог, куда скрипт
   кладёт файлы (по умолчанию `/root/awg`).

   **Защита от дубликатов.** Если клиент с введённым именем уже существует,
   бот не перезаписывает его молча — вместо этого показывается предупреждение
   с кнопками «♻️ Пересоздать» / «⬅️ В меню». При выборе «Пересоздать» бот
   спрашивает срок и PSK заново, затем удаляет старого клиента и создаёт
   нового (новые ключи, новый IP). Существование проверяется через
   `list --json` — если этот вызов падает, создание не блокируется
   (fail-open, с `warn` в логе).
```

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs(readme): document duplicate client guard in add flow"
```

---

### Task 8: Final verification

**Files:** none (verification only)

- [ ] **Step 1: Run the full test suite**

Run: `cargo test 2>&1 | tail -40`
Expected: all tests pass, zero failures.

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --all-targets 2>&1 | tail -20`
Expected: no warnings or errors.

- [ ] **Step 3: Verify no untracked files left behind**

Run: `git status`
Expected: clean working tree (only `.serena/` untracked if it was there before).

- [ ] **Step 4: Review the commit log**

Run: `git log --oneline -8`
Expected: a clean sequence of commits for the duplicate client guard feature.
