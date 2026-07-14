# Changelog

Формат — [Keep a Changelog](https://keepachangelog.com/ru/1.1.0/),
версионирование — [SemVer](https://semver.org/lang/ru/).

## [Unreleased]

## [0.1.0] — 2026-07-14

### Added

- Telegram-бот для управления клиентами AmneziaWG через `manage_amneziawg.sh`
  (`--json`): добавление/удаление/список/трафик, QR и `.conf` клиентов.
- Перевыпуск конфигов: одного клиента и массовый (`--reset-routes`).
- Диагностика окружения (кнопка 🔬), метка ⏳ срока действия клиентов.
- Локализация RU/EN, PSK-дефолт, backup/restore, персистентное состояние.
- Статическая musl-сборка `scripts/build-musl.sh` (linux-amd64, rustls).
