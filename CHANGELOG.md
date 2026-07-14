# Changelog

Формат — [Keep a Changelog](https://keepachangelog.com/ru/1.1.0/), версионирование — [SemVer](https://semver.org/lang/ru/).
Format — [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), versioning — [SemVer](https://semver.org/).

## [Unreleased]

## [0.1.0] — 2026-07-15

### 🇷🇺 Русский

#### ⚠️ Переименование awg-bot → awgram (миграция действующего деплоя)

Проект переименован; бинарник, юнит, env-переменные и пути конфига изменились.
На работающем VPS выполните разово:

1. `systemctl disable --now awg-bot` — остановить старый юнит.
2. `mv /etc/awg-bot /etc/awgram` — каталог конфига (config.toml, env, state.json).
3. В `/etc/awgram/env` переименуйте переменную `AWG_BOT_TOKEN` → `AWGRAM_TOKEN`;
   если в `config.toml` задан `state_file` — поправьте путь на `/etc/awgram/state.json`.
4. Установите новый бинарник `/usr/local/bin/awgram` и юнит `deploy/awgram.service`,
   затем `systemctl daemon-reload && systemctl enable --now awgram`.
5. Удалите старые `/usr/local/bin/awg-bot` и `/etc/systemd/system/awg-bot.service`;
   в hardened-режиме также обновите `/etc/sudoers.d/awg-bot` (пользователь теперь `awgram`).

#### Добавлено

- Telegram-бот для управления клиентами AmneziaWG через `manage_amneziawg.sh`
  (`--json`): добавление/удаление/список/трафик, QR и `.conf` клиентов.
- Установщик `install.sh` / `awgram-setup`: установка одной командой
  (интерактивно или флагами `--yes`), режимы root/hardened, RU/EN,
  команды update/config/status/uninstall, sha256-проверка релиза.
- Релизные статические бинарники **amd64 + arm64** (`awgram-linux-{amd64,arm64}`):
  сборка через [cross](https://github.com/cross-rs/cross) по тегу `v*`;
  `scripts/build-musl.sh` принимает `amd64|arm64|all`.
- Перевыпуск конфигов: одного клиента и массовый (`--reset-routes`).
- Диагностика окружения (кнопка 🔬), метка ⏳ срока действия клиентов.
- Локализация RU/EN, PSK-дефолт, backup/restore, персистентное состояние.

### 🇬🇧 English

#### ⚠️ Rename awg-bot → awgram (migrating an existing deployment)

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

#### Added

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
