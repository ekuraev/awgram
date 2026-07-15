use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Lang {
    #[default]
    Ru,
    En,
}

pub fn parse_lang(code: &str) -> Option<Lang> {
    match code {
        "ru" => Some(Lang::Ru),
        "en" => Some(Lang::En),
        _ => None,
    }
}

pub fn lang_code(l: Lang) -> &'static str {
    match l {
        Lang::Ru => "ru",
        Lang::En => "en",
    }
}

pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

use crate::error::Error;

// --- экран выбора языка (без lang: показывает оба варианта) ---
pub fn choose_language() -> String {
    "🌐 Выберите язык / Choose language:".to_string()
}

// --- меню ---
pub fn menu_title(lang: Lang) -> String {
    match lang {
        Lang::Ru => "🔐 <b>AmneziaWG</b>",
        Lang::En => "🔐 <b>AmneziaWG</b>",
    }
    .to_string()
}
pub fn btn_clients(lang: Lang) -> String {
    match lang {
        Lang::Ru => "👥 Клиенты",
        Lang::En => "👥 Clients",
    }
    .to_string()
}
pub fn btn_add(lang: Lang) -> String {
    match lang {
        Lang::Ru => "➕ Добавить",
        Lang::En => "➕ Add",
    }
    .to_string()
}
pub fn btn_stats(lang: Lang) -> String {
    match lang {
        Lang::Ru => "📊 Статистика",
        Lang::En => "📊 Stats",
    }
    .to_string()
}
pub fn btn_backup(lang: Lang) -> String {
    match lang {
        Lang::Ru => "💾 Бэкап",
        Lang::En => "💾 Backup",
    }
    .to_string()
}
pub fn btn_check(lang: Lang) -> String {
    match lang {
        Lang::Ru => "🩺 Проверка",
        Lang::En => "🩺 Check",
    }
    .to_string()
}
pub fn btn_settings(lang: Lang) -> String {
    match lang {
        Lang::Ru => "⚙️ Настройки",
        Lang::En => "⚙️ Settings",
    }
    .to_string()
}
pub fn btn_back(lang: Lang) -> String {
    match lang {
        Lang::Ru => "⬅️ В меню",
        Lang::En => "⬅️ Menu",
    }
    .to_string()
}

pub fn access_denied(lang: Lang) -> String {
    match lang {
        Lang::Ru => "⛔ Доступ запрещён.",
        Lang::En => "⛔ Access denied.",
    }
    .to_string()
}

/// Уведомление для не-приватных чатов (группы/супергруппы/каналы). Бот
/// доставляет секреты (конфиги, QR, ссылки импорта, бэкапы, диагностику check)
/// в чат, откуда пришёл апдейт — авторизация же идёт по user_id, поэтому в
/// группе секреты могут утечь всем участникам. Строка билингвальна, т.к. язык
/// пользователя на этом этапе может быть ещё не определён (не-админ/новый чат).
pub fn private_only() -> String {
    "🔒 Бот работает только в личном чате. / Bot works only in a private chat.".to_string()
}

