# Changelog

Формат — [Keep a Changelog](https://keepachangelog.com/ru/1.1.0/),
версионирование — [SemVer](https://semver.org/lang/ru/).

## [Unreleased]

## [0.1.0] — 2026-07-15

### ⚠️ Переименование awg-bot → awgram (миграция действующего деплоя)

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

### Added

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
- Статическая musl-сборка `scripts/build-musl.sh` (linux-amd64, rustls).

[Unreleased]: https://github.com/ekuraev/awgram/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/ekuraev/awgram/releases/tag/v0.1.0
