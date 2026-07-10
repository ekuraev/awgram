# AmneziaWG Telegram Bot — фичи v2 — дизайн

**Дата:** 2026-07-10
**Статус:** утверждён к реализации
**Базируется на:** [дизайн v1](2026-07-10-awg-telegram-bot-design.md) (реализован, в `main`)

## 1. Цель

Добавить в существующий бот пять фич:

1. **Язык RU/EN** — выбирается при первом `/start`, меняется позже в настройках. Язык хранится **по каждому админу**.
2. **PresharedKey (PSK)** — опционально при добавлении клиента. Глобальный дефолт в настройках бота + переопределение для конкретного клиента.
3. **Backup** — создать бэкап, опционально скачать архив, посмотреть список существующих.
4. **Restore** — восстановление из бэкапов, лежащих на сервере (выбор из списка).
5. **check/status** — диагностика сервера прямо из бота.

Плюс сквозная работа над **форматированием сообщений** (переход на HTML parse mode).

Всё аддитивно к v1: `manage_amneziawg.sh` уже поддерживает `add --psk`, `backup`,
`restore <путь>`, `check|status` (проверено по исходнику скрипта).

### Не входит в объём (YAGNI)
- Загрузка файла бэкапа в бота для restore (только restore из бэкапов на сервере).
- Языки, кроме RU/EN.
- PSK как per-admin настройка (PSK-дефолт — **глобальный**).
- Локализация серверного вывода `check` (он приходит от скрипта как есть).

## 2. Ключевые решения

| Решение | Выбор | Обоснование |
|---|---|---|
| Хранение языка | JSON-файл, ключ `user_id` | Просто, без БД; язык переживает рестарт |
| Дефолт PSK | Глобально (в том же state.json) | Одна политика на бота; меняется в настройках |
| i18n-механизм | Функции модуля `i18n`, `match lang` | Локаль поклиентная; компиляторная гарантия полноты; без внешних крейтов |
| Restore | Из бэкапов на сервере (по индексу) | Просто, безопасно, без загрузки файлов |
| Backup-скачивание | По запросу (кнопка «Скачать») | Не спамить архивом автоматически |
| check код выхода 1 | Показать вывод, не считать ошибкой | Код 1 = «найдены проблемы», это результат |
| Форматирование | HTML parse mode | Надёжное экранирование (3 спецсимвола), решает проблему URI |

## 3. Архитектура

Дополняет v1 двумя опорными модулями (персистентные настройки, i18n) и
расширяет слои `vpn/` и `bot/`. Регистрация зависимостей — через `dptree::deps!`.

### Новые/изменённые файлы

```
src/
  settings.rs      — НОВЫЙ: SettingsStore (персистентный state.json)
  i18n.rs          — НОВЫЙ: Lang + локализованные сообщения + error_text
  config.rs        — +поле state_file
  error.rs         — без изменений (варианты те же)
  vpn/mod.rs       — +backup/list_backups/restore/check; add(psk); BackupFile/BackupInfo
  vpn/runner.rs    — +run_capture (stdout независимо от кода выхода)
  bot/mod.rs       — State: +новые диалоговые состояния; schema без структурных изменений
  bot/menu.rs      — +клавиатуры: язык, настройки, бэкапы, PSK-шаг
  bot/render.rs    — HTML-рендер, html_escape; рендер использует i18n
  bot/handlers.rs  — язык-гейт, настройки, PSK-шаг, backup/restore/check; новые callback
  main.rs          — загрузка SettingsStore, регистрация в deps!
```

### Принцип изоляции
- `settings.rs` и `i18n.rs` не знают про Telegram.
- `vpn/` не знает про Telegram и про язык.
- Локализация и HTML-разметка сосредоточены в `i18n.rs` + `bot/render.rs`.
- Бизнес-логика (Vpn, SettingsStore, i18n) тестируется без Telegram API.

## 4. Персистентные настройки (`src/settings.rs`)

