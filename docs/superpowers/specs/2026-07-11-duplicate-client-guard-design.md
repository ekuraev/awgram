# Duplicate Client Guard — Design

**Date:** 2026-07-11
**Status:** Approved (brainstormed)

## Problem

Currently `add` silently overwrites an existing client's configuration when a
user enters a name that already exists. `vpn.add()` (`src/vpn/mod.rs:43`) runs
`manage_amneziawg.sh add <name>` with no duplicate check — the script
regenerates `.conf`/`.png`/`.vpnuri` on disk and rewrites the WireGuard peer.
The user gets a fresh config with no warning that the old one is gone.

This is dangerous: the admin loses the old client's keys, IP assignment, and
expiry without consent.

## Requirements

1. When a user enters a name that already exists, the bot must **not** silently
   overwrite it.
2. The bot must show a warning ("client already exists") and offer a choice:
   recreate or cancel.
3. "Recreate" must fully replace the old client: `remove` the old peer, then
   `add` a new one (new keys, new IP).
4. After choosing "Recreate", the user re-selects expiry and PSK (same flow as
   a normal add) — no defaults assumed.
5. The check must be authoritative: reflect WireGuard's actual state, not just
   files on disk.
6. If the existence check itself fails (script error), the bot must not block
   legitimate creation (fail-open with a `warn` log).

## Decisions (from brainstorming)

| Decision | Choice | Rationale |
|---|---|---|
| When to check | Immediately after name input, before expiry menu | User learns of duplicate before investing in expiry/PSK selection |
| Source of truth | `vpn.list()` (`list --json`) | Authoritative — reflects WireGuard state, not just files |
| Recreate action | `vpn.remove(name)` then `vpn.add(...)` | Clean replacement; avoids stale peers if script doesn't auto-remove |
| Recreate flow | Re-ask expiry and PSK | Flexible — user can change params on recreate |
| Where remove happens | In `finish_add`, after user has committed (PSK chosen) | If user abandons mid-flow, old client is untouched |
| Approach for state | `recreate: bool` flag in existing state variants | Atomic remove+add; minimal diff; no state-variant duplication |

## Architecture

### State machine changes (`src/bot/mod.rs`)

Add `recreate: bool` to the three add-flow state variants:

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

The flag propagates through the flow and is consumed in `finish_add`. If the
user abandons the flow (presses "Menu", session expires), `recreate` is never
acted upon — the old client stays intact.

### Flow

```
AwaitingName (text input)
  │
  ├─ validate_name(name) → Err → bad_name (unchanged)
  │
  └─ validate_name(name) → Ok(valid)
      │
      ├─ vpn.exists(valid) → Ok(false)
      │   → "Client: {valid}\n{ask_expiry}" + expiry_menu
      │   → State::AwaitingExpiry { name, recreate: false }
      │
      ├─ vpn.exists(valid) → Ok(true)
      │   → i18n::client_exists(lang, valid) + confirm_recreate menu
      │   → State::Idle  (button carries name in callback_data)
      │
      └─ vpn.exists(valid) → Err(e)
          → warn log, proceed without guard
          → ask_expiry + expiry_menu
          → State::AwaitingExpiry { name, recreate: false }

Action::Recreate(name):
  → ask_expiry + expiry_menu
  → State::AwaitingExpiry { name, recreate: true }

Action::Expiry(kind)  — extracts (name, recreate) from state
Action::AddPsk(psk)   — extracts (name, expires, recreate) from state
  → finish_add(..., recreate)
```

### VPN layer (`src/vpn/mod.rs`)

New method — existence check via `list --json`:

```rust
/// Checks whether a client with the given name exists (via `list --json`).
/// Authoritative: reflects WireGuard's actual state, not just files on disk.
pub async fn exists(&self, name: &str) -> Result<bool> {
    let name = validate::validate_name(name)
        .map_err(|e| crate::error::Error::Parse(e.to_string()))?;
    let clients = self.list().await?;
    Ok(clients.iter().any(|c| c.name == name))
}
```