// --- add-диалог ---
pub fn ask_client_name(lang: Lang, slug_on: bool) -> String {
    match (lang, slug_on) {
        (Lang::Ru, true) => {
            "Введите имя клиента.\n• пробелы будут автоматически заменены на «-»\n• ID-префикс: вкл — к имени добавится уникальный префикс (например k3x9f-name)"
        }
        (Lang::Ru, false) => {
            "Введите имя клиента.\n• пробелы будут автоматически заменены на «-»\n• ID-префикс: выкл"
        }
        (Lang::En, true) => {
            "Enter client name.\n• spaces are replaced with \"-\" automatically\n• ID prefix: on — a unique prefix will be added (e.g. k3x9f-name)"
        }
        (Lang::En, false) => {
            "Enter client name.\n• spaces are replaced with \"-\" automatically\n• ID prefix: off"
        }
    }
    .to_string()
}
pub fn bad_name(lang: Lang, slug_on: bool) -> String {
    let max = if slug_on { "1–26" } else { "1–32" };
    match lang {
        Lang::Ru => {
            format!("⚠️ Некорректное имя (латиница/цифры/пробел/-/_, {max}). Введите ещё раз:")
        }
        Lang::En => format!("⚠️ Invalid name (a-z0-9 space -_, {max}). Try again:"),
    }
}
pub fn ask_expiry(lang: Lang) -> String {
    match lang {
        Lang::Ru => "Выберите срок действия:",
        Lang::En => "Choose expiry:",
    }
    .to_string()
}
pub fn ask_custom_expiry(lang: Lang) -> String {
    match lang {
        Lang::Ru => "Введите срок (например 10d, 12h, 3w):",
        Lang::En => "Enter duration (e.g. 10d, 12h, 3w):",
    }
    .to_string()
}
pub fn bad_expiry(lang: Lang) -> String {
    match lang {
        Lang::Ru => "⚠️ Формат срока: Nh/Nd/Nw (например 10d).",
        Lang::En => "⚠️ Duration format: Nh/Nd/Nw (e.g. 10d).",
    }
    .to_string()
}
pub fn psk_step(lang: Lang, default_on: bool) -> String {
    let d = if default_on {
        "вкл/on"
    } else {
        "выкл/off"
    };
    match lang {
        Lang::Ru => format!("PresharedKey (по умолчанию: {d}). Создать клиента:"),
        Lang::En => format!("PresharedKey (default: {d}). Create client:"),
    }
}
pub fn btn_create_with_psk(lang: Lang) -> String {
    match lang {
        Lang::Ru => "🔐 С PSK",
        Lang::En => "🔐 With PSK",
    }
    .to_string()
}
pub fn btn_create_no_psk(lang: Lang) -> String {
    match lang {
        Lang::Ru => "🔓 Без PSK",
        Lang::En => "🔓 No PSK",
    }
    .to_string()
}
pub fn creating(lang: Lang) -> String {
    match lang {
        Lang::Ru => "⏳ Создаю клиента…",
        Lang::En => "⏳ Creating client…",
    }
    .to_string()
}
pub fn import_link(lang: Lang, uri: &str) -> String {
    let u = html_escape(uri);
    match lang {
        Lang::Ru => format!("🔗 Ссылка для импорта:\n<code>{u}</code>"),
        Lang::En => format!("🔗 Import link:\n<code>{u}</code>"),
    }
}

