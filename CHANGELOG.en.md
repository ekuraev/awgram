# Changelog

[🇷🇺 Русский](CHANGELOG.md) · 🇬🇧 English

Format — [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
versioning — [SemVer](https://semver.org/).

## [Unreleased]

## [0.1.0] — 2026-07-15

### ⚠️ Rename awg-bot → awgram (migrating an existing deployment)

The project has been renamed; the binary, unit, environment variables and
config paths have changed. On a running VPS, perform once:

1. `systemctl disable --now awg-bot` — stop the old unit.
2. `mv /etc/awg-bot /etc/awgram` — the config directory (config.toml, env, state.json).
3. In `/etc/awgram/env` rename the variable `AWG_BOT_TOKEN` → `AWGRAM_TOKEN`;
   if `state_file` is set in `config.toml`, update the path to `/etc/awgram/state.json`.
4. Install the new binary `/usr/local/bin/awgram` and the `deploy/awgram.service` unit,
   then `systemctl daemon-reload && systemctl enable --now awgram`.
5. Remove the old `/usr/local/bin/awg-bot` and `/etc/systemd/system/awg-bot.service`;
   in hardened mode also update `/etc/sudoers.d/awg-bot` (the user is now `awgram`).

### Added

- Telegram bot for managing AmneziaWG clients via `manage_amneziawg.sh`
  (`--json`): add/remove/list/traffic, client QR codes and `.conf` files.
- Installer `install.sh` / `awgram-setup`: one-command install
  (interactive or via `--yes` flags), root/hardened modes, RU/EN,
  update/config/status/uninstall commands, sha256 release verification.
- Release static binaries **amd64 + arm64** (`awgram-linux-{amd64,arm64}`):
  built via [cross](https://github.com/cross-rs/cross) on `v*` tags;
  `scripts/build-musl.sh` accepts `amd64|arm64|all`.
- Config regeneration: single client and bulk (`--reset-routes`).
- Environment diagnostics (🔬 button), ⏳ client expiry badges.
- RU/EN localization, PSK default, backup/restore, persistent state.

[Unreleased]: https://github.com/ekuraev/awgram/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/ekuraev/awgram/releases/tag/v0.1.0
