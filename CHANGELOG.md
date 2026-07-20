# Changelog

Формат — [Keep a Changelog](https://keepachangelog.com/ru/1.1.0/), версионирование — [SemVer](https://semver.org/lang/ru/).
Format — [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), versioning — [SemVer](https://semver.org/).

## [Unreleased]

## [0.3.0] — 2026-07-20

### 🇷🇺 Русский

#### ⚠️ Breaking

- Минимальная версия инсталлера поднята до
  [v5.21.0](https://github.com/bivlked/amneziawg-installer/releases/tag/v5.21.0).
  Бот переведён на расширенный `--json`-интерфейс команд управления
  (`add`/`remove`/`regen`/`modify`/`backup`/`restore`/`check`/`restart`/
  `repair-module`), которого нет в v5.20.x. На действующем VPS обновите
  инсталлер: `awgram-setup update` (или `bash install_amneziawg.sh --force`).

#### Добавлено

- 🛠 **Изменение параметров клиента** (`modify`): Keepalive, DNS, AllowedIPs,
  Endpoint — кнопка «⚙️ Изменить» в карточке клиента.
- 🔁 **Перезапуск сервиса** (`restart`) и 🛠 **починка модуля** (`repair-module`)
  — новый ряд обслуживания в главном меню.
- 🩺 **Структурированная карточка проверки**: статус сервиса, интерфейса,
  порта, модуля, клиентов и фаервола — вместо сырого `<pre>` с текстом.
- Точные сообщения об ошибках: «клиент не найден», «восстановление откачено».

#### Изменено

- Убраны хрупкие эвристики: fingerprint `.conf` для обнаружения «тихого
  пропуска» при `add`, поиск новейшего бэкапа по mtime, угадывание путей
  `.conf`/`.png`/`.vpnuri` по имени — теперь всё из JSON-ответа скрипта.
- Деструктивные команды (`remove`/`restore`/`restart`) запускаются с
  `AWG_STRICT_CONFIRM=1` + `--yes` (рекомендация маинтейнера инсталлера).

#### Исправлено (багфиксы code review)

- **P1.1**: `run()` отбрасывал stdout при ненулевом exit code, но инсталлер
  v5.21.0 печатает JSON и ЗАТЕМ выходит с кодом 1 для `exists`/`not_found`/
  `partial`/`rolled_back`/`repair rc=1/2`. Все status-ветки были недостижимы
  в проде (стабы `exit 0` маскировали баг). `run()` теперь всегда возвращает
  `(stdout, exit_code)`, методы парсят JSON независимо от кода выхода.
- **P1.2**: `restored.keys` десериализовался как `u32`, но инсталлер
  возвращает `"keys": true|false` (наличие `*.private`). Успешный restore
  падал на парсинге → бот сообщал о провале.
- **P2.1**: `vpnuri` в JSON-конверте — ПУТЬ к файлу, а не ссылка `vpn://`.
  `add`/`regen_client` теперь читают содержимое файла, иначе пользователь
  получал серверный путь вместо импорт-ссылки.
- **P2.2**: аварийный конверт `{"ok":false,"error":...}` при фатальной ошибке
  `check` десериализовался в фиктивный отчёт (все defaults). Теперь
  `try_error_envelope` ловит его → `ScriptFailed`.
- **P2.3**: `repair-module` использует отдельный timeout 300с (общий 60с
  обрывал DKMS rebuild + apt-установку kernel headers — заявлено до 5 минут).
- **P2.4**: endpoint-валидатор принимает порт 1..=65535 и требует парные
  скобки `[IPv6]:port` (ранее пропускал `host:0`, `host:99999`, `[host:port`).

### 🇬🇧 English

#### ⚠️ Breaking

- Minimum installer version bumped to
  [v5.21.0](https://github.com/bivlked/amneziawg-installer/releases/tag/v5.21.0).
  The bot now uses the extended `--json` interface for management commands
  (`add`/`remove`/`regen`/`modify`/`backup`/`restore`/`check`/`restart`/
  `repair-module`), unavailable in v5.20.x. On a running VPS, update the
  installer: `awgram-setup update` (or `bash install_amneziawg.sh --force`).

#### Added

- 🛠 **Modify client parameters** (`modify`): Keepalive, DNS, AllowedIPs,
  Endpoint — "⚙️ Modify" button in the client card.
- 🔁 **Restart service** (`restart`) and 🛠 **repair module** (`repair-module`)
  — new maintenance row in the main menu.
- 🩺 **Structured check card**: service, interface, port, module, clients and
  firewall status — instead of raw `<pre>` text.
- Precise error messages: "client not found", "restore rolled back".

#### Changed

- Removed fragile heuristics: `.conf` fingerprinting for silent-skip detection
  on `add`, newest-backup-by-mtime lookup, path guessing for
  `.conf`/`.png`/`.vpnuri` — now all from JSON response.
- Destructive commands (`remove`/`restore`/`restart`) run with
  `AWG_STRICT_CONFIRM=1` + `--yes` (recommended by the installer maintainer).

#### Fixed (code review bugfixes)

- **P1.1**: `run()` discarded stdout on non-zero exit code, but installer
  v5.21.0 prints JSON THEN exits with code 1 for `exists`/`not_found`/
  `partial`/`rolled_back`/`repair rc=1/2`. All status branches were
  unreachable in production (stubs `exit 0` masked the bug). `run()` now
  always returns `(stdout, exit_code)`; methods parse JSON regardless of
  exit code.
- **P1.2**: `restored.keys` deserialized as `u32`, but the installer returns
  `"keys": true|false` (presence of `*.private`). A successful restore failed
  to parse → bot reported failure.
- **P2.1**: `vpnuri` in the JSON envelope is a file PATH, not a `vpn://`
  link. `add`/`regen_client` now read the file contents — otherwise the user
  got a server path instead of an import link.
- **P2.2**: an error envelope `{"ok":false,"error":...}` on a fatal `check`
  failure deserialized into a fake report (all defaults). Now
  `try_error_envelope` catches it → `ScriptFailed`.
- **P2.3**: `repair-module` uses a dedicated 300s timeout (the common 60s
  cut off DKMS rebuild + apt kernel headers install — up to 5 minutes).
- **P2.4**: endpoint validator accepts port 1..=65535 and requires paired
  `[IPv6]:port` brackets (previously allowed `host:0`, `host:99999`,
  `[host:port`).
- **P2.5**: keepalive range widened from 0..=600 to 0..=65535 to match the
  installer (`manage.sh:1024`).

## [0.2.0] — 2026-07-15

### 🇷🇺 Русский

#### Добавлено

- Автозамена пробелов на «-» в имени клиента при добавлении; промпт явно
  предупреждает об этом.
- Опциональный уникальный ID-префикс имён (5 символов a-z0-9, например
  `k3x9f-alice`): глобальный тумблер «ID-префикс» в настройках бота,
  по умолчанию выключен.

### 🇬🇧 English

#### Added

- Spaces in a new client name are automatically replaced with "-";
  the name prompt says so explicitly.
- Optional unique name ID prefix (5 chars a-z0-9, e.g. `k3x9f-alice`):
  global "ID prefix" toggle in bot settings, off by default.

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

[0.3.0]: https://github.com/ekuraev/awgram/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/ekuraev/awgram/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/ekuraev/awgram/releases/tag/v0.1.0
