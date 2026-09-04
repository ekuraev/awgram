# Changelog

Формат — [Keep a Changelog](https://keepachangelog.com/ru/1.1.0/), версионирование — [SemVer](https://semver.org/lang/ru/).
Format — [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), versioning — [SemVer](https://semver.org/).

## [0.11.0] — 2026-09-05

### 🇷🇺 Русский

#### ✨ Добавлено

- **Режим «исключать из VPN» на экране AllowedIPs.** Переключатель режима
  меняет смысл тумблеров: весь трафик идёт в туннель, а отмеченные сети —
  мимо него, так что клиент остаётся доступен в своей локальной сети.
  WireGuard не знает исключений, поэтому бот сам считает дополнение
  `0.0.0.0/0` минус выбранные сети и всегда возвращает в список подсеть VPN
  (иначе исключение `10.0.0.0/8` отрезало бы сам туннель). В заголовке
  показывается сводка «всё, кроме …», при правке такое значение распознаётся
  и предзаполняет тумблеры.
- **Узкие пресеты локальных сетей**: к трём диапазонам RFC 1918 добавлены
  типовые /24 домашних роутеров — `10.0.0.0/24`, `10.0.1.0/24`,
  `192.168.0.0/24`, `192.168.1.0/24`, `192.168.10.0/24`, `192.168.100.0/24`.
  Работают в обоих режимах.

### 🇬🇧 English

#### ✨ Added

- **"Exclude from VPN" mode on the AllowedIPs screen.** The mode switch flips
  the meaning of the toggles: all traffic goes through the tunnel and the
  marked networks bypass it, so the client stays reachable on its LAN.
  WireGuard has no notion of exclusions, so the bot computes the complement
  `0.0.0.0/0` minus the selected networks itself and always puts the VPN
  subnet back (otherwise excluding `10.0.0.0/8` would cut the tunnel itself).
  The title shows an "all except …" summary; when editing, such a value is
  recognised and pre-fills the toggles.
- **Narrow local-network presets**: besides the three RFC 1918 ranges, the
  typical home-router /24s — `10.0.0.0/24`, `10.0.1.0/24`, `192.168.0.0/24`,
  `192.168.1.0/24`, `192.168.10.0/24`, `192.168.100.0/24`. Available in both
  modes.

## [0.10.1] — 2026-09-04

### 🇷🇺 Русский

#### 🐛 Исправлено

- **Ширина инлайн-кнопок больше не «плавает» между экранами.** Клиент Telegram
  подгоняет клавиатуру под ширину пузыря сообщения: короткий заголовок давал
  широкое меню, а многострочный текст (статистика, карточка) — узкое, с
  обрезанными подписями вроде «Поч…модуля». Теперь к тексту каждого экрана с
  клавиатурой добавляется невидимая строка-распорка, и пузырь с кнопками везде
  одной ширины.

### 🇬🇧 English

#### 🐛 Fixed

- **Inline button width no longer drifts between screens.** Telegram clients
  size the keyboard to the message bubble: a short title produced a wide menu,
  while multi-line text (stats, client card) squeezed it and truncated labels.
  Every screen with a keyboard now carries an invisible spacer line, so the
  bubble and buttons have the same width everywhere.

## [0.10.0] — 2026-09-03

### 🇷🇺 Русский

#### ✨ Добавлено

- **AllowedIPs при создании и правке клиента**: новый шаг диалога с готовыми
  пресетами — три диапазона RFC 1918 по отдельности или разом, подсеть VPN
  (берётся из `check`), полный туннель `0.0.0.0/0, ::/0` и ручной ввод CIDR.
  Кнопка «⏭ Как на сервере» оставляет глобальный режим маршрутизации
  инсталлера, как было раньше. «Изменить → AllowedIPs» открывает тот же экран
  с тумблерами, предзаполненными текущим значением клиента. При пересоздании
  тумблеры тоже предзаполняются: раньше `remove` + `add` молча сбрасывал
  индивидуальные маршруты к серверным.
- **Маршруты у массовой генерации**: тот же экран после выбора PSK, значение
  применяется ко всей пачке.
- **Одна команда вместо двух, если инсталлер это умеет.** Бот спрашивает у
  скрипта его справку и, увидев `add --allowed-ips`, создаёт клиента сразу с
  нужными маршрутами. Тогда не бывает промежуточного состояния «клиент есть,
  маршруты ещё нет», а `.conf`, QR и ссылка сразу верные. Инсталлер без флага
  обслуживается прежним путём: `add`, затем `modify` до выдачи файлов.
  Развилка идёт по ответу скрипта, а не по номеру версии: неизвестную опцию
  он отвергает, ничего не создавая, поэтому откат безопасен. У массовой
  генерации отката нет — там `modify` пришлось бы звать на каждого клиента,
  поэтому на старом инсталлере шага маршрутов просто нет.
- **Умные бэкапы** (#35, #53): автобэкап по расписанию (ежедневно / еженедельно /
  ежемесячно, время из пресетов), ротация «хранить N» с закреплением важных копий,
  отчёты владельцам об успехе и сбое; при сбое бот повторяет попытки с
  нарастающим интервалом (до суток) и напоминает не чаще раза в 6 часов, пока
  бэкап не пройдёт. Список с датой, размером и комментарием, карточка с
  проверкой целостности по SHA-256, удаление, восстановление из карточки,
  загрузка бэкапа файлом через Telegram.
- **Бандл с БД бота**: бот хранит бэкапы в `backups/awgram/` как
  `awgram_backup_<ts>.tar.gz` — архив инсталлера байт в байт, `meta.json` и, если
  включено в настройках, снимок БД бота (группы, инвайты, статистика). При
  восстановлении можно выбрать «AWG и БД бота» или «только AWG». Чистые архивы
  инсталлера принимаются и оборачиваются в бандл.
- **Комментарии к бэкапам** (#53): опциональный вопрос при создании, правка и
  очистка с карточки, показ в списке.

#### 🔧 Изменено

- **Чат больше не копит сообщения**: ответы на нажатия кнопок редактируют
  сообщение-источник, а не добавляют новое. Это касается подтверждений
  (удаление, перевыпуск, рестарт, восстановление), результатов проверок
  `check`/`diagnose`, операций с группами и бэкапами. Индикаторы «⏳» больше
  не отдельные сообщения: они рисуются на месте нажатой кнопки. Итог операций
  с файлами уходит вниз, под выданные `.conf`/QR, взамен удалённого
  индикатора — клавиатура остаётся под файлами, а число сообщений не растёт.
  Вопросы с текстовым вводом (имя клиента, свой срок, имя/лимит группы, ID
  админа, свой CIDR) тоже занимают место меню и получают кнопку «◀️ Назад»;
  ответ бота перерисовывает сам вопрос.

- **Совместимость с инсталлером**: поддерживаемая версия —
  [v5.31.0](https://github.com/bivlked/amneziawg-installer/releases/tag/v5.31.0)
  (сверен `--json`-контракт: v5.27.1 нормализует списки `AllowedIPs`/`DNS`
  в `modify`/`regen` к виду «a, b, c» и чинит `regen`, схлопывавший эти
  списки; v5.28.0–v5.29.0 меняют только установщик, подписи релизов и
  документацию; v5.30.0 ограничил обращения к интерфейсу таймаутом и при
  неудачном чтении оставляет клиентам `no_data` вместо «нет handshake» —
  бот этот статус уже понимает и красит жёлтым, а пустой
  `interface.addresses` в `check --json` просто прячет пресет подсети VPN;
  v5.31.0 определяет полный туннель покрытием маршрутов и предупреждает,
  когда `modify` получает полный туннель без `::/0` — пресет «весь трафик»
  шлёт `0.0.0.0/0, ::/0`, так что предупреждение не срабатывает. Конверты
  `--json` не изменились, новые сообщения уходят в stderr); минимальная —
  по-прежнему v5.21.0.
- **Старые архивы в `backups/`** показываются как «снапшоты инсталлера»:
  восстановить и удалить можно, комментария и пина у них нет.
- **Hardened-режим**: `install.sh` выдаёт пользователю `awgram` права на
  `backups/`; архивы самого инсталлера (`chmod 600`) для него нечитаемы,
  см. README.

### 🇬🇧 English

#### ✨ Added

- **AllowedIPs at client creation and edit**: a new dialog step with ready-made
  presets — the three RFC 1918 ranges separately or all at once, the VPN subnet
  (taken from `check`), the full tunnel `0.0.0.0/0, ::/0`, and manual CIDR
  entry. The "⏭ Server default" button keeps the installer's global routing
  mode, exactly as before. "Modify → AllowedIPs" opens the same screen with the
  toggles prefilled from the client's current value. Recreating a client
  prefills them too: `remove` + `add` used to reset individual routes to the
  server defaults silently.
- **Routes for bulk creation**: the same screen after the PSK step, applied to
  the whole batch.
- **One command instead of two where the installer supports it.** The bot reads
  the script's own help and, seeing `add --allowed-ips`, creates the client
  with the right routes in a single call. There is then no in-between state
  where the client exists but its routes do not, and the `.conf`, QR and link
  are correct from the start. An installer without the flag is served the old
  way: `add`, then `modify` before delivery. The fork follows the script's
  answer rather than a version number: an unknown option is rejected without
  creating anything, which makes the fallback safe. Bulk creation has no
  fallback, because `modify` would have to run once per client, so on an older
  installer the routes step is simply not offered.
- **Smart backups** (#35, #53): scheduled auto-backup (daily / weekly /
  monthly, time from presets), "keep N" rotation with pinning for important
  copies, owner reports on success and failure; on failure the bot retries
  with increasing intervals (up to a day) and reminds at most every 6 hours
  until a backup succeeds. A list with date, size and comment, a card with
  SHA-256 integrity verification, deletion, restore from the card, backup
  download as a file via Telegram.
- **Bundle with the bot's database**: the bot stores backups in
  `backups/awgram/` as `awgram_backup_<ts>.tar.gz` — the installer's archive
  byte for byte, a `meta.json`, and, if enabled in settings, a snapshot of the
  bot's own database (groups, invites, stats). Restore offers a choice of
  "AWG and the bot's DB" or "AWG only". Plain installer archives are accepted
  and wrapped into a bundle.
- **Comments on backups** (#53): an optional prompt at creation time, editing
  and clearing from the card, shown in the list.

#### 🔧 Changed

- **The chat no longer piles up messages**: replies to button presses edit the
  source message instead of adding a new one. This covers confirmations
  (delete, re-issue, restart, restore), `check`/`diagnose` reports, and group
  and backup operations. "⏳" indicators are no longer separate messages: they
  are drawn in place of the pressed button. The result of file-producing
  operations goes below the delivered `.conf`/QR files, replacing the removed
  indicator — the keyboard stays under the files and the message count does not
  grow. Text prompts (client name, custom expiry, group name/quota, admin ID,
  custom CIDR) also take over the menu message and carry a "◀️ Back" button;
  the bot's reply redraws the prompt itself.

- **Installer compatibility**: the supported version is now
  [v5.31.0](https://github.com/bivlked/amneziawg-installer/releases/tag/v5.31.0)
  (`--json` contract verified: v5.27.1 normalizes the `AllowedIPs`/`DNS`
  lists in `modify`/`regen` to the canonical "a, b, c" form and fixes
  `regen` collapsing those lists; v5.28.0–v5.29.0 only touch the installer,
  release signing and docs; v5.30.0 bounds interface calls by a timeout and
  on a failed read leaves clients at `no_data` instead of "no handshake" —
  the bot already understands that status and marks it yellow, and an empty
  `interface.addresses` in `check --json` simply hides the VPN subnet
  preset; v5.31.0 decides a full tunnel by route coverage and warns when
  `modify` is handed a full tunnel without `::/0` — the "all traffic" preset
  sends `0.0.0.0/0, ::/0`, so that warning never fires. The `--json`
  envelopes are unchanged and new messages go to stderr); the minimum is
  still v5.21.0.
- **Old archives in `backups/`** are shown as "installer snapshots": they can
  be restored and deleted, but have no comment or pin.
- **Hardened mode**: `install.sh` grants the `awgram` user access to
  `backups/`; the installer's own archives (`chmod 600`) remain unreadable to
  it, see the README.

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

[0.11.0]: https://github.com/ekuraev/awgram/compare/v0.10.1...v0.11.0
[0.10.1]: https://github.com/ekuraev/awgram/compare/v0.10.0...v0.10.1
[0.10.0]: https://github.com/ekuraev/awgram/compare/v0.9.0...v0.10.0
[0.9.0]: https://github.com/ekuraev/awgram/compare/v0.8.2...v0.9.0
[0.8.2]: https://github.com/ekuraev/awgram/compare/v0.8.1...v0.8.2
[0.8.1]: https://github.com/ekuraev/awgram/compare/v0.8.0...v0.8.1
[0.8.0]: https://github.com/ekuraev/awgram/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/ekuraev/awgram/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/ekuraev/awgram/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/ekuraev/awgram/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/ekuraev/awgram/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/ekuraev/awgram/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/ekuraev/awgram/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/ekuraev/awgram/releases/tag/v0.1.0