// --- карточка/статистика (динамика экранируется) ---
#[allow(clippy::too_many_arguments)]
pub fn client_card(
    lang: Lang,
    name: &str,
    status: &str,
    ip: &str,
    rx: &str,
    tx: &str,
    handshake: &str,
    expires: &str,
) -> String {
    let (name, status, ip) = (html_escape(name), html_escape(status), html_escape(ip));
    let ip_line = if ip.is_empty() {
        String::new()
    } else {
        match lang {
            Lang::Ru => format!("IP: {ip}\n"),
            Lang::En => format!("IP: {ip}\n"),
        }
    };
    match lang {
        Lang::Ru => format!("👤 <b>{name}</b>\nСтатус: {status}\n{ip_line}Трафик:  ↓ {rx}   ↑ {tx}\nРукопожатие: {handshake}\nДействует: {expires}"),
        Lang::En => format!("👤 <b>{name}</b>\nStatus: {status}\n{ip_line}Traffic:  ↓ {rx}   ↑ {tx}\nHandshake: {handshake}\nExpires: {expires}"),
    }
}
pub fn stats_summary(lang: Lang, total: usize, active: usize, rx: &str, tx: &str) -> String {
    match lang {
        Lang::Ru => format!("📊 <b>Статистика</b>\nВсего клиентов: {total}\nАктивных: {active}\nТрафик суммарно: ↓ {rx}  ↑ {tx}"),
        Lang::En => format!("📊 <b>Stats</b>\nTotal clients: {total}\nActive: {active}\nTotal traffic: ↓ {rx}  ↑ {tx}"),
    }
}
pub fn clients_empty(lang: Lang) -> String {
    match lang {
        Lang::Ru => "Пока нет клиентов.",
        Lang::En => "No clients yet.",
    }
    .to_string()
}
pub fn clients_title(lang: Lang) -> String {
    match lang {
        Lang::Ru => "👥 <b>Клиенты</b>:",
        Lang::En => "👥 <b>Clients</b>:",
    }
    .to_string()
}
pub fn not_found(lang: Lang) -> String {
    match lang {
        Lang::Ru => "Клиент не найден.",
        Lang::En => "Client not found.",
    }
    .to_string()
}
pub fn backup_not_found(lang: Lang) -> String {
    match lang {
        Lang::Ru => "Бэкап не найден.",
        Lang::En => "Backup not found.",
    }
    .to_string()
}
pub fn confirm_delete(lang: Lang, name: &str) -> String {
    let n = html_escape(name);
    match lang {
        Lang::Ru => format!("Точно удалить <b>{n}</b>?"),
        Lang::En => format!("Delete <b>{n}</b>?"),
    }
}
pub fn deleted(lang: Lang, name: &str) -> String {
    let n = html_escape(name);
    match lang {
        Lang::Ru => format!("🗑 Клиент {n} удалён."),
        Lang::En => format!("🗑 Client {n} removed."),
    }
}
pub fn done(lang: Lang) -> String {
    match lang {
        Lang::Ru => "Готово.",
        Lang::En => "Done.",
    }
    .to_string()
}
pub fn btn_regen(lang: Lang) -> String {
    match lang {
        Lang::Ru => "🔄 Перевыпустить",
        Lang::En => "🔄 Reissue",
    }
    .to_string()
}
pub fn regen_running(lang: Lang) -> String {
    match lang {
        Lang::Ru => "⏳ Перевыпускаю…",
        Lang::En => "⏳ Reissuing…",
    }
    .to_string()
}
pub fn btn_regen_all(lang: Lang) -> String {
    match lang {
        Lang::Ru => "🔄 Перевыпустить всех",
        Lang::En => "🔄 Reissue all",
    }
    .to_string()
}
pub fn confirm_regen_all(lang: Lang) -> String {
    match lang {
        Lang::Ru => "🔄 <b>Перевыпустить конфиги всех клиентов?</b>\nФайлы и QR будут перегенерированы, ключи и IP сохранятся — существующие подключения продолжат работать.\n\n🔀 <b>+ сброс маршрутов</b>: дополнительно заменит индивидуальные AllowedIPs клиентов глобальным режимом маршрутизации сервера (нужно после смены режима).",
        Lang::En => "🔄 <b>Reissue configs for all clients?</b>\nFiles and QR codes will be regenerated; keys and IPs are preserved — existing connections keep working.\n\n🔀 <b>+ reset routes</b>: additionally replaces per-client AllowedIPs with the server's global routing mode (needed after a mode change).",
    }.to_string()
}
pub fn btn_regen_all_go(lang: Lang) -> String {
    match lang {
        Lang::Ru => "✅ Перевыпустить",
        Lang::En => "✅ Reissue",
    }
    .to_string()
}
pub fn btn_regen_all_routes(lang: Lang) -> String {
    match lang {
        Lang::Ru => "🔀 + сброс маршрутов",
        Lang::En => "🔀 + reset routes",
    }
    .to_string()
}
pub fn regen_all_running(lang: Lang) -> String {
    match lang {
        Lang::Ru => "⏳ Перевыпускаю всех…",
        Lang::En => "⏳ Reissuing all…",
    }
    .to_string()
}
pub fn regen_all_done(lang: Lang) -> String {
    match lang {
        Lang::Ru => "✅ Все конфиги перевыпущены.",
        Lang::En => "✅ All client configs reissued.",
    }
    .to_string()
}
pub fn regen_all_partial(lang: Lang) -> String {
    match lang {
        Lang::Ru => "⚠️ Завершено, но с ошибками у части клиентов — проверьте логи сервера.",
        Lang::En => "⚠️ Completed, but with errors for some clients — check the server logs.",
    }
    .to_string()
}
pub fn client_exists(lang: Lang, name: &str) -> String {
    let n = html_escape(name);
    match lang {
        Lang::Ru => format!("⚠️ Клиент <b>{n}</b> уже существует. Пересоздать? Старый конфиг будет заменён (новые ключи, новый IP)."),
        Lang::En => format!("⚠️ Client <b>{n}</b> already exists. Recreate? The old config will be replaced (new keys, new IP)."),
    }
}

