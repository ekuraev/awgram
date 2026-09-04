# awgram

🇷🇺 Русский · [🇬🇧 English](README.en.md)

[![CI](https://github.com/ekuraev/awgram/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/ekuraev/awgram/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/ekuraev/awgram?logo=github&label=release)](https://github.com/ekuraev/awgram/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/ekuraev/awgram/total?logo=github&label=downloads)](https://github.com/ekuraev/awgram/releases)
[![Discussions](https://img.shields.io/github/discussions/ekuraev/awgram?logo=github&label=discussions)](https://github.com/ekuraev/awgram/discussions)
<br>
[![Platform](https://img.shields.io/badge/linux-amd64%20%7C%20arm64-informational?logo=linux&logoColor=white)](https://github.com/ekuraev/awgram/releases/latest)
[![Rust](https://img.shields.io/badge/dynamic/toml?url=https%3A%2F%2Fraw.githubusercontent.com%2Fekuraev%2Fawgram%2Fmain%2FCargo.toml&query=%24.package.rust-version&prefix=%E2%89%A5%20&label=rust&logo=rust&color=orange)](Cargo.toml)
[![Installer](https://img.shields.io/badge/amneziawg--installer-%E2%89%A5%20v5.21.0%20%C2%B7%20tested%20v5.31.0-blue)](docs/compat.md)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Telegram-бот на Rust для управления клиентами [AmneziaWG](https://amnezia.org/) прямо
с телефона: добавить/удалить клиента, посмотреть список и трафик — без SSH.

<p align="center">
  <img src="docs/media/social-preview.png" alt="awgram — управление AmneziaWG из Telegram" width="100%">
</p>

https://github.com/user-attachments/assets/35af60f4-c7d8-44d5-9c90-bcd06b20c864

**awgram управляет нативным AmneziaWG** — kernel-модулем для Linux
(ставится [инсталлером](https://github.com/bivlked/amneziawg-installer)) —
целиком из Telegram: после установки не нужны ни консоль, ни терминал.
Нативный AWG заметно быстрее и экономнее контейнерных решений — особенно
это ощутимо на недорогих VPS.

## Возможности

### Клиенты

- ➕ **Добавление**: срок (пресеты 1д–365д или свой), PSK, защита от
  дубликатов с пересозданием; в ответ — `.conf`, QR и ссылка импорта.
- 👥 **Список**: трёхцветный статус (🟢 онлайн / 🟡 без handshake / 🔴 оффлайн)
  и время последнего handshake прямо в кнопке, трафик ↓/↑, метка ⏳ срока;
  фильтр по статусу и сортировка «онлайн вперёд»; карточка клиента,
  удаление с подтверждением; меню, списки, вопросы и результаты операций
  живут в одном сообщении — чат не копит дубли.
- 🔗 **AllowedIPs готовыми пресетами** — при создании, массовой генерации и
  правке клиента: локальные сети RFC 1918 и типовые /24 домашних роутеров,
  подсеть VPN, полный туннель или свой список CIDR; можно оставить режим
  маршрутизации сервера. Режим «исключать из VPN» пускает весь трафик в
  туннель, кроме отмеченных сетей — клиент остаётся виден в своей локальной
  сети. Если инсталлер
  умеет `add --allowed-ips`, маршруты ставятся одной командой вместе с
  созданием, иначе — отдельной правкой сразу после него.
- ⚙️ **Изменение параметров** клиента: Keepalive, DNS, AllowedIPs, Endpoint.
- 🔄 **Перевыпуск конфигов**: одного или всех сразу (опционально — со сбросом
  маршрутов).
- 📊 **Детальная статистика трафика**: сегодня / 7 дней / 30 дней, тренды,
  топ клиентов — собственное SQLite-хранилище, данные переживают ребуты.
- 📜 **История** подключений и операций по каждому клиенту.
- 🟢 **Честный онлайн-статус**: онлайн только при handshake младше 5 минут.
- 📦 **Массовая генерация** — создание до 10 клиентов за раз по префиксу
  (`user-01 … user-10`) одним действием, с выдачей конфигов альбомом.
- 🎛️ **Фильтр выдачи** — настройка, какие артефакты (`.conf` / QR / ссылка)
  автоматически выдаются после создания.
- 🧩 **Поартефактная выдача** — из карточки клиента можно запросить конфиг,
  QR, ссылку или всё сразу по отдельности.

### Группы и делегирование

- 🗂️ **Группы клиентов**: создание, переименование, удаление; перенос
  клиентов между группами и массовый перевыпуск конфигов группы.
- 🤝 **Делегирование**: групповые админы видят и управляют только своей
  группой; назначение — одноразовой инвайт-ссылкой (TTL 24 ч) или
  по user ID.
- 📏 **Квоты**: лимит числа клиентов на группу — действует на групповых
  админов (владелец без ограничений).

### Сервер

- 🩺 **Проверка**: карточка со статусом сервиса, интерфейса, порта, модуля,
  клиентов и фаервола (✅/⚠️/❌).
- 🔬 **Диагностика окружения**.
- 🔁 **Перезапуск сервиса** и 🛠 **починка модуля ядра** (DKMS rebuild).
- 💾 **Бэкапы**: по расписанию с ротацией и отчётами, комментарии
  и закрепление, восстановление AmneziaWG и, по желанию, БД бота
  (группы, статистика), загрузка бэкапа файлом. Нюансы hardened-режима
  и ручное восстановление — в [docs/operations.md](docs/operations.md).

### Настройки и безопасность

- ⚙️ **Настройки**: язык RU/EN (у каждого админа свой), PSK по умолчанию,
  ID-префикс имён клиентов; всё переживает рестарт (персистентный state).
- 🔒 **Безопасность**: доступ только для владельцев из `admin_ids` и
  назначенных ими групповых админов, вызов manage-скрипта без shell,
  секреты не попадают в логи, hardened-режим (отдельный пользователь +
  sudoers).
- 🧦 **Прокси до Telegram**: приоритетный список `socks5`/`socks5h`/`http`/
  `https` в `telegram_proxies` с автофейловером — для серверов, где Bot API
  напрямую недоступен; подробности и альтернатива через маршрутизацию —
  в [docs/proxy.md](docs/proxy.md).

## Требования

- Linux-VPS на amd64 или arm64 с systemd; root или sudo на время установки.
- Нативный AmneziaWG, поставленный
  [bivlked/amneziawg-installer](https://github.com/bivlked/amneziawg-installer)
  версии v5.21.0 и новее (сверено с v5.31.0, см.
  [docs/compat.md](docs/compat.md)).
- Токен бота от [@BotFather](https://t.me/BotFather) и числовые Telegram ID
  администраторов.
- Доступ с сервера к `api.telegram.org` — напрямую или через прокси
  ([docs/proxy.md](docs/proxy.md)).

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
Предрелизные сборки — `awgram-setup update --channel rc`; подробнее о каналах
обновлений — в [docs/operations.md](docs/operations.md#каналы-обновлений).

## Как это работает

`awgram` — один статический бинарник (Rust, `teloxide`, long polling, без
webhook), который живёт на том же VPS, что и VPN. Конфигурацию AmneziaWG он
не трогает — вызывает штатный скрипт `manage_amneziawg.sh` (без shell, с
флагом `--json`) и рендерит результат в inline-меню Telegram. Доступ
ограничен владельцами из `admin_ids` и назначенными ими групповыми
админами; токен и содержимое `.conf`/QR никогда не попадают в логи.

## Совместимость с инсталлером AmneziaWG

Бот — надстройка над `manage_amneziawg.sh` из
[bivlked/amneziawg-installer](https://github.com/bivlked/amneziawg-installer)
и напрямую зависит от его `--json`-интерфейса.

- **Поддерживаемая версия — [v5.31.0](https://github.com/bivlked/amneziawg-installer/releases/tag/v5.31.0)**
  (сверен `--json`-контракт), минимальная —
  [v5.21.0](https://github.com/bivlked/amneziawg-installer/releases/tag/v5.21.0).
- Подкоманды: `add`, `remove`, `list`, `stats`, `regen`, `modify`, `backup`,
  `restore`, `check`, `restart`, `repair-module` — все с `--json`.
- Необязательный `add --allowed-ips` определяется по справке скрипта, а не по
  версии; без него работает путь `add` + `modify`.

Что менялось в каждой версии инсталлера и как это отразилось на боте — в
[docs/compat.md](docs/compat.md).

## Сборка из исходников

Нужен стабильный Rust не ниже 1.95 и `cargo`; TLS — на rustls, системный
`libssl` не нужен.

```bash
cargo build --release                 # target/release/awgram
./scripts/build-musl.sh [arm64|all]   # статические Linux-бинарники в dist/ (нужен Docker)
```

Релизы на тег `v*` собирают бинарники amd64+arm64 c `sha256`-суммами
автоматически.

## Сообщество

- 💬 Вопросы и обсуждения — [Discussions](https://github.com/ekuraev/awgram/discussions).
- 🐞 Баги и идеи — [Issues](https://github.com/ekuraev/awgram/issues/new/choose)
  по шаблонам; дорожная карта — [открытые предложения](https://github.com/ekuraev/awgram/issues?q=is%3Aissue+is%3Aopen+label%3Aenhancement).
- 🔐 Уязвимости — приватно, по [SECURITY.md](SECURITY.md).
- 🤝 Вклад — [CONTRIBUTING.md](CONTRIBUTING.md); история изменений —
  [CHANGELOG.md](CHANGELOG.md).

## Благодарности

- [bivlked/amneziawg-installer](https://github.com/bivlked/amneziawg-installer) —
  инсталлер и `manage_amneziawg.sh`, на которые опирается бот.
- [Amnezia](https://amnezia.org/) — за AmneziaWG.
- [teloxide](https://github.com/teloxide/teloxide) — Telegram Bot API на Rust.

## Лицензия

[MIT](LICENSE)
