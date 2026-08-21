# Changelog

Формат — [Keep a Changelog](https://keepachangelog.com/ru/1.1.0/), версионирование — [SemVer](https://semver.org/lang/ru/).
Format — [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), versioning — [SemVer](https://semver.org/).

## [0.9.0] — 2026-08-21

### 🇷🇺 Русский

#### ✨ Добавлено

- **Прокси до Telegram Bot API**: приоритетный список `telegram_proxies`
  в конфиге (`socks5`/`socks5h`/`http`/`https`, авторизация в URL).
  При старте бот выбирает первый живой прокси probe-запросом `getMe`;
  умерший в рантайме прокси лечится автоперезапуском через systemd с
  повторным выбором. Креды прокси не попадают в логи. Настройка и
  альтернатива через маршрутизацию — в [docs/proxy.md](docs/proxy.md)
  ([#48](https://github.com/ekuraev/awgram/issues/48)).

#### 🔧 Изменено

- **rusqlite 0.32 → 0.40** (bundled SQLite 3.53); для сборки из исходников
  теперь нужен Rust не ниже 1.95 — MSRV зафиксирован в `Cargo.toml`
  ([#44](https://github.com/ekuraev/awgram/pull/44)).
- **Совместимость с инсталлером**: поддерживаемая версия —
  [v5.27.0](https://github.com/bivlked/amneziawg-installer/releases/tag/v5.27.0)
  (сверен `--json`-контракт: `manage_amneziawg.sh` в v5.26.0/v5.27.0 не
  менялся — v5.26.0 трогает только каскадный скрипт маршрутизации и
  диагностический отчёт, v5.27.0 — только установщик); минимальная —
  по-прежнему v5.21.0.

### 🇬🇧 English

#### ✨ Added

- **Proxy to the Telegram Bot API**: a `telegram_proxies` priority list in
  the config (`socks5`/`socks5h`/`http`/`https`, credentials in the URL).
  At startup the bot picks the first live proxy via a `getMe` probe; a
  proxy dying at runtime is handled by an automatic systemd restart with
  re-selection. Proxy credentials never reach the logs. Setup and the
  routing-based alternative are covered in
  [docs/proxy.en.md](docs/proxy.en.md)
  ([#48](https://github.com/ekuraev/awgram/issues/48)).

#### 🔧 Changed

- **rusqlite 0.32 → 0.40** (bundled SQLite 3.53); building from source now
  requires Rust 1.95 or newer — the MSRV is pinned in `Cargo.toml`
  ([#44](https://github.com/ekuraev/awgram/pull/44)).
- **Installer compatibility**: the supported version is now
  [v5.27.0](https://github.com/bivlked/amneziawg-installer/releases/tag/v5.27.0)
  (`--json` contract verified: `manage_amneziawg.sh` is unchanged in
  v5.26.0/v5.27.0 — v5.26.0 only touches the cascade routing script and
  the diagnostic report, v5.27.0 only the installer); the minimum is
  still v5.21.0.

## [0.8.2] — 2026-08-03

### 🇷🇺 Русский

#### ✨ Добавлено

- **Кнопка «Клиенты группы»** в карточке группы: открывает список клиентов
  с фильтром по этой группе
  ([#20](https://github.com/ekuraev/awgram/issues/20)).

#### 🐛 Исправлено

- **Тупик фильтра «Без группы»**: когда все клиенты распределены по группам
  (или под липкий статус-фильтр никто не попал), раздел «Клиенты» больше не
  запирается на «клиентов нет» — экран пустой выборки сохраняет кнопки смены
  статус-фильтра и группового фильтра, а текст различает «клиентов нет
  вообще» и «под фильтр никто не попал»
  ([#20](https://github.com/ekuraev/awgram/issues/20)).

#### ♻️ Изменено

- **Совместимость с инсталлером**: поддерживаемая версия —
  [v5.23.0](https://github.com/bivlked/amneziawg-installer/releases/tag/v5.23.0)
  (сверен `--json`-контракт: v5.22.0 добавил предупреждение о рассинхроне
  `awgsetup_cfg.init` только в stderr, v5.23.0 меняет только установщики);
  минимальная — по-прежнему v5.21.0.

### 🇬🇧 English

#### ✨ Added

- **"Group clients" button** on the group card: opens the client list
  filtered to that group
  ([#20](https://github.com/ekuraev/awgram/issues/20)).

#### 🐛 Fixed

- **"No group" filter dead end**: when every client is assigned to a group
  (or the sticky status filter matches nobody), the "Clients" section no
  longer locks up on "no clients" — the empty-selection screen keeps the
  status-filter and group-filter buttons, and the text distinguishes
  "no clients at all" from "nothing matches the filter"
  ([#20](https://github.com/ekuraev/awgram/issues/20)).

#### ♻️ Changed

- **Installer compatibility**: the supported version is now
  [v5.23.0](https://github.com/bivlked/amneziawg-installer/releases/tag/v5.23.0)
  (`--json` contract verified: v5.22.0 adds an `awgsetup_cfg.init` drift
  warning to stderr only, v5.23.0 only changes the installers); the minimum
  stays v5.21.0.

## [0.8.1] — 2026-07-31

### 🇷🇺 Русский

#### 🐛 Исправлено

- **Гонка квоты группы**: при двух одновременных созданиях/переносах
  клиентов в одну группу квота могла быть превышена — проверка и привязка
  теперь атомарны; создание, проигравшее гонку, откатывается с сообщением
  «квота исчерпана» ([#20](https://github.com/ekuraev/awgram/issues/20)).
- **«Готово» после ошибки создания**: провал добавления клиента больше не
  завершается сообщением «Готово» — после ошибки бот возвращает главное
  меню ([#40](https://github.com/ekuraev/awgram/issues/40)).
- **Устойчивость к «Text file busy»**: запуск manage-скрипта теперь
  переживает короткое окно, когда файл открыт на запись (например,
  `awgram-setup update` переписывает скрипт под работающим ботом) —
  spawn ретраится до 200 мс вместо немедленной ошибки.

#### ♻️ Изменено

- **Централизованная авторизация кнопок**: все callback-действия проходят
  через единую таблицу доступа (владелец / групповой админ) — забытая
  проверка в новом действии теперь невозможна by construction.

### 🇬🇧 English

#### 🐛 Fixed

- **Group quota race**: two concurrent client creations/moves into the
  same group could exceed the quota — the check and the binding are now
  atomic; a creation that loses the race is rolled back with a
  "quota reached" message ([#20](https://github.com/ekuraev/awgram/issues/20)).
- **"Done" after a failed add**: a failed client creation no longer ends
  with a "Done" message — on error the bot now returns the main menu
  ([#40](https://github.com/ekuraev/awgram/issues/40)).
- **Resilience to "Text file busy"**: launching the manage script now
  survives a brief window when the file is open for writing (e.g.
  `awgram-setup update` rewriting the script under a running bot) — the
  spawn retries for up to 200 ms instead of failing immediately.

#### ♻️ Changed

- **Centralized button authorization**: every callback action now passes
  through a single access table (owner / group admin) — a forgotten check
  in a new action is impossible by construction.

## [0.8.0] — 2026-07-31

### 🇷🇺 Русский

#### ✨ Добавлено

- **Собственное SQLite-хранилище** (`rusqlite`, bundled): настройки,
  статистика трафика, история подключений и операций — вместо/поверх
  `state.json`, путь настраивается через `db_path` в конфиге.
- **Фоновый сборщик статистики**: тик раз в 60 с опрашивает `stats --json`,
  сохраняет сэмплы трафика и события online/offline, каждые 5 мин сворачивает
  сэмплы в часовые/дневные агрегаты с ретенцией.
- **Экран «Статистика»**: трафик за сегодня/7д/30д, тренд, среднее, топ
  клиентов по трафику — данные переживают ребут сервера.
- **Экран «История»** по каждому клиенту: подключения/отключения и операции
  (добавление, изменение, перевыпуск, удаление) с таймстампами.
- **Группы клиентов и делегирование**: групповые админы с доступом только к
  своей группе, одноразовые инвайт-ссылки (TTL 24 ч), квоты на группу,
  перенос клиентов между группами, массовый перевыпуск группы
  ([#20](https://github.com/ekuraev/awgram/issues/20)).

#### 🐛 Исправлено

- **Честный онлайн-статус**: клиент с хэндшейком старше 5 минут больше не
  показывается онлайн (ранее порог ошибочно достигал суток).
- **Цвет статуса клиентов в списке**: клиенты, никогда не подключавшиеся или
  давно отключившиеся, снова корректно показываются как 🟡, а не 🔴. Экран
  списка (#27) был переключён на `stats --json`, который не различает «никогда
  не подключался» и «был, но давно» — обоим он ставит `inactive` (🔴). Теперь
  `status_code` берётся из `list --json` (детальная классификация), а
  `last_handshake`/трафик — из `stats --json`.

#### ♻️ Изменено

- **Миграция `state.json` → SQLite** выполняется автоматически и один раз при
  первом запуске новой версии; старый файл не удаляется.
- **Каналы обновлений сокращены до `stable|rc`** — beta/alpha упразднены,
  не успев использоваться: `--channel beta|alpha` теперь отклоняется с
  ошибкой, а старое значение `CHANNEL=beta|alpha` в `setup.conf` молча
  трактуется как `stable`. В README и `awgram-setup help` зафиксировано,
  что каналы доступны начиная с v0.7.0, и добавлена инструкция обновления
  для серверов со скриптом старше v0.7.0.

### 🇬🇧 English

#### ✨ Added

- **Dedicated SQLite store** (`rusqlite`, bundled): settings, traffic
  statistics, connection and operation history — replacing/augmenting
  `state.json`; the path is configurable via `db_path` in the config.
- **Background stats collector**: a 60s tick polls `stats --json`, saves
  traffic samples and online/offline events, and every 5 min rolls samples
  up into hourly/daily aggregates with retention.
- **"Stats" screen**: traffic for today/7d/30d, trend, average, top clients
  by traffic — data survives server reboots.
- **Per-client "History" screen**: connections/disconnections and operations
  (add, modify, re-issue, delete) with timestamps.
- **Groups & delegation**: per-group admins scoped to their own group,
  one-time invite links (24 h TTL), per-group client quotas, moving clients
  between groups, group-wide regen
  ([#20](https://github.com/ekuraev/awgram/issues/20)).

#### 🐛 Fixed

- **Honest online status**: a client with a handshake older than 5 minutes
  is no longer shown as online (the threshold used to erroneously reach
  a full day).
- **Client status color in the list**: clients that never connected or
  disconnected long ago are correctly shown as 🟡 again, not 🔴. The list
  screen (#27) was switched to `stats --json`, which does not distinguish
  "never connected" from "was connected long ago" — both get `inactive` (🔴).
  Now `status_code` is taken from `list --json` (detailed classification),
  while `last_handshake`/traffic come from `stats --json`.

#### ♻️ Changed

- **`state.json` → SQLite migration** runs automatically, once, on first
  startup of the new version; the old file is not deleted.
- **Update channels narrowed to `stable|rc`** — beta/alpha removed before
  ever being used: `--channel beta|alpha` is now rejected with an error,
  and a legacy `CHANNEL=beta|alpha` value in `setup.conf` is silently
  treated as `stable`. README and `awgram-setup help` now state that
  channels are available since v0.7.0 and include upgrade instructions
  for servers running a pre-v0.7.0 script.

## [0.7.0] — 2026-07-29

### 🇷🇺 Русский

#### ✨ Добавлено

- **Каналы обновлений**: `awgram-setup update --channel stable|rc|beta|alpha` —
  установка предрелизных сборок на своём сервере. Канал запоминается;
  prerelease-каналы видят и стабильные релизы. Теги с суффиксом
  (например `v0.7.0-rc.1`) публикуются как GitHub prerelease и невидимы
  для обычного `update` на существующих установках.

### 🇬🇧 English

#### ✨ Added

- **Update channels**: `awgram-setup update --channel stable|rc|beta|alpha` —
  install pre-release builds on your own server. The channel is sticky;
  pre-release channels also see stable releases. Suffixed tags
  (e.g. `v0.7.0-rc.1`) are published as GitHub prereleases and stay
  invisible to plain `update` on existing installs.

## [0.6.0] — 2026-07-29

### 🇷🇺 Русский

#### ✨ Добавлено

- **Массовая генерация клиентов**: префикс + количество (1/3/5/10, cap 10 —
  лимит альбома Telegram). Один вызов инсталлера, выдача альбомом `.conf`.
  Превентивная проверка свободных адресов подсети и коллизий имён
  ([#22](https://github.com/ekuraev/awgram/issues/22)).
- **Фильтр выдачи после создания**: тумблеры `.conf` / QR / ссылка в
  настройках. Действует на одиночное и массовое добавление.
- **Карточка клиента**: отдельные кнопки для конфига, QR, ссылки и «всё»
  (раньше — одна кнопка «всё»).
- **Трёхцветная индикация статуса**: 🟢 активен/недавно, 🟡 нет handshake
  (никогда не подключался), 🔴 оффлайн/ошибка ключа — вместо прежнего
  бинарного «зелёный/красный». Время последнего handshake теперь прямо в
  кнопке списка клиентов; карточка перерисована в иконочном формате
  ([#21](https://github.com/ekuraev/awgram/issues/21)).
- **Фильтр и сортировка списка клиентов**: кнопки фильтра по статусу
  (Все / 🟢 Онлайн / 🔴 Оффлайн / 🟡 Никогда) и сортировка «онлайн вперёд»
  (🟢 → 🔴 → 🟡, внутри группы — по имени). Выбранный фильтр сохраняется
  между сессиями и отображается в заголовке списка
  ([#28](https://github.com/ekuraev/awgram/issues/28)).

### 🇬🇧 English

#### ✨ Added

- **Bulk client generation**: prefix + count (1/3/5/10, cap 10 — Telegram
  album limit). A single installer call, with configs delivered as an album
  of `.conf` files. Pre-emptive check of free subnet addresses and name
  collisions ([#22](https://github.com/ekuraev/awgram/issues/22)).
- **Post-creation delivery filter**: `.conf` / QR / link toggles in settings.
  Applies to both single and bulk addition.
- **Client card**: separate buttons for config, QR, link and "all"
  (previously a single "all" button).
- **Three-color status indicators**: 🟢 active/recent, 🟡 no handshake
  (never connected), 🔴 offline/key error — replacing the former binary
  "green/red". Last handshake time now shown directly in the client list
  button; the card was restyled to an icon-based layout
  ([#21](https://github.com/ekuraev/awgram/issues/21)).
- **Client list filter and sort**: status filter buttons
  (All / 🟢 Online / 🔴 Offline / 🟡 Never) and "online-first" sorting
  (🟢 → 🔴 → 🟡, by name within a group). The selected filter persists
  across sessions and is shown in the list title
  ([#28](https://github.com/ekuraev/awgram/issues/28)).

## [0.5.0] — 2026-07-28

### 🇷🇺 Русский

#### ✨ Добавлено

- Механика in-place-навигации (`editMessageText`) расширена на **все**
  экраны-меню: настройки и тумблеры, карточка клиента, статистика,
  бэкапы (меню/список/карточка), подтверждения (удаление/рестарт/рестор/
  перевыпуск) и выбор языка. Чат больше не плодит дубли ни при каком
  переходе по кнопкам — каждое меню живёт в одном сообщении
  (продолжение [#16](https://github.com/ekuraev/awgram/issues/16)).

### 🇬🇧 English

#### ✨ Added

- The in-place navigation (`editMessageText`) now covers **all** menu
  screens: settings and toggles, client card, stats, backups
  (menu/list/card), confirmations (delete/restart/restore/regen) and
  language selection. No transition through a button clutters the chat
  with duplicates anymore — every menu lives in a single message
  (follow-up to [#16](https://github.com/ekuraev/awgram/issues/16)).

## [0.4.0] — 2026-07-28

### 🇷🇺 Русский

#### ✨ Добавлено

- Навигация по меню/списку клиентов (меню ↔ список ↔ страницы) теперь
  обновляет сообщение на месте через `editMessageText`, а не отправляет
  новое — чат больше не захламляется копиями
  ([#16](https://github.com/ekuraev/awgram/issues/16)). Если исходное
  сообщение нельзя отредактировать (удалено и т.п.) — бот отправляет новое
  и снимает клавиатуру со старого, чтобы не висели две активные.
- Кнопка 🔄 «Обновить» в списке клиентов: перерисовывает актуальные статусы
  и метки срока действия в том же сообщении, сохраняя текущую страницу
  (актуально для списков длиннее одной страницы).

#### ♻️ Изменено

- Поддержка инсталлера расширена до
  [v5.21.2](https://github.com/bivlked/amneziawg-installer/releases/tag/v5.21.2)
  (минимум остался [v5.21.0](https://github.com/bivlked/amneziawg-installer/releases/tag/v5.21.0)).
  JSON-контракт не изменился — v5.21.1/v5.21.2 это багфиксы валидации
  (нормализация порта в `check`, числовые счётчики в `stats --json`),
  которые бот переваривает как есть.
- Обновление зависимостей: `rand` 0.9 → 0.10 (переход на свободную функцию
  `rand::random_range`), а также минорные бампы `regex`, `thiserror`,
  `tokio`, `serde`.

### 🇬🇧 English

#### ✨ Added

- Menu/clients navigation (menu ↔ list ↔ pages) now updates the message
  in place via `editMessageText` instead of sending a new one — the chat
  no longer gets cluttered with duplicate copies
  ([#16](https://github.com/ekuraev/awgram/issues/16)). If the source
  message can't be edited (deleted, etc.), the bot sends a new one and
  clears the old keyboard so two active ones never sit side by side.
- 🔄 "Refresh" button in the clients list: redraws current statuses and
  expiry badges in the same message, keeping the current page (relevant
  for lists longer than a single page).

#### ♻️ Changed

- Installer support extended to
  [v5.21.2](https://github.com/bivlked/amneziawg-installer/releases/tag/v5.21.2)
  (minimum remains [v5.21.0](https://github.com/bivlked/amneziawg-installer/releases/tag/v5.21.0)).
  The JSON contract is unchanged — v5.21.1/v5.21.2 are validation bugfixes
  (port normalisation in `check`, numeric counters in `stats --json`) that
  the bot handles as-is.
- Dependency updates: `rand` 0.9 → 0.10 (moved to the `rand::random_range`
  free function), plus minor bumps of `regex`, `thiserror`, `tokio`, `serde`.

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

[0.8.1]: https://github.com/ekuraev/awgram/compare/v0.8.0...v0.8.1
[0.8.0]: https://github.com/ekuraev/awgram/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/ekuraev/awgram/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/ekuraev/awgram/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/ekuraev/awgram/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/ekuraev/awgram/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/ekuraev/awgram/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/ekuraev/awgram/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/ekuraev/awgram/releases/tag/v0.1.0