// --- настройки ---
pub fn settings_title(lang: Lang, psk_default: bool, name_slug: bool) -> String {
    let psk = if psk_default {
        "вкл/on"
    } else {
        "выкл/off"
    };
    let slug = if name_slug { "вкл/on" } else { "выкл/off" };
    match lang {
        Lang::Ru => format!(
            "⚙️ <b>Настройки</b>\nЯзык: русский\nPSK по умолчанию: {psk}\nID-префикс имён: {slug}"
        ),
        Lang::En => format!(
            "⚙️ <b>Settings</b>\nLanguage: English\nDefault PSK: {psk}\nName ID prefix: {slug}"
        ),
    }
}
pub fn btn_lang_ru(lang: Lang) -> String {
    let _ = lang;
    "🇷🇺 Русский".to_string()
}
pub fn btn_lang_en(lang: Lang) -> String {
    let _ = lang;
    "🇬🇧 English".to_string()
}
pub fn btn_psk_toggle(lang: Lang, on: bool) -> String {
    match (lang, on) {
        (Lang::Ru, true) => "PSK: вкл ✅",
        (Lang::Ru, false) => "PSK: выкл ⬜",
        (Lang::En, true) => "PSK: on ✅",
        (Lang::En, false) => "PSK: off ⬜",
    }
    .to_string()
}
pub fn btn_slug_toggle(lang: Lang, on: bool) -> String {
    match (lang, on) {
        (Lang::Ru, true) => "ID-префикс: вкл ✅",
        (Lang::Ru, false) => "ID-префикс: выкл ⬜",
        (Lang::En, true) => "ID prefix: on ✅",
        (Lang::En, false) => "ID prefix: off ⬜",
    }
    .to_string()
}