```rust
#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Lang { Ru, En }               // Default = Ru

#[derive(Default, Serialize, Deserialize)]
pub struct BotState {
    pub psk_default: bool,             // ГЛОБАЛЬНО
    pub langs: HashMap<i64, Lang>,     // язык по user_id админа
}

pub struct SettingsStore {
    path: PathBuf,
    state: Mutex<BotState>,
}

impl SettingsStore {
    pub fn load(path: PathBuf) -> Self;          // читает файл или пустой BotState
    pub fn lang(&self, uid: i64) -> Lang;        // Ru по умолчанию
    pub fn has_lang(&self, uid: i64) -> bool;
    pub fn set_lang(&self, uid: i64, lang: Lang);// мутирует + persist
    pub fn psk_default(&self) -> bool;           // глобально
    pub fn set_psk_default(&self, v: bool);      // глобально + persist
}
```
- Оборачивается в `Arc`, регистрируется в `deps!`.
- Потокобезопасно (`Mutex`) — teloxide обрабатывает апдейты конкурентно.
- Запись атомарная: сериализация в `path.tmp` → `rename` в `path`.
- Ошибку записи логируем (`tracing::error`), но не роняем бота (настройка в памяти уже применена).
- Путь: новое поле конфига `state_file` (дефолт `/etc/awg-bot/state.json`).

## 5. i18n (`src/i18n.rs`)

- `Lang` (из `settings.rs`, реэкспорт или общий тип).
- Каждое пользовательское сообщение — функция `fn(lang, args…) -> String`, внутри `match lang`, интерполяция через `format!`.
- **Соглашение:** динамические пользовательские/серверные данные (имена, IP, URI,
  вывод `check`) **экранируются** (`html_escape`) внутри i18n/render перед вставкой в HTML.
- `pub fn error_text(lang: Lang, err: &Error) -> String` — локализованный текст ошибки
  (маппинг варианта `Error` → строка). Заменяет `Error::user_message()` в хендлерах.
  Сам `Error::user_message()` остаётся (используется как fallback/для логов).

Пример состава (не исчерпывающий):
```rust
pub fn choose_language() -> String;               // без lang: показывает обе
pub fn menu_title(lang) -> String;
pub fn btn_clients(lang) -> String; /* … кнопки … */
pub fn access_denied(lang) -> String;
pub fn ask_client_name(lang) -> String;
pub fn ask_expiry(lang) -> String;
pub fn psk_step(lang, enabled: bool) -> String;
pub fn client_card(lang, name, status, ip, rx, tx, handshake, expires) -> String;
pub fn stats_summary(lang, total, active, rx, tx) -> String;
pub fn backup_done(lang, filename) -> String;
pub fn backups_list_title(lang) -> String;
pub fn confirm_restore(lang, filename) -> String;
pub fn restore_done(lang) -> String;
pub fn check_result(lang, body) -> String;         // body в <pre>
pub fn settings_title(lang, psk_default: bool) -> String;
pub fn error_text(lang, err: &Error) -> String;
```

## 6. UX и сценарии

### Выбор языка
`/start`: если `!has_lang(uid)` → экран выбора (`🇷🇺 Русский` / `🇬🇧 English`,
callback `lang:ru` / `lang:en`) → сохранить → меню. Иначе сразу меню.

### Главное меню (локализованное)
```
🔐 AmneziaWG
├─ 👥 Клиенты / Clients        (list)
├─ ➕ Добавить / Add           (add)
├─ 📊 Статистика / Stats       (stats)
├─ 💾 Бэкап / Backup           (backup)
├─ 🩺 Проверка / Check         (check)
└─ ⚙️ Настройки / Settings     (settings)
```

### ⚙️ Настройки
```
Язык / Language:  🇷🇺 / 🇬🇧        → set:lang:ru | set:lang:en  (per-admin)
PSK по умолчанию: [вкл/выкл]       → set:psk:on | set:psk:off   (глобально)
```
Тап переключает и сохраняет в state.json.

### ➕ Добавить (диалог)
Имя → срок → **шаг PSK**: показывается «PSK: <из глобального дефолта>» с кнопкой
переключения для этого клиента (`add:psk:on` / `add:psk:off`) → создание.
При включённом PSK: `add <имя> [--expires=…] --psk`.

### 💾 Бэкап
- «Создать» (`bk:new`) → `backup` → «Готово: <файл>» + кнопка «📥 Скачать» (`bk:dl:<idx>`).
- «Список» (`bk:list`) → архивы из `clients_dir/backups/` кнопками (по индексу);
  карточка бэкапа: «📥 Скачать» (`bk:dl:<idx>`) и «♻️ Восстановить» (`bk:restore:<idx>`).
