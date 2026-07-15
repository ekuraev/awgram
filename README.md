# awgram

🇷🇺 Русский · [🇬🇧 English](README.en.md)

[![CI](https://github.com/ekuraev/awgram/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/ekuraev/awgram/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/ekuraev/awgram)](https://github.com/ekuraev/awgram/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Telegram-бот на Rust для управления клиентами [AmneziaWG](https://amnezia.org/) прямо
с телефона: добавить/удалить клиента, посмотреть список и трафик — без SSH.

<p align="center">
  <img src="docs/media/awgram-tg.webp" alt="awgram в Telegram" width="420">
</p>

**awgram управляет нативным AmneziaWG** — kernel-модулем для Linux
(ставится [инсталлером](https://github.com/bivlked/amneziawg-installer)) —
целиком из Telegram: после установки не нужны ни консоль, ни терминал.
Нативный AWG заметно быстрее и экономнее контейнерных решений — особенно
это ощутимо на недорогих VPS.

## Возможности

- ➕ **Добавление клиента**: срок действия (пресеты 1д–365д или свой), PSK,
  защита от дубликатов с пересозданием; в ответ — `.conf`-файл, QR-код и
  ссылка для импорта.
- 👥 **Список клиентов**: статус, трафик ↓/↑, метка ⏳ срока действия;
  карточка клиента, повторная выдача конфига, удаление с подтверждением.
- 🔄 **Перевыпуск конфигов**: одного клиента или всех сразу (опционально —
  со сбросом маршрутов).
- 📊 **Статистика**: всего клиентов, активных, суммарный трафик.
- 💾 **Бэкап/восстановление** состояния AmneziaWG, скачивание архива в чат.
- 🩺 **Проверка сервера** и 🔬 **диагностика окружения**.
- ⚙️ **Настройки**: язык RU/EN (у каждого админа свой), PSK по умолчанию,
  ID-префикс имён клиентов; всё переживает рестарт (персистентный state).
- 🔒 **Безопасность**: доступ только для `admin_ids`, вызов manage-скрипта
  без shell, секреты не попадают в логи, hardened-режим (отдельный
  пользователь + sudoers).

## Быстрый старт

1. Получите токен бота у [@BotFather](https://t.me/BotFather) (`/newbot`)
   и свой числовой ID у [@userinfobot](https://t.me/userinfobot).
2. На VPS с установленным
   [AmneziaWG-инсталлером](https://github.com/bivlked/amneziawg-installer) выполните:

   ```bash
   curl -fsSL https://github.com/ekuraev/awgram/releases/latest/download/install.sh | bash
   ```

3. Ответьте на вопросы установщика (язык, режим root/hardened, токен,
   ID админов) — готово: откройте бота в Telegram и нажмите `/start`.

Полностью автоматическая установка — флагами:

```bash
curl -fsSL https://github.com/ekuraev/awgram/releases/latest/download/install.sh \
  | bash -s -- install --lang ru --mode root --token 'ТОКЕН' --admins 111111111 --yes
```

Токен можно не передавать флагом (тогда он не попадёт ни в `argv`, ни в
историю шелла) — вместо этого `export AWGRAM_TOKEN='ТОКЕН'` перед той же
командой без `--token`.

Управление после установки: `awgram-setup update | config | status | uninstall`.

## Как это работает

`awgram` — один статический бинарник (Rust, `teloxide`, long polling, без
webhook), который живёт на том же VPS, что и VPN. Конфигурацию AmneziaWG он
не трогает — вызывает штатный скрипт `manage_amneziawg.sh` (без shell, с
флагом `--json`) и рендерит результат в inline-меню Telegram. Доступ
ограничен списком `admin_ids`; токен и содержимое `.conf`/QR никогда не
попадают в логи.

## Совместимость с инсталлером AmneziaWG

Бот — надстройка над `manage_amneziawg.sh` из
[bivlked/amneziawg-installer](https://github.com/bivlked/amneziawg-installer)
и напрямую зависит от его интерфейса.

- **Поддерживаемая версия инсталлера:
  [v5.19.2](https://github.com/bivlked/amneziawg-installer/releases/tag/v5.19.2)**
  (проверено с ней; более новые — на свой риск до обновления этой строки).
- Используемые подкоманды: `add`, `remove`, `list`, `stats`, `regen`,
  `backup`, `restore`, `check`, `diagnose` — все с `--json`.

## Сборка из исходников

Нужен стабильный Rust и `cargo`; TLS — на rustls, системный `libssl` не нужен.

```bash
cargo build --release                 # target/release/awgram
./scripts/build-musl.sh [arm64|all]   # статические Linux-бинарники в dist/ (нужен Docker)
```

Релизы на тег `v*` собирают бинарники amd64+arm64 c `sha256`-суммами
автоматически.

## Лицензия

[MIT](LICENSE)