// --- backup / restore ---
pub fn btn_backup_new(lang: Lang) -> String {
    match lang {
        Lang::Ru => "➕ Создать бэкап",
        Lang::En => "➕ Create backup",
    }
    .to_string()
}
pub fn btn_backup_list(lang: Lang) -> String {
    match lang {
        Lang::Ru => "📃 Список бэкапов",
        Lang::En => "📃 List backups",
    }
    .to_string()
}
pub fn backup_menu_title(lang: Lang) -> String {
    match lang {
        Lang::Ru => "💾 <b>Бэкап</b>",
        Lang::En => "💾 <b>Backup</b>",
    }
    .to_string()
}
pub fn backup_creating(lang: Lang) -> String {
    match lang {
        Lang::Ru => "⏳ Создаю бэкап…",
        Lang::En => "⏳ Creating backup…",
    }
    .to_string()
}
pub fn backup_done(lang: Lang, filename: &str) -> String {
    let f = html_escape(filename);
    match lang {
        Lang::Ru => format!("✅ Бэкап создан:\n<code>{f}</code>"),
        Lang::En => format!("✅ Backup created:\n<code>{f}</code>"),
    }
}
pub fn backups_empty(lang: Lang) -> String {
    match lang {
        Lang::Ru => "Бэкапов пока нет.",
        Lang::En => "No backups yet.",
    }
    .to_string()
}
pub fn backups_list_title(lang: Lang) -> String {
    match lang {
        Lang::Ru => "📃 <b>Бэкапы</b>:",
        Lang::En => "📃 <b>Backups</b>:",
    }
    .to_string()
}
pub fn btn_download(lang: Lang) -> String {
    match lang {
        Lang::Ru => "📥 Скачать",
        Lang::En => "📥 Download",
    }
    .to_string()
}
pub fn btn_restore(lang: Lang) -> String {
    match lang {
        Lang::Ru => "♻️ Восстановить",
        Lang::En => "♻️ Restore",
    }
    .to_string()
}
pub fn confirm_restore(lang: Lang, filename: &str) -> String {
    let f = html_escape(filename);
    match lang {
        Lang::Ru => {
            format!("♻️ Восстановить из <code>{f}</code>? Текущее состояние будет заменено.")
        }
        Lang::En => format!("♻️ Restore from <code>{f}</code>? Current state will be replaced."),
    }
}
pub fn btn_confirm(lang: Lang) -> String {
    match lang {
        Lang::Ru => "✅ Да",
        Lang::En => "✅ Yes",
    }
    .to_string()
}
pub fn restoring(lang: Lang) -> String {
    match lang {
        Lang::Ru => "⏳ Восстанавливаю…",
        Lang::En => "⏳ Restoring…",
    }
    .to_string()
}
pub fn restore_done(lang: Lang) -> String {
    match lang {
        Lang::Ru => "✅ Восстановление завершено.",
        Lang::En => "✅ Restore complete.",
    }
    .to_string()
}

// --- check ---
pub fn check_running(lang: Lang) -> String {
    match lang {
        Lang::Ru => "⏳ Проверяю сервер…",
        Lang::En => "⏳ Checking server…",
    }
    .to_string()
}
pub fn check_result(lang: Lang, body: &str) -> String {
    let b = html_escape(body);
    match lang {
        Lang::Ru => format!("🩺 <b>Проверка</b>\n<pre>{b}</pre>"),
        Lang::En => format!("🩺 <b>Check</b>\n<pre>{b}</pre>"),
    }
}

pub fn btn_diagnose(lang: Lang) -> String {
    match lang {
        Lang::Ru => "🔬 Диагностика",
        Lang::En => "🔬 Diagnostics",
    }
    .to_string()
}
pub fn diagnose_running(lang: Lang) -> String {
    match lang {
        Lang::Ru => "⏳ Диагностирую…",
        Lang::En => "⏳ Running diagnostics…",
    }
    .to_string()
}
pub fn diagnose_result(lang: Lang, body: &str) -> String {
    let b = html_escape(body);
    match lang {
        Lang::Ru => format!("🔬 <b>Диагностика</b>\n<pre>{b}</pre>"),
        Lang::En => format!("🔬 <b>Diagnostics</b>\n<pre>{b}</pre>"),
    }
}

// --- статус клиента (по стабильному status_code) ---
/// Возвращает локализованную метку статуса по стабильному `status_code`.
/// Текст НЕ экранируется — вызывающий код (`client_card`) сам делает html_escape,
/// экранировать здесь означало бы двойное экранирование.
pub fn status_label(lang: Lang, status_code: &str, raw: &str) -> String {
    match (lang, status_code) {
        (Lang::Ru, "active") => "Активен",
        (Lang::En, "active") => "Active",
        (Lang::Ru, "recent") => "Недавно",
        (Lang::En, "recent") => "Recently",
        (Lang::Ru, "no_handshake") => "Нет handshake",
        (Lang::En, "no_handshake") => "No handshake",
        (Lang::Ru, "inactive") => "Неактивен",
        (Lang::En, "inactive") => "Inactive",
        (Lang::Ru, "key_error") => "Ошибка ключа",
        (Lang::En, "key_error") => "Key error",
        (Lang::Ru, "no_data") => "Нет данных",
        (Lang::En, "no_data") => "No data",
        (Lang::Ru, _) => {
            return if raw.is_empty() {
                "неизвестно".to_string()
            } else {
                raw.to_string()
            };
        }
        (Lang::En, _) => {
            return if raw.is_empty() {
                "unknown".to_string()
            } else {
                raw.to_string()
            };
        }
    }
    .to_string()
}

