# awgram

[🇷🇺 Русский](README.md) · 🇬🇧 English

[![CI](https://github.com/ekuraev/awgram/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/ekuraev/awgram/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/ekuraev/awgram)](https://github.com/ekuraev/awgram/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A Rust Telegram bot for managing [AmneziaWG](https://amnezia.org/) clients
straight from your phone: add/remove a client, view the list and traffic —
no SSH required.

<p align="center">
  <img src="docs/media/awgram-tg.webp" alt="awgram in Telegram" width="420">
</p>

**awgram manages native AmneziaWG** — the Linux kernel module (set up by the
[installer](https://github.com/bivlked/amneziawg-installer)) — entirely from
Telegram: once installed, no console or terminal is ever needed. Native AWG
is noticeably faster and lighter than container-based setups — especially
tangible on budget VPS hosts.

## Features

### Clients

- ➕ **Add**: expiry (1d–365d presets or custom), PSK, duplicate guard with
  recreate; you get back a `.conf`, a QR and an import link.
- 👥 **List**: three-color status (🟢 online / 🟡 no handshake / 🔴 offline)
  with last-handshake time right in the button, ↓/↑ traffic, ⏳ expiry badge;
  status filter and "online-first" sorting; client card, deletion with
  confirmation; menus, lists, prompts and operation results all live in one
  message — the chat never piles up duplicates.
- 🔗 **AllowedIPs from presets** — at creation, in bulk creation and when
  editing a client: RFC 1918 local networks, the VPN subnet, a full tunnel or
  your own CIDR list; you can also keep the server's routing mode. When the
  installer supports `add --allowed-ips`, the routes are set by the same call
  that creates the client, otherwise by a separate edit right after it.
- ⚙️ **Modify client parameters**: Keepalive, DNS, AllowedIPs, Endpoint.
- 🔄 **Config re-issue**: one client or all at once (optionally with route
  reset).
- 📊 **Detailed traffic stats**: today / 7 days / 30 days, trends, top
  clients — a dedicated SQLite store, data survives reboots.
- 📜 **History** of connections and operations for every client.
- 🟢 **Honest online status**: online only when the handshake is under
  5 minutes old.
- 📦 **Bulk generation** — create up to 10 clients at once by prefix
  (`user-01 … user-10`) in a single action, delivering configs as an album.
- 🎛️ **Delivery filter** — configure which artifacts (`.conf` / QR / link)
  are automatically sent after creation.
- 🧩 **Per-artifact delivery** — from the client card you can request config,
  QR, link, or all of them separately.

### Groups & delegation

- 🗂️ **Client groups**: create, rename, delete; move clients between
  groups and re-issue all configs of a group at once.
- 🤝 **Delegation**: group admins see and manage only their own group;
  assigned via a one-time invite link (24 h TTL) or by user ID.
- 📏 **Quotas**: per-group client limit — applies to group admins (the
  owner is unlimited).

### Server

- 🩺 **Check**: card with service, interface, port, module, clients and
  firewall status (✅/⚠️/❌).
- 🔬 **Environment diagnostics**.
- 🔁 **Restart service** and 🛠 **kernel module repair** (DKMS rebuild).
- 💾 **Backup/restore** of the AmneziaWG state, archive download to chat.

### Settings & security

- ⚙️ **Settings**: RU/EN language (per admin), default PSK, client name
  ID prefix; everything survives restarts (persistent state).
- 🔒 **Security**: access restricted to owners from `admin_ids` and the
  group admins they appoint, shell-free manage-script invocation, secrets
  never reach the logs, hardened mode (dedicated user + sudoers).
- 🧦 **Proxy to Telegram**: a priority list of `socks5`/`socks5h`/`http`/
  `https` proxies in `telegram_proxies` with automatic failover — for
  servers where the Bot API is unreachable directly; details and the
  routing-based alternative in [docs/proxy.en.md](docs/proxy.en.md).

## Quick start

1. Get a bot token from [@BotFather](https://t.me/BotFather) (`/newbot`)
   and your numeric ID from [@userinfobot](https://t.me/userinfobot).
2. On a VPS with the
   [AmneziaWG installer](https://github.com/bivlked/amneziawg-installer) set up, run:

   ```bash
   curl -fsSL https://github.com/ekuraev/awgram/releases/latest/download/install.sh | bash
   ```

3. Answer the installer's questions (language, root/hardened mode, token,
   admin IDs) — done: open your bot in Telegram and press `/start`.

Fully automated install — via flags:

```bash
curl -fsSL https://github.com/ekuraev/awgram/releases/latest/download/install.sh \
  | bash -s -- install --lang en --mode root --token 'TOKEN' --admins 111111111 --yes
```

You can skip the `--token` flag (so the token never lands in `argv` or shell
history) — `export AWGRAM_TOKEN='TOKEN'` before the same command without
`--token` instead.

Post-install management: `awgram-setup update | config | status | uninstall`.

Pre-release builds — available since **v0.7.0**: `awgram-setup update
--channel rc` (the choice is sticky, the rc channel also sees stable
releases; to return run `awgram-setup update --channel stable`). If the
server's `awgram-setup` is older than v0.7.0, it has no `--channel` flag
yet — either run a plain `awgram-setup update` (it updates the script
itself too), or install an rc right away with a one-liner from the release:

```bash
curl -fsSL https://github.com/ekuraev/awgram/releases/download/vX.Y.Z-rc.N/install.sh \
  | bash -s -- update --channel rc
```

## How it works

`awgram` is a single static binary (Rust, `teloxide`, long polling, no
webhook) living on the same VPS as the VPN. It never touches the AmneziaWG
configuration directly — it invokes the standard `manage_amneziawg.sh`
script (shell-free, with `--json`) and renders the result as an inline
Telegram menu. Access is restricted to owners from `admin_ids` and the
group admins they appoint; the token and `.conf`/QR contents never reach
the logs.

## AmneziaWG installer compatibility

The bot is a layer on top of `manage_amneziawg.sh` from
[bivlked/amneziawg-installer](https://github.com/bivlked/amneziawg-installer)
and depends directly on its interface.

- **Supported installer version:
  [v5.31.0](https://github.com/bivlked/amneziawg-installer/releases/tag/v5.31.0)**
  (`--json` contract verified). Minimum is
  [v5.21.0](https://github.com/bivlked/amneziawg-installer/releases/tag/v5.21.0);
  older v5.20.x releases are not supported — the bot uses the extended
  `--json` interface for management commands introduced in v5.21.0.
  v5.21.1–v5.31.0 keep the JSON contract intact: v5.21.1/v5.21.2 are
  validation bugfixes, v5.22.0 added an `awgsetup_cfg.init` drift warning
  to `regen`/`check` (goes to stderr, stdout JSON untouched), v5.23.0
  only changes the installers (kernel module handling on older kernels),
  v5.24.0 added an additive `module.version` field to `check --json`,
  v5.25.0 adds only new warnings, all of which go to stderr, v5.26.0 only
  touches the cascade routing script and the diagnostic report,
  v5.27.0 only changes the installer (package-removal consent),
  v5.27.1 changes `manage_amneziawg.sh`: `modify`/`regen` normalize the
  `AllowedIPs`/`DNS` lists to the canonical "a, b, c" form and `regen`
  no longer collapses them; the envelopes are untouched — the `modify`
  reply still echoes `value` as sent, new messages go to stderr, and the
  bot already sends lists in the canonical form; v5.28.0–v5.29.0 only
  touch the installer (boot-critical package protection during the system
  upgrade, key masking in the diagnostic report), release signing and
  docs — `manage_amneziawg.sh` changed nothing but its version number;
  v5.30.0 bounds every interface call by a timeout and stops reporting an
  unread state as a measured one — on a failed read `list --json` leaves
  clients at `no_data` instead of "no handshake" (the bot understands that
  status and marks it yellow), and `check --json` returns an empty
  `interface.addresses`, which the bot degrades on gracefully: the VPN
  subnet preset is hidden and bulk creation reports capacity as
  unavailable; v5.31.0 decides a full tunnel by route coverage, so the
  default "Amnezia" mode now gets `::/0`, and `modify` warns when it is
  handed a full tunnel without `::/0` — the bot's "all traffic" preset
  sends `0.0.0.0/0, ::/0`, so that warning never fires. All new messages
  from both releases go to stderr and the `--json` envelopes are
  unchanged.
- Subcommands used: `add`, `remove`, `list`, `stats`, `regen`, `modify`,
  `backup`, `restore`, `check`, `restart`, `repair-module` — all with `--json`.
- **Optional `add --allowed-ips`**
  ([Issue #253](https://github.com/bivlked/amneziawg-installer/issues/253))
  — per-client routes set by the same call that creates the client. The bot
  detects the flag from the script's own help (`--help`, which changes
  nothing) rather than from a version number: the release carrying the flag is
  not known in advance, and on a half-updated server the number and reality
  disagree. Without the flag the old `add` + `modify` path is used and the
  minimum version stays the same. Bulk creation has no fallback, since
  `modify` would have to run once per client, so the routes step is hidden
  there.

## Building from source

You need a stable Rust toolchain (1.95 or newer) and `cargo`; TLS is
rustls-based, no system `libssl` required.

```bash
cargo build --release                 # target/release/awgram
./scripts/build-musl.sh [arm64|all]   # static Linux binaries in dist/ (requires Docker)
```

Releases on a `v*` tag build amd64+arm64 binaries with `sha256` checksums
automatically.

## License

[MIT](LICENSE)