- «♻️ Восстановить» → подтверждение (`bk:restore_yes:<idx>`) → `restore <путь>`.

### 🩺 Проверка
`check` (`check`) → вывод `check_server()` в `<pre>` + итог OK/проблемы (по коду выхода).

### Новые callback-data
`lang:ru`, `lang:en`, `settings`, `set:lang:ru`, `set:lang:en`, `set:psk:on`,
`set:psk:off`, `add:psk:on`, `add:psk:off`, `backup`, `bk:new`, `bk:list`,
`bk:dl:<idx>`, `bk:restore:<idx>`, `bk:restore_yes:<idx>`, `check`.
Бэкапы адресуются **по индексу** свежего `list_backups()` (короткая и безопасная callback-data).

## 7. Слой Vpn (`src/vpn/mod.rs`, `runner.rs`)

```rust
pub struct BackupFile { pub name: String, pub path: PathBuf, pub size: u64, pub mtime: i64 }

impl Vpn {
    pub async fn add(&self, name, expires: Option<&str>, psk: bool) -> Result<AddResult>;
    pub async fn backup(&self) -> Result<BackupFile>;      // run backup → новейший *.tar.gz
    pub fn list_backups(&self) -> Result<Vec<BackupFile>>; // clients_dir/backups/, сорт по mtime desc
    pub async fn restore(&self, index: usize) -> Result<()>;// list_backups()[index] → restore <path>
    pub async fn check(&self) -> Result<String>;           // run_capture: stdout независимо от кода
}
```
- `runner::run_capture(spec, args) -> Result<(String /*stdout*/, i32 /*code*/)>` — не считает
  ненулевой код ошибкой; тайм-аут по-прежнему `Error::Timeout`. Нужен для `check`.
- `backup()`: запускает `backup` через обычный `run` (ненулевой код = ошибка), затем берёт
  новейший файл из `clients_dir/backups/`.
- `restore(index)`: `list_backups()` → берёт `[index]` (out-of-range → `Error::Parse`),
  имя файла дополнительно валидируется (basename, шаблон `awg_backup_*.tar.gz`) перед `restore`.

## 8. HTML-форматирование

- Все `send_message` с форматированием — `.parse_mode(ParseMode::Html)`.
- `html_escape(&str)` (`&`→`&amp;`, `<`→`&lt;`, `>`→`&gt;`) для всех динамических данных.
- URI — `<code>`, вывод `check` — `<pre>`, заголовки — `<b>`.
- Клиентские `.conf`/QR/архивы уходят документами — экранирование не требуется.
- Решает старую проблему MarkdownV2-экранирования URI (снимает открытый пункт из v1).

## 9. Обработка ошибок

- Ядро `Error` без изменений.
- В хендлерах пользователю показывается `i18n::error_text(lang, &e)` (локализовано).
- Ошибка записи state.json логируется, но не роняет бота.
- `check` с кодом 1 — не ошибка: вывод показывается, итог помечается «проблемы».
- Каждый хендлер возвращает `Result`, ошибка операции не роняет диспетчер.

## 10. Тестирование

- **SettingsStore:** round-trip load/save (temp-файл), дефолты (Ru, psk=false),
  per-user язык, глобальный psk, атомарная запись (tmp+rename).
- **i18n:** для набора ключевых сообщений обе локали дают непустую строку;
  `error_text` покрывает все варианты `Error`.
- **html_escape:** юнит-тесты, включая `<script>`, `&`, смешанное.
- **Vpn (скрипты-заглушки):** `add --psk` (заглушка проверяет наличие флага),
  `backup` (создаёт файл → возвращает путь/новейший), `list_backups` (temp-каталог,
  сортировка), `restore(index)` (валидный/невалидный индекс), `check` (заглушка
  с кодом 1 → вывод всё равно возвращается).
- **parse_callback:** новые action-ы; round-trip menu↔parse_callback остаётся зелёным.
- Хендлеры тонкие; бизнес-логика тестируется без Telegram API.

## 11. Совместимость и деплой
- Новое поле конфига `state_file` (дефолт `/etc/awg-bot/state.json`) — при отсутствии
  файла создаётся пустой state (все дефолты). Существующий config.toml остаётся валидным.
- Бинарник — та же статическая musl-сборка (`scripts/build-musl.sh`).
- README: описать язык/настройки/PSK/backup/restore/check и новое поле `state_file`.