// --- ошибки (локализованные, без утечки stderr) ---
pub fn error_text(lang: Lang, err: &Error) -> String {
    match (lang, err) {
        (Lang::Ru, Error::Timeout) => "⏳ Превышено время ожидания. Попробуйте позже.",
        (Lang::En, Error::Timeout) => "⏳ Operation timed out. Try later.",
        (Lang::Ru, Error::ScriptFailed { .. }) => "❌ Операция не удалась. Попробуйте ещё раз.",
        (Lang::En, Error::ScriptFailed { .. }) => "❌ Operation failed. Try again.",
        (Lang::Ru, Error::Parse(_)) => "❌ Не удалось разобрать ответ сервера.",
        (Lang::En, Error::Parse(_)) => "❌ Failed to parse server response.",
        (Lang::Ru, Error::ClientExists(_)) => {
            "⚠️ Клиент с таким именем уже существует — создание пропущено."
        }
        (Lang::En, Error::ClientExists(_)) => {
            "⚠️ A client with this name already exists — creation was skipped."
        }
        (Lang::Ru, _) => "❌ Ошибка выполнения операции.",
        (Lang::En, _) => "❌ Operation error.",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_html_specials() {
        assert_eq!(html_escape("a<b>&c"), "a&lt;b&gt;&amp;c");
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("plain"), "plain");
    }

    #[test]
    fn amp_escaped_first() {
        // & должен экранироваться до < и >, иначе получим двойное экранирование
        assert_eq!(html_escape("<"), "&lt;");
        assert!(!html_escape("a & b").contains("&amp;amp;"));
    }

    #[test]
    fn ask_client_name_mentions_spaces_and_slug_status() {
        for l in [Lang::Ru, Lang::En] {
            let on = ask_client_name(l, true);
            let off = ask_client_name(l, false);
            // промпт всегда предупреждает про замену пробелов
            assert!(on.contains('-'));
            assert!(off.contains('-'));
            // и различает вкл/выкл id-префикса
            assert_ne!(on, off);
            // лимит в сообщении об ошибке зависит от слага
            assert!(bad_name(l, true).contains("26"));
            assert!(bad_name(l, false).contains("32"));
        }
    }

    #[test]
    fn lang_roundtrip() {
        assert_eq!(parse_lang("ru"), Some(Lang::Ru));
        assert_eq!(parse_lang("en"), Some(Lang::En));
        assert_eq!(parse_lang("xx"), None);
        assert_eq!(lang_code(Lang::Ru), "ru");
        assert_eq!(lang_code(Lang::En), "en");
        assert_eq!(Lang::default(), Lang::Ru);
    }

    #[test]
    fn private_only_mentions_both_languages() {
        // Строка билингвальна (язык может быть ещё не определён), поэтому
        // проверяем оба маркера, а не полагаемся на конкретный Lang.
        let text = private_only();
        assert!(!text.is_empty());
        assert!(text.contains("личном"));
        assert!(text.contains("private"));
    }

    #[test]
    fn all_messages_nonempty_both_langs() {
        for l in [Lang::Ru, Lang::En] {
            assert!(!menu_title(l).is_empty());
            assert!(!access_denied(l).is_empty());
            assert!(!ask_client_name(l, true).is_empty());
            assert!(!ask_expiry(l).is_empty());
            assert!(!settings_title(l, true, true).is_empty());
            assert!(!backups_empty(l).is_empty());
            assert!(!restore_done(l).is_empty());
            // карточка: имя экранируется
            let card = client_card(
                l,
                "a<b>",
                "Активен",
                "10.0.0.2",
                "1 KB",
                "0 B",
                "никогда",
                "бессрочно",
            );
            assert!(card.contains("a&lt;b&gt;"));
            assert!(!card.contains("a<b>"));
        }
    }

    #[test]
    fn status_label_known_codes_translated() {
        assert_eq!(status_label(Lang::En, "active", "Активен"), "Active");
        assert_eq!(status_label(Lang::Ru, "active", ""), "Активен");
    }

    #[test]
    fn status_label_unknown_code_falls_back_to_raw() {
        // status_label не экранирует — экранирование делает client_card ниже по цепочке.
        assert_eq!(status_label(Lang::En, "weird_code", "<x>"), "<x>");
        assert_eq!(status_label(Lang::Ru, "weird_code", ""), "неизвестно");
        assert_eq!(status_label(Lang::En, "weird_code", ""), "unknown");
    }

    #[test]
    fn client_exists_nonempty_both_langs() {
        for l in [Lang::Ru, Lang::En] {
            let msg = client_exists(l, "alice");
            assert!(!msg.is_empty());
            assert!(msg.contains("alice"));
        }
    }

    #[test]
    fn client_exists_escapes_html() {
        // Имя проходит validate_name (без <>), но html_escape не должен
        // давать двойное экранирование (&amp;amp;).
        let msg = client_exists(Lang::Ru, "alice");
        assert!(!msg.contains("&amp;amp;"));
    }

    #[test]
    fn error_text_covers_variants() {
        use crate::error::Error;
        for l in [Lang::Ru, Lang::En] {
            for e in [
                Error::Timeout,
                Error::Parse("x".into()),
                Error::ScriptFailed {
                    code: Some(1),
                    stderr: "secret".into(),
                },
                Error::Telegram("x".into()),
                Error::ClientExists("alice".into()),
            ] {
                let t = error_text(l, &e);
                assert!(!t.is_empty());
                assert!(!t.contains("secret")); // stderr не утекает
            }
        }
    }

    #[test]
    fn error_text_client_exists_is_specific() {
        use crate::error::Error;
        let e = Error::ClientExists("alice".into());
        assert!(error_text(Lang::Ru, &e).contains("существует"));
        assert!(error_text(Lang::En, &e).contains("exists"));
    }

    #[test]
    fn diagnose_strings_nonempty_both_langs() {
        for l in [Lang::Ru, Lang::En] {
            assert!(!btn_diagnose(l).is_empty());
            assert!(!diagnose_running(l).is_empty());
            let r = diagnose_result(l, "body <x>");
            assert!(r.contains("<pre>"));
            assert!(r.contains("&lt;x&gt;")); // вывод экранируется
        }
    }

    #[test]
    fn regen_strings_nonempty_both_langs() {
        for l in [Lang::Ru, Lang::En] {
            assert!(!btn_regen(l).is_empty());
            assert!(!regen_running(l).is_empty());
        }
    }

    #[test]
    fn regen_all_strings_nonempty_both_langs() {
        for l in [Lang::Ru, Lang::En] {
            assert!(!btn_regen_all(l).is_empty());
            assert!(!confirm_regen_all(l).is_empty());
            assert!(!btn_regen_all_go(l).is_empty());
            assert!(!btn_regen_all_routes(l).is_empty());
            assert!(!regen_all_running(l).is_empty());
            assert!(!regen_all_done(l).is_empty());
            assert!(!regen_all_partial(l).is_empty());
        }
    }

    #[test]
    fn backup_not_found_differs_from_client_not_found() {
        let ru_backup = backup_not_found(Lang::Ru);
        let en_backup = backup_not_found(Lang::En);
        let ru_client = not_found(Lang::Ru);
        let en_client = not_found(Lang::En);

        assert!(!ru_backup.is_empty());
        assert!(!en_backup.is_empty());
        assert!(ru_backup.contains("Бэкап"));
        assert!(en_backup.contains("Backup"));

        assert_ne!(ru_backup, ru_client);
        assert_ne!(en_backup, en_client);
    }
}