`finish_add` gains a `recreate: bool` parameter. When true, it calls
`vpn.remove(name)` before `vpn.add(...)`. If `remove` fails, `add` is not
attempted — the error is shown to the user, and the old client remains (as
much as the script's `remove` left it).

### Callback data (`src/bot/handlers.rs`)

New action and prefix — `recreate:<name>`. Placed in `parse_callback` before
the general `lang:` block (following the established ordering convention for
prefix-specific callbacks, though `recreate:` has no collision risk with
existing prefixes).

```rust
Recreate(String),

// in parse_callback:
} else if let Some(v) = data.strip_prefix("recreate:") {
    Action::Recreate(v.to_string())
}
```

### Menu (`src/bot/menu.rs`)

New keyboard shown when a duplicate is detected:

```rust
pub fn confirm_recreate(lang: Lang, name: &str) -> InlineKeyboardMarkup {
    let yes_txt = match lang { Lang::Ru => "♻️ Пересоздать", Lang::En => "♻️ Recreate" };
    InlineKeyboardMarkup::new(vec![vec![
        cb(yes_txt, &format!("recreate:{name}")),
        cb(&i18n::btn_back(lang), "menu"),
    ]])
}
```

### i18n (`src/i18n.rs`)

New message — duplicate warning:

```rust
pub fn client_exists(lang: Lang, name: &str) -> String {
    let n = html_escape(name);
    match lang {
        Lang::Ru => format!("⚠️ Клиент <b>{n}</b> уже существует. Пересоздать? Старый конфиг будет заменён (новые ключи, новый IP)."),
        Lang::En => format!("⚠️ Client <b>{n}</b> already exists. Recreate? The old config will be replaced (new keys, new IP)."),
    }
}
```

No separate "recreated" success message — `i18n::done()` is reused, since the
result (new `.conf`/QR/URI) is already delivered to the user.

### Handlers (`src/bot/handlers.rs`)

- `message_handler`, `State::AwaitingName` branch: call `vpn.exists()` after
  `validate_name` succeeds; branch on Ok(false)/Ok(true)/Err. Rename `_vpn`
  → `vpn` in the function signature (the parameter was previously unused).
- `State::AwaitingCustomExpiry` branch: destructure `recreate` and pass it
  into `AwaitingPsk`.
- `Action::Recreate(name)`: new arm — transitions to
  `AwaitingExpiry { name, recreate: true }`.
- `Action::Expiry(kind)`: destructure `(name, recreate)` from
  `AwaitingExpiry`; propagate `recreate` into `AwaitingCustomExpiry` /
  `AwaitingPsk`.
- `Action::AddPsk(psk)`: destructure `(name, expires, recreate)` from
  `AwaitingPsk`; pass `recreate` to `finish_add`.
- `finish_add`: new `recreate: bool` parameter; if true, `vpn.remove(name)`
  before `vpn.add(...)`.

## Error handling

- **`vpn.exists()` fails (script error/timeout):** fail-open — log `warn`,
  proceed to normal add flow without the duplicate guard. Rationale: `list`
  may transiently fail; blocking `add` would worsen the situation. The
  duplicate guard is a safety net, not a hard gate.
- **`vpn.remove()` fails during recreate:** `add` is not attempted. The error
  is shown to the user via `i18n::error_text`. The old client may be in a
  partially-removed state — that's the script's responsibility, not the bot's.
- **Session expiry mid-flow:** if the user abandons after choosing "Recreate"
  but before `finish_add`, `recreate` is never acted on — `State::Idle` on
  next interaction, old client untouched.

## Testing

### VPN layer (`src/vpn/mod.rs`)

- `exists_returns_true_for_existing_client` — stub returns list with "alice",
  `exists("alice")` → `true`.
- `exists_returns_false_for_missing_client` — stub returns list with "alice",
  `exists("bob")` → `false`.
- `exists_rejects_bad_name` — `exists("bad name;rm")` → `Err`.
- `exists_propagates_script_failure` — stub exits 1, `exists("alice")` →
  `Err`.

### Parser (`src/bot/handlers.rs`)

- `parses_all_actions` — add
  `assert_eq!(parse_callback("recreate:alice"), Action::Recreate("alice".into()))`.
- `all_menu_callback_data_parse_to_known_actions` — add
  `menu::confirm_recreate(Lang::Ru, "alice")` to the keyboards vector.

### Menu (`src/bot/menu.rs`)

- `confirm_recreate_encodes_name` — callback data contains `"recreate:bob"`
  and `"menu"`.

### i18n (`src/i18n.rs`)

- `client_exists_nonempty_both_langs` — message non-empty for both langs,
  contains the name.
- `client_exists_escapes_html` — no double-escaping (`&amp;amp;` absent).

### Handler flow

Not unit-tested with a live `Dialogue` (teloxide dialogue storage is heavy in
unit tests). Relies on:
- Rust's exhaustive match — the compiler catches any state variant missing
  `recreate`.
- Contract tests (`all_menu_callback_data_parse_to_known_actions`) — catch
  keyboard/parser mismatches.
- Full duplicate flow validated manually/integration.

## TDD order

1. `exists()` tests (red) → `exists()` impl (green).
2. `parse_callback` + `confirm_recreate` menu tests (red) → impl (green).
3. `client_exists` i18n tests (red) → impl (green).
4. State + handler changes (compiler-driven — exhaustive match guides
   implementation).
5. `finish_add` `recreate` parameter + remove-before-add.

## Files to modify

| File | Change |
|---|---|
| `src/bot/mod.rs` | Add `recreate: bool` to 3 state variants |
| `src/vpn/mod.rs` | Add `exists()` method |
| `src/bot/handlers.rs` | `Action::Recreate`, `parse_callback` arm, `AwaitingName`/`AwaitingCustomExpiry`/`Expiry`/`AddPsk` branches, `finish_add` signature, rename `_vpn`→`vpn`, tests |
| `src/bot/menu.rs` | `confirm_recreate()` keyboard + test |
| `src/i18n.rs` | `client_exists()` message + tests |
