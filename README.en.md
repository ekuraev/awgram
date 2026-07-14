# awgram

[🇷🇺 Русский](README.md) · 🇬🇧 English

[![CI](https://github.com/ekuraev/awgram/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/ekuraev/awgram/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/ekuraev/awgram)](https://github.com/ekuraev/awgram/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A Rust Telegram bot for managing [AmneziaWG](https://amnezia.org/) clients
straight from your phone: add/remove a client, view the list and traffic —
no SSH required.

## What it is and how it works

`awgram` is a single binary (`tokio` + `teloxide 0.17`, long polling, no
webhook) that lives on the same VPS as the VPN itself. The bot doesn't touch
the AmneziaWG configuration directly — it invokes the standard
`manage_amneziawg.sh` script (the same one you use to manage the VPN by
hand) with the `add` / `remove` / `list` / `stats` subcommands and the
`--json` flag, and renders the result as an inline menu in Telegram.

Architecturally, this is two independent layers:

- `vpn/` — invokes the script via `tokio::process::Command` (no shell, no
  string-formatted commands) and parses its JSON output into the types from
  `vpn/model.rs`. It knows nothing about Telegram.
- `bot/` — builds menus, handles updates, and renders replies. It knows
  nothing about the script's internals.

Access to the bot is restricted to the `admin_ids` list from the config: any
update from a user not on that list gets "⛔ Access denied" and no VPN
operation is performed. The bot token and the contents of clients' `.conf`
files/QR codes are never written to the logs.

## AmneziaWG installer compatibility

The bot is a layer on top of the `manage_amneziawg.sh` script from
[bivlked/amneziawg-installer](https://github.com/bivlked/amneziawg-installer)
and depends directly on its interface.

- **Supported installer version:
  [v5.19.2](https://github.com/bivlked/amneziawg-installer/releases/tag/v5.19.2)**
  (tested against it; newer versions are at your own risk until this line is updated).
- Subcommands used: `add`, `remove`, `list`, `stats`, `regen`, `backup`,
  `restore`, `check`, `diagnose` — all with the `--json` flag.
- Interface changes in newer installer versions may break bot features — when
  upgrading the installer, check this section (the currently supported version
  is stated here and in awgram release notes).

## Building

You need a stable Rust toolchain (edition 2021) and `cargo`.

```bash
cargo build --release
```

The binary will appear at `target/release/awgram`.

TLS is implemented with **rustls** (no OpenSSL), so the binary has no system
dependency on `libssl`.

### Static Linux binaries (for VPS deployment)

A ready-to-use portable binary can be built with a single command (requires
Docker):

```bash
./scripts/build-musl.sh          # amd64 (default)
./scripts/build-musl.sh arm64    # aarch64
./scripts/build-musl.sh all      # both architectures
```

The script builds a fully static ET_EXEC binary inside a Docker container
(`x86_64-` / `aarch64-unknown-linux-musl`; `crt-static` +
`relocation-model=static` are set in `.cargo/config.toml`, rustls) and places
it at `dist/awgram-linux-amd64` / `dist/awgram-linux-arm64`. This binary
depends on neither glibc, nor `libssl`, nor the musl loader — it runs on any
Linux host of the matching architecture (Ubuntu/Debian/Alpine). Verify with:
`ldd dist/awgram-linux-amd64` → `not a dynamic executable`.

A GitHub Release (on a `v*` tag) builds both binaries via
[cross](https://github.com/cross-rs/cross) and attaches them together with
`sha256` checksums.

Copying it to the server:

```bash
scp dist/awgram-linux-amd64 root@SERVER:/usr/local/bin/awgram
ssh root@SERVER chmod +x /usr/local/bin/awgram
```

> Building a non-native architecture runs under qemu emulation (on Apple
> Silicon: arm64 is native, amd64 is emulated) — slower, but the result is
> correct. The cargo cache is kept in per-arch `awgram-cargo-registry-<arch>`
> docker volumes, so subsequent builds are faster.

## Installation

This assumes `manage_amneziawg.sh` is already installed and working (see
[bivlked/amneziawg-installer](https://github.com/bivlked/amneziawg-installer)
and the "Compatibility" section above), usually at
`/root/awg/manage_amneziawg.sh`.

### Quick install (recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/ekuraev/awgram/main/install.sh | bash
```

The interactive installer asks for the language (RU/EN), mode (root or
hardened), token and admin IDs, downloads the binary for your architecture
(amd64/arm64) from the latest release with sha256 verification and starts
the service. Fully automated install — via flags:

```bash
curl -fsSL https://raw.githubusercontent.com/ekuraev/awgram/main/install.sh \
  | bash -s -- install --lang en --mode root --token 'TOKEN' --admins 111111111 --yes
```

You can skip the `--token` flag (so the token never lands in `argv` or shell
history) and set the `AWGRAM_TOKEN` environment variable instead — use
`export`, since a plain `AWGRAM_TOKEN=... curl ... | bash` would not pass the
variable to the second command in the pipeline:

```bash
export AWGRAM_TOKEN='TOKEN'
curl -fsSL https://raw.githubusercontent.com/ekuraev/awgram/main/install.sh \
  | bash -s -- install --lang en --mode root --admins 111111111 --yes
```

After installation the `awgram-setup` command is available:
`update` (upgrade to the latest release), `config` (token/admins/paths),
`status`, `uninstall`, `help`.

### Manual installation

1. Copy the binary:

   ```bash
   sudo cp target/release/awgram /usr/local/bin/awgram
   sudo chmod 755 /usr/local/bin/awgram
   ```

2. Create the config directory and copy the example:

   ```bash
   sudo mkdir -p /etc/awgram
   sudo cp deploy/config.example.toml /etc/awgram/config.toml
   sudo chmod 640 /etc/awgram/config.toml
   ```

   Edit `/etc/awgram/config.toml`: fill in `admin_ids` (see the "Telegram
   setup" section below). The `bot_token` field in this file can be left
   empty — it's better to keep the token separate (next step).

3. Create an environment file with the token and set permissions to `600`
   so that no one but the owner can read it:

   ```bash
   sudo bash -c 'echo "AWGRAM_TOKEN=<your_botfather_token>" > /etc/awgram/env'
   sudo chmod 600 /etc/awgram/env
   ```

   The `awgram.service` unit picks up this file via
   `EnvironmentFile=-/etc/awgram/env` (the leading `-` means "don't fail if
   the file is missing" — but without a token the bot will still refuse to
   start, see below).

4. Install the systemd unit:

   ```bash
   sudo cp deploy/awgram.service /etc/systemd/system/awgram.service
   sudo systemctl daemon-reload
   ```

The token can be provided either via `AWGRAM_TOKEN` in `/etc/awgram/env`
(higher priority), or directly in `config.toml` as `bot_token = "..."`. If
neither is set, the bot logs an error on startup and exits with code 1.

## Telegram setup

1. Open [@BotFather](https://t.me/BotFather) in Telegram, run `/newbot`,
   follow the instructions — you'll get a token like
   `123456789:AAExampleTokenValue`.
2. Find your numeric Telegram user ID via
   [@userinfobot](https://t.me/userinfobot) — just send it `/start` and it
   will reply with your ID.
3. Put this ID into `admin_ids` in `config.toml` (you can list several
   admins, comma-separated: `admin_ids = [111111111, 222222222]`). Only
   users on this list can use the bot; every other update is rejected with
   the message "⛔ Access denied" and a `warn` log entry.

## Privilege modes

The bot runs `manage_amneziawg.sh`, which normally needs root privileges to
work with the WireGuard/AmneziaWG interface and files under `/root/awg`.
There are two supported ways to set this up.

### Simple mode (service as root)

- Leave the `awgram.service` unit as is (no `User=`) — the process runs as
  `root`, as the unit's comment says.
- In `config.toml`: `sudo_prefix = ""`.
- Pros: minimal setup. Cons: compromising the bot (e.g. via a bug in
  Telegram update parsing) grants full root on the server running the VPN.

### Hardened mode (dedicated user + sudoers)

A safer option — the bot process runs as an unprivileged user, and only
invoking that specific script is allowed via `sudo` without a password.

1. Create a system user with no home directory and no shell:

   ```bash
   sudo useradd --system --no-create-home --shell /usr/sbin/nologin awgram
   ```

2. Allow this user to run **only** `manage_amneziawg.sh` as root without a
   password prompt. Create the file `/etc/sudoers.d/awgram` (via
   `sudo visudo -f /etc/sudoers.d/awgram`) with strictly one line:

   ```
   awgram ALL=(root) NOPASSWD: /root/awg/manage_amneziawg.sh
   ```

   Verify the syntax (`visudo` does this automatically on save) and the
   file permissions (`0440`).

3. In `config.toml` set `sudo_prefix = "sudo"` — the bot will then run
   `sudo /root/awg/manage_amneziawg.sh <subcommand> ...` instead of calling
   the script directly.

4. In `/etc/systemd/system/awgram.service` add the line `User=awgram` to
   the `[Service]` section (after `Type=simple`).

5. **Important — access to config.toml and `state_file`.** `config.toml`
   defaults to `root:root 640` — the `awgram` user cannot read it, so the
   service will fail to start. Give the group ownership to `awgram`, keep
   the `640` permissions:

   ```bash
   sudo chown root:awgram /etc/awgram/config.toml
   ```

   By default `state_file` points at `/etc/awgram/state.json`, and the
   `/etc/awgram` directory isn't writable by `awgram` — the bot won't be
   able to persist its state. Create a separate directory owned by `awgram`
   and point `config.toml` at it:

   ```bash
   sudo install -d -o awgram -g awgram -m 750 /var/lib/awgram
   ```

   and in `config.toml`: `state_file = "/var/lib/awgram/state.json"`.

6. **Important — read access to files.** The script places clients' `.conf`
   files and QR codes in `clients_dir` (default `/root/awg`), and the bot
   reads them directly (without sudo) from disk to send them to Telegram via
   `send_document`/`send_photo`. The `/root/awg` directory is inherently
   inaccessible to the `awgram` user (it's inside `/root`). Options to fix
   this:
   - move/duplicate the script's output to a directory readable by the
     `awgram` user (change `clients_dir` in both `manage_amneziawg.sh` and
     `config.toml` to something like `/var/lib/awgram/clients` with
     permissions that let the `awgram` group/user read the files);
   - or grant the `awgram` user a targeted read ACL on that directory
     (`setfacl -R -m u:awgram:rx /root/awg`).

     Without one of these measures, the "Add client" and "Config" steps
     will fail with a file-read error, even if the sudo call to the script
     itself succeeded.

7. Apply the changes:

   ```bash
   sudo systemctl daemon-reload
   ```

## Running

```bash
sudo systemctl enable --now awgram
```

Check status and follow logs in real time:

```bash
sudo systemctl status awgram
sudo journalctl -u awgram -f
```

The log level is controlled by the standard `RUST_LOG` variable
(`tracing-subscriber` with `EnvFilter`, defaults to `info`) — you can add it
to `/etc/awgram/env`, e.g. `RUST_LOG=debug`.

## Language

The bot is bilingual (Russian/English); the choice is per-admin (each
administrator in `admin_ids` has their own language, independent of the
others).

- **First run.** On the first `/start` from an administrator whose language
  hasn't been saved yet, the bot shows a language-selection screen
  ("🌐 Выберите язык / Choose language:") with two buttons — 🇷🇺 Русский and
  🇬🇧 English. After the choice, the whole menu and all subsequent messages
  are rendered in that language.
- **Changing the language.** ⚙️ Settings → the top row has the 🇷🇺 Русский /
  🇬🇧 English buttons — tapping one immediately switches the interface
  language and refreshes the settings screen in the new language.
- The language is stored keyed by the administrator's numeric Telegram user
  ID (`langs` in `state_file`, see below), so different admins on the same
  bot can have different languages at the same time.
- If no language has been chosen yet (the `langs` map has no entry for that
  `uid`), internal texts default to rendering in Russian (`Lang::Ru` is the
  default), but in that case the chat is shown the language-selection screen
  itself first and foremost.

## PresharedKey (PSK)

AmneziaWG/WireGuard supports an additional symmetric key (PresharedKey) on
top of the regular key pair — it improves resistance to future quantum
attacks on Diffie-Hellman. The bot can enable it when creating a client via
the `--psk` flag, which is passed to `manage_amneziawg.sh`.

- **Global default.** ⚙️ Settings → the "PSK: on ✅ / PSK: off ⬜" button
  toggles the default value that will be offered the next time a client is
  added. The value is stored in `state_file` (the `psk_default` field) and
  is shared across all administrators of the bot (unlike the language
  setting).
- **Override when adding.** In the ➕ Add dialog, after choosing the expiry
  period, the bot shows a "PresharedKey (default: on/off). Create client:"
  step with two buttons — "🔐 With PSK" and "🔓 Without PSK". The button
  matching the current global default comes first. The choice at this step
  applies only to the client being created and does not change the global
  setting.
- If "With PSK" is chosen, the bot calls
  `manage_amneziawg.sh add <name> [--expires=<period>] --psk`; in that case
  the generated client `.conf` file's `[Interface]`/`[Peer]` section
  contains a `PresharedKey = ...` line (this can be checked by opening the
  file — the secret itself is never printed to the chat separately).

## Backup / Restore

The 💾 Backup section in the main menu manages backups of the AmneziaWG
state, which `manage_amneziawg.sh` itself creates and stores (via the
`backup`/`restore` subcommands) in the `clients_dir/backups/` directory on
the server — the bot only invokes the script and reads the list of files
from there.

- **Create a backup.** 💾 Backup → "➕ Create backup" — the bot runs
  `manage_amneziawg.sh backup`, then picks the most recently modified
  `*.tar.gz` from `clients_dir/backups/` and sends a confirmation with the
  file name (`✅ Backup created: <name>.tar.gz`) along with a card for that
  backup (buttons "📥 Download" / "♻️ Restore").
- **List of backups.** 💾 Backup → "📃 List backups" — a list of archives
  from `clients_dir/backups/`, sorted by modification time (newest first);
  if the directory is empty — "No backups yet.". Each row is a button with
  the archive's name that leads to that backup's card, identified by its
  index in this list (0 = newest).
- **Download.** On a backup's card, the "📥 Download" button sends the
  `.tar.gz` file as a document straight to the chat (`send_document`), with
  no parsing of its contents — you can save it locally as an off-server
  backup.
- **Restore.** The "♻️ Restore" button on a backup's card asks for
  confirmation ("♻️ Restore from `<name>.tar.gz`? The current state will be
  replaced.") with "✅ Yes" / "⬅️ To menu" buttons — the operation is
  irreversible (it overwrites the current AmneziaWG client state), so it
  never runs without confirmation. After confirming, the bot calls
  `manage_amneziawg.sh restore <path-to-archive>` and sends "✅ Restore
  complete.". The backup to restore is always selected by its index in the
  current `list_backups()`, never by an arbitrary path — this rules out
  path traversal via the file name.

## Health check

🩺 Check in the main menu runs the built-in server self-diagnostics — the
`manage_amneziawg.sh check` subcommand.

- Unlike other operations, `check` may exit with a non-zero exit code
  (usually `1`) if the script detected problems (e.g. the interface isn't
  running, the configuration doesn't match) — the bot does **not** treat
  this as an execution error: both stdout and the exit code are inspected,
  but the user is only shown an error message if the output itself is
  empty. The bot sends the entire `check` stdout to the chat as-is.
- The output is wrapped in `<pre>` (HTML `parse_mode`), so it preserves the
  script's monospace formatting and isn't parsed as markup; special
  characters (`<`, `>`, `&`) in the output are escaped before sending, so
  that stray `<...>` sequences in the diagnostics don't break the HTML
  message.
- While the check is running, the bot shows an intermediate message:
  "⏳ Checking server…".

## Settings persistence (state_file)

Each administrator's language and the global PSK default are stored not in
memory but in a JSON file whose path is set by the `state_file` field in
`config.toml` (default `/etc/awgram/state.json`, see
`deploy/config.example.toml`). The format is plain JSON like
`{"psk_default": true, "langs": {"111111111": "ru"}}`. Writes are atomic
(first to a temporary `state.json.tmp`, then `rename` over the main file),
so a process interruption mid-write doesn't corrupt the file. On startup the
bot reads `state_file` if it exists and parses successfully — otherwise it
starts with an empty state (no one has a chosen language, PSK default off)
and creates the file on the first settings change. So a language/PSK-default
change survives a bot or server restart — nothing else needs to be
configured, but the directory holding `state_file` (default `/etc/awgram`)
must exist and be writable by the bot process (see privilege modes above if
the bot doesn't run as root).

## HTML message markup

All bot messages use Telegram's `parse_mode = HTML` (`ParseMode::Html` in
`teloxide`), not Markdown/MarkdownV2. Any dynamic value that ends up in a
message's text (a client's name, the import URI, `check` output, backup file
names) is escaped beforehand by the `i18n::html_escape` function
(`&` → `&amp;`, `<` → `&lt;`, `>` → `&gt;`, in that order — otherwise `&`
would get double-escaped). Markup such as `<b>bold</b>` or
`<code>monospace</code>` inside the message templates themselves still works
as usual. Secrets (the bot token, private keys from `.conf`, PresharedKey,
the script's stderr) are never inserted into message text directly —
`.conf`/QR are sent as files, and script errors are turned into generic
localized phrases via `i18n::error_text`.

Example client card (`👥 Clients` → a client), as it's actually rendered in
Russian with `parse_mode = HTML`:

```
👤 <b>alice</b>
Статус: Активен
IP: 10.8.0.2
Трафик:  ↓ 1.2 MB   ↑ 340 KB
Рукопожатие: 2 мин назад
Действует: 12д 4ч
```

The keyboard under the card: "📄 Config" / "🗑 Delete", then "⬅️ To menu". In
English the same fields read as `Status:`, `Traffic:`, `Handshake:`,
`Expires:`.

## Smoke checklist

This is a manual checklist for the first real run on a test (or production)
server that already has AmneziaWG configured. Check items off as you go —
the results here are **not** filled in; this is an operator's instruction
sheet, not a run report.

1. **Install and start.** Install the bot following the steps above, start
   the service. `journalctl -u awgram -f` should show the messages
   `конфиг загружен` and `запуск long polling`, with no errors.
2. **Authorization.** `/start` from an administrator (an ID from
   `admin_ids`) → the bot shows the main menu (`👥 Clients`, `➕ Add`,
   `📊 Stats`). `/start` from an outside user → the bot replies "⛔ Access
   denied", a `warn` entry appears in the log; no VPN operation is
   performed.
3. **Adding a client.** `➕ Add` → enter the client's name → choose the
   expiry period → the bot sends the `.conf` file, a QR code (if generated
   by the script), and a text message with an import URI link. `add`
   doesn't print JSON — the bot runs
   `manage_amneziawg.sh add <name> [--expires=<period>]` and then reads the
   created files from `clients_dir`. If the bot can't find the `.conf`
   file — check that `clients_dir` in the config points to the directory
   where the script places files (default `/root/awg`).

   **Duplicate protection.** If a client with the entered name already
   exists, the bot doesn't silently overwrite it — instead a warning is
   shown with "♻️ Recreate" / "⬅️ To menu" buttons. Choosing "Recreate"
   makes the bot ask for the expiry and PSK again, then delete the old
   client and create a new one (new keys, new IP). Existence is checked via
   `list --json` — if that call fails, creation is not blocked (fail-open,
   with a `warn` in the log).

   **Detecting the script's "silent skip".** The upstream `manage add`, when
   given an existing name, prints a warning, skips the client, and exits
   with code 0 — to the bot this would look like success, and it would send
   the old `.conf` as if it were new (without the requested expiry and PSK).
   The bot compares the `<name>.conf` fingerprint (mtime, size, inode)
   before and after `add`: if the file existed and didn't change, creation
   was skipped. In that case (a race between the check and `add`, or the
   fail-open above) the bot shows the same warning with a "♻️ Recreate"
   button instead of sending the stale config.
4. **Client list.** `👥 Clients` → list (`list --json`: `name`, `ip`,
   `client_ipv6`, `status`, `status_code`). The client card is built from
   `stats --json` (`rx`, `tx`, `last_handshake`, `status`) and shows status
   and traffic (↓/↑). The "Config" button re-sends the `.conf`, QR, and URI
   for an existing client.

   **Expiry label in the list.** Temporary clients are marked in the list
   with a label like "⏳ 6d" (days/hours until expiry); a client that has
   expired but hasn't yet been removed by cron shows "⏳ expired".
   Unlimited clients are shown with no label. The expiry is read from
   `clients_dir/expiry/<name>` (the same source used by the client card).

   **Config regeneration (regen).** The "🔄 Regenerate" button on a
   client's card regenerates its `.conf`/QR/URI (keys and IP are preserved)
   and sends the fresh files. The "🔄 Regenerate all" button at the bottom
   of the client list — after confirmation, runs `regen` for everyone; the
   "🔀 + reset routes" option adds `--reset-routes` (replacing individual
   AllowedIPs with the server's global routing mode — needed after changing
   the routing mode). The bulk regeneration runs with a tripled timeout; on
   partial failures the bot reports "completed with errors", with details in
   the server logs.
5. **Stats.** `📊 Stats` → a summary: total clients, active ones, total
   traffic.
6. **Deletion.** Deleting a client → confirmation request → after
   confirming, the client disappears from the `👥 Clients` list.
7. **Custom expiry validation.** In the "✏️ Custom" field (custom expiry),
   input like `10d` is accepted and used; garbage input (an empty string,
   `10`, `10x`, `1.5d`, `10d;ls`, etc.) is rejected with a clear error
   message, without ever calling the script.

### Notes from the first real run

- **(a) `--json` schema — checked against the script's source. ✅** The
  `Client`/`AddResult` structs in `src/vpn/model.rs` have been aligned with
  the real `manage_amneziawg.sh`: `list --json` (`name`, `ip`,
  `client_ipv6`, `status`, `status_code`), `stats --json` (the same fields
  plus `rx`, `tx`, `last_handshake`), and `add` doesn't print JSON — the
  result is assembled from files. If the script's output format changes in
  the future, update the structs and fixtures in `src/vpn/model.rs` and run
  `cargo test`.
- **(b) HTML instead of MarkdownV2 for the URI. ✅ Resolved.**
  `send_client_files` in `src/bot/render.rs` now sends the client's import
  URI with `parse_mode = ParseMode::Html` (via `i18n::import_link`, which
  wraps the URI in `<code>...</code>` after `html_escape`), instead of
  MarkdownV2 — see the "HTML message markup" section above. This applies to
  every bot message, not just the URI: MarkdownV2 is no longer used
  anywhere in the project.

## Smoke checklist (v2)

A manual checklist for verifying second-iteration features (language, PSK,
backup/restore, diagnostics, persistence) on a test (or production) server.
As with the section above, this is an operator's instruction sheet — the
items **were not run** as part of this task (no access to a live AmneziaWG
server from the environment where these changes were prepared), so the
results here are not filled in.

1. **Language choice on first `/start`.** Remove/rename the existing
   `state_file` so the admin is treated as "new" → `/start` → a screen
   "🌐 Выберите язык / Choose language:" should appear with 🇷🇺 Русский /
   🇬🇧 English buttons. Choose one → the main menu should render in the
   chosen language (check the button labels "👥 Клиенты"/"👥 Clients" etc.).
2. **Changing the language in settings.** ⚙️ Settings → tap the other
   language's button → the settings screen's text and keyboard should
   switch immediately, without a repeat `/start`. Open the main menu — it
   should also be in the new language.
3. **PSK default in settings + override in add.**
   - ⚙️ Settings → toggle "PSK: on ✅ / PSK: off ⬜" → the value should
     persist (see item 7 below about `state.json`).
   - ➕ Add → name → expiry → at the PSK step, the button matching the
     current global default should come first; create a client with an
     explicit "🔐 With PSK" (even if the default is "off") → open the sent
     `.conf` file and confirm it contains a `PresharedKey = ...` line.
     Repeat with "🔓 Without PSK" → confirm `PresharedKey` is absent from
     the config.
4. **Client card (HTML, localization).** `👥 Clients` → pick a client → the
   card should render with the name in bold (`<b>...</b>` → bold text, not
   raw tags), with fields "Статус/Status", "Трафик/Traffic",
   "Рукопожатие/Handshake", "Действует/Expires" in the current language; the
   "📄 Config"/"🗑 Delete" buttons should also be in that language.
5. **Backup: create, download, list, restore.**
   - 💾 Backup → "➕ Create backup" → a confirmation with the `.tar.gz`
     archive's name and a card with "📥 Download"/"♻️ Restore" buttons
     should arrive.
   - On the card, tap "📥 Download" → a `.tar.gz` file should arrive in the
     chat as a document (check that it opens/extracts).
   - 💾 Backup → "📃 List backups" → all created archives should be listed
     (newest on top).
   - Open an archive from the list → "♻️ Restore" → a confirmation request
     with the archive name should appear → confirm "✅ Yes" → "✅ Restore
     complete." should arrive; after that `👥 Clients` should reflect the
     state at the time of the backup.
6. **Check.** 🩺 Check → the message "⏳ Checking server…" should appear,
   followed by the result in a monospace block (`<pre>`) — verify that the
   block doesn't break the message markup even if the script's output
   contains `<`/`>`/`&` characters. If `manage_amneziawg.sh check` detects a
   problem on this server (exits with code 1) — the message should still
   arrive with the problem's text, not show a generic execution error.

   **Diagnostics.** The "🔬 Diagnose" button in the main menu runs
   `manage diagnose` (public IP, module state, known carrier-related
   issues) and shows the report; long output is truncated to Telegram's
   limit.
7. **Persistence across a restart.** Note the current language and PSK
   default state → `cat /etc/awgram/state.json` (or the path from
   `state_file` in the config) and confirm the JSON contains `"psk_default"`
   at the expected value and an entry in `"langs"` with the administrator's
   ID and chosen language → `sudo systemctl restart awgram` → `/start` from
   the same administrator → the menu should open immediately in the
   previous language (no repeated language prompt), and the next visit to
   ⚙️ Settings should show the previous PSK-default value.

## Tests and lint

Before deploying to production, run:

```bash
cargo test
cargo build --release
cargo clippy --all-targets -- -D warnings   # if clippy is installed
```
