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
pub fn btn_refresh(lang: Lang) -> String {
    match lang {
        Lang::Ru => "🔄 Обновить",
        Lang::En => "🔄 Refresh",
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

// --- массовая генерация ---
pub fn btn_bulk(lang: Lang) -> String {
    match lang {
        Lang::Ru => "📦 Пакет",
        Lang::En => "📦 Bulk",
    }
    .to_string()
}
pub fn ask_bulk_prefix(lang: Lang) -> String {
    match lang {
        Lang::Ru => "Введите префикс для имён (напр. «user» → user-01 … user-10):",
        Lang::En => "Enter a name prefix (e.g. \"user\" → user-01 … user-10):",
    }
    .to_string()
}
/// `max_len` — фактический предел длины префикса (зависит от настройки
/// ID-префикса, см. `validate::max_bulk_prefix_len`).
pub fn bad_bulk_prefix(lang: Lang, max_len: usize) -> String {
    match lang {
        Lang::Ru => {
            format!("⚠️ Префикс: латиница/цифры/-/_, 1–{max_len} символов. Попробуйте ещё раз:")
        }
        Lang::En => format!("⚠️ Prefix: a-z0-9 -_, 1–{max_len} chars. Try again:"),
    }
}
pub fn ask_bulk_count(lang: Lang) -> String {
    match lang {
        Lang::Ru => "Выберите количество (максимум 10 — лимит альбома Telegram):",
        Lang::En => "Choose quantity (max 10 — Telegram album limit):",
    }
    .to_string()
}
pub fn bulk_creating(lang: Lang) -> String {
    match lang {
        Lang::Ru => "⏳ Создаю клиентов…",
        Lang::En => "⏳ Creating clients…",
    }
    .to_string()
}
/// Итог массовой генерации: «Создано N» (+ «, пропущено: b (существует)…» если есть).
pub fn bulk_result_summary(lang: Lang, res: &crate::vpn::model::BulkResult) -> String {
    use crate::vpn::model::SkipReason;
    let created = res.created.len();
    if res.skipped.is_empty() {
        return match lang {
            Lang::Ru => format!("✅ Создано клиентов: {created}."),
            Lang::En => format!("✅ Created {created} clients."),
        };
    }
    let skip_lines: Vec<String> = res
        .skipped
        .iter()
        .map(|s| {
            let reason = match (lang, s.reason) {
                (Lang::Ru, SkipReason::Exists) => "существует",
                (Lang::En, SkipReason::Exists) => "exists",
                (Lang::Ru, SkipReason::InvalidName) => "невалидное имя",
                (Lang::En, SkipReason::InvalidName) => "invalid name",
                (Lang::Ru, SkipReason::Error) => "ошибка",
                (Lang::En, SkipReason::Error) => "error",
            };
            format!("• {} ({})", html_escape(&s.name), reason)
        })
        .collect();
    match lang {
        Lang::Ru => format!(
            "✅ Создано: {created}.\n⚠️ Пропущено:\n{}",
            skip_lines.join("\n")
        ),
        Lang::En => format!(
            "✅ Created: {created}.\n⚠️ Skipped:\n{}",
            skip_lines.join("\n")
        ),
    }
}

// --- capacity-ошибки ---
pub fn capacity_insufficient(lang: Lang, free: u32, needed: u32) -> String {
    match lang {
        Lang::Ru => {
            format!("⚠️ Свободно адресов: {free}, запрошено {needed}. Уменьшите количество.")
        }
        Lang::En => {
            format!("⚠️ Only {free} addresses free, {needed} requested. Reduce the quantity.")
        }
    }
}
pub fn capacity_exhausted(lang: Lang) -> String {
    match lang {
        Lang::Ru => "⚠️ Свободные адреса исчерпаны. Удалите неиспользуемых клиентов.",
        Lang::En => "⚠️ No free addresses left. Remove unused clients.",
    }
    .to_string()
}
pub fn capacity_unavailable(lang: Lang) -> String {
    match lang {
        Lang::Ru => {
            "⚠️ Не удалось получить информацию об интерфейсе. Проверьте сервер («🩺 Проверка»)."
        }
        Lang::En => "⚠️ Could not get interface info. Check the server (\"🩺 Check\").",
    }
    .to_string()
}

// --- тумблеры выдачи (экран настроек) ---
pub fn btn_conf_toggle(lang: Lang, on: bool) -> String {
    match (lang, on) {
        (Lang::Ru, true) => "📄 Конфиг: вкл ✅",
        (Lang::Ru, false) => "📄 Конфиг: выкл ⬜",
        (Lang::En, true) => "📄 Config: on ✅",
        (Lang::En, false) => "📄 Config: off ⬜",
    }
    .to_string()
}
pub fn btn_qr_toggle(lang: Lang, on: bool) -> String {
    match (lang, on) {
        (Lang::Ru, true) => "🖼 QR: вкл ✅",
        (Lang::Ru, false) => "🖼 QR: выкл ⬜",
        (Lang::En, true) => "🖼 QR: on ✅",
        (Lang::En, false) => "🖼 QR: off ⬜",
    }
    .to_string()
}
pub fn btn_link_toggle(lang: Lang, on: bool) -> String {
    match (lang, on) {
        (Lang::Ru, true) => "🔗 Ссылка: вкл ✅",
        (Lang::Ru, false) => "🔗 Ссылка: выкл ⬜",
        (Lang::En, true) => "🔗 Link: on ✅",
        (Lang::En, false) => "🔗 Link: off ⬜",
    }
    .to_string()
}

// --- карточка клиента: отдельные артефакты ---
pub fn btn_card_qr(lang: Lang) -> String {
    match lang {
        Lang::Ru => "🖼 QR",
        Lang::En => "🖼 QR",
    }
    .to_string()
}
pub fn btn_card_link(lang: Lang) -> String {
    match lang {
        Lang::Ru => "🔗 Ссылка",
        Lang::En => "🔗 Link",
    }
    .to_string()
}
pub fn btn_card_all(lang: Lang) -> String {
    match lang {
        Lang::Ru => "📦 Всё",
        Lang::En => "📦 All",
    }
    .to_string()
}
pub fn qr_not_generated(lang: Lang) -> String {
    match lang {
        Lang::Ru => "⚠️ QR не сгенерирован (возможно, qrencode не установлен на сервере).",
        Lang::En => "⚠️ QR was not generated (qrencode may be missing on the server).",
    }
    .to_string()
}
pub fn link_unavailable(lang: Lang) -> String {
    match lang {
        Lang::Ru => "⚠️ Ссылка для импорта недоступна.",
        Lang::En => "⚠️ Import link is unavailable.",
    }
    .to_string()
}

// --- карточка/статистика (динамика экранируется) ---
#[allow(clippy::too_many_arguments)]
pub fn client_card(
    lang: Lang,
    name: &str,
    mark: &str,
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
            Lang::Ru => format!("🌐 IP: {ip}\n"),
            Lang::En => format!("🌐 IP: {ip}\n"),
        }
    };
    match lang {
        Lang::Ru => format!("👤 <b>{name}</b>\n{mark} Статус: {status}\n{ip_line}📊 Трафик:  ↓ {rx} · ↑ {tx}\n⏱ Рукопожатие: {handshake}\n📅 Действует: {expires}"),
        Lang::En => format!("👤 <b>{name}</b>\n{mark} Status: {status}\n{ip_line}📊 Traffic:  ↓ {rx} · ↑ {tx}\n⏱ Handshake: {handshake}\n📅 Expires: {expires}"),
    }
}

/// Блок трафика по периодам в карточке клиента: дополняет `client_card`
/// объёмами за сегодня/7/30 дней/всё время и онлайн-временем за 7 дней.
/// Объёмы и длительность уже отформатированы вызывающим кодом
/// (`human_bytes`/`format_minutes`).
pub fn client_card_traffic(
    lang: Lang,
    today: &str,
    d7: &str,
    d30: &str,
    total: &str,
    online7: &str,
) -> String {
    match lang {
        Lang::Ru => format!(
            "📈 Сегодня: {today} · 7 дн: {d7} (онлайн {online7}) · 30 дн: {d30} · Всего: {total}"
        ),
        Lang::En => format!(
            "📈 Today: {today} · 7 d: {d7} (online {online7}) · 30 d: {d30} · Total: {total}"
        ),
    }
}

/// Экран «История»: заголовок с именем клиента (экранируется) + готовый
/// многострочный блок событий, собранный вызывающим кодом.
pub fn history_screen(lang: Lang, name: &str, lines: &str) -> String {
    let name = html_escape(name);
    match lang {
        Lang::Ru => format!("📜 <b>История {name}</b>\n{lines}"),
        Lang::En => format!("📜 <b>History {name}</b>\n{lines}"),
    }
}

/// Экран «История» без событий — дружелюбная плашка вместо пустого списка.
pub fn history_empty(lang: Lang, name: &str) -> String {
    let name = html_escape(name);
    match lang {
        Lang::Ru => format!("📜 <b>История {name}</b>\nпока нет событий."),
        Lang::En => format!("📜 <b>History {name}</b>\nno events yet."),
    }
}

/// Плейсхолдер для блока «топ клиентов» на экране статистики, когда данных
/// за период ещё нет (пустой список).
pub fn top_empty(lang: Lang) -> String {
    match lang {
        Lang::Ru => "пока нет данных".to_string(),
        Lang::En => "no data yet".to_string(),
    }
}

/// Подпись события журнала для экрана «История». `kind` — строка из БД
/// (см. `EventRow`): online/offline пишет `ingest`, остальные — `log_event`.
/// Неизвестный вид события — возвращаем `kind` как есть (не должно случаться
/// в норме, но не роняем рендер на будущих/чужих значениях).
pub fn event_label(lang: Lang, kind: &str) -> String {
    match (lang, kind) {
        (Lang::Ru, "online") => "🟢 подключился",
        (Lang::En, "online") => "🟢 connected",
        (Lang::Ru, "offline") => "⚪ отключился",
        (Lang::En, "offline") => "⚪ disconnected",
        (Lang::Ru, "client_add") => "➕ создан",
        (Lang::En, "client_add") => "➕ created",
        (Lang::Ru, "client_remove") => "🗑 удалён",
        (Lang::En, "client_remove") => "🗑 removed",
        (Lang::Ru, "regen") | (Lang::Ru, "regen_all") => "♻️ перевыпуск",
        (Lang::En, "regen") | (Lang::En, "regen_all") => "♻️ regenerated",
        (Lang::Ru, "modify") => "✏️ изменён",
        (Lang::En, "modify") => "✏️ modified",
        _ => return kind.to_string(),
    }
    .to_string()
}
/// Расширенный экран статистики: периоды трафика (сегодня/7/30 дней/всё
/// время), тренд недели, среднее в день и топ клиентов. Все объёмы уже
/// отформатированы вызывающим кодом через `human_bytes`; `top_lines` —
/// готовый многострочный блок (или плашка «нет данных»).
#[allow(clippy::too_many_arguments)]
pub fn stats_screen(
    lang: Lang,
    total: usize,
    online: usize,
    today: &str,
    d7: &str,
    d30: &str,
    all_time: &str,
    avg_day: &str,
    trend: &str,
    top_lines: &str,
) -> String {
    match lang {
        Lang::Ru => format!(
            "📊 <b>Статистика сервера</b>\n👥 Клиентов: {total} · Онлайн: {online}\n\n📈 Трафик (↓+↑):\n• Сегодня: {today}\n• 7 дн: {d7} {trend}\n• 30 дн: {d30}\n• Всего: {all_time}\n• В среднем/день (7 дн): {avg_day}\n\n🏆 Топ за 7 дн:\n{top_lines}"
        ),
        Lang::En => format!(
            "📊 <b>Server stats</b>\n👥 Clients: {total} · Online: {online}\n\n📈 Traffic (↓+↑):\n• Today: {today}\n• 7 d: {d7} {trend}\n• 30 d: {d30}\n• All time: {all_time}\n• Avg/day (7 d): {avg_day}\n\n🏆 Top for 7 d:\n{top_lines}"
        ),
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

/// Заголовок списка клиентов с индикатором активного фильтра.
/// `All` (или когда показаны все) → без пометки (как `clients_title`).
/// Иначе → «👥 Клиенты — 🟢 онлайн (3 из 10)».
pub fn clients_title_filtered(
    lang: Lang,
    filter: crate::vpn::model::ClientFilter,
    shown: usize,
    total: usize,
) -> String {
    if matches!(filter, crate::vpn::model::ClientFilter::All) || shown == total {
        return clients_title(lang);
    }
    let mark = filter.mark();
    let label = match (lang, filter) {
        (Lang::Ru, crate::vpn::model::ClientFilter::Online) => "онлайн",
        (Lang::En, crate::vpn::model::ClientFilter::Online) => "online",
        (Lang::Ru, crate::vpn::model::ClientFilter::Offline) => "оффлайн",
        (Lang::En, crate::vpn::model::ClientFilter::Offline) => "offline",
        (Lang::Ru, crate::vpn::model::ClientFilter::Never) => "никогда",
        (Lang::En, crate::vpn::model::ClientFilter::Never) => "never",
        _ => "",
    };
    match lang {
        Lang::Ru => format!("👥 <b>Клиенты</b> — {mark} {label} ({shown} из {total})"),
        Lang::En => format!("👥 <b>Clients</b> — {mark} {label} ({shown} of {total})"),
    }
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
pub fn settings_title(
    lang: Lang,
    psk_default: bool,
    name_slug: bool,
    deliver_conf: bool,
    deliver_qr: bool,
    deliver_link: bool,
) -> String {
    let onoff = |b: bool| match (lang, b) {
        (Lang::Ru, true) => "вкл",
        (Lang::Ru, false) => "выкл",
        (Lang::En, true) => "on",
        (Lang::En, false) => "off",
    };
    match lang {
        Lang::Ru => format!(
            "⚙️ <b>Настройки</b>\nЯзык: русский\nPSK по умолчанию: {}\nID-префикс имён: {}\n📄 Выдача конфига: {}\n🖼 Выдача QR: {}\n🔗 Выдача ссылки: {}",
            onoff(psk_default),
            onoff(name_slug),
            onoff(deliver_conf),
            onoff(deliver_qr),
            onoff(deliver_link)
        ),
        Lang::En => format!(
            "⚙️ <b>Settings</b>\nLanguage: English\nDefault PSK: {}\nName ID prefix: {}\n📄 Deliver config: {}\n🖼 Deliver QR: {}\n🔗 Deliver link: {}",
            onoff(psk_default),
            onoff(name_slug),
            onoff(deliver_conf),
            onoff(deliver_qr),
            onoff(deliver_link)
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

// --- статус клиента (по вычисленной ботом метке) ---
/// Подпись статуса по вычисленной боту метке (см. `model::status_mark_at`).
pub fn status_label_mark(lang: Lang, mark: &str) -> String {
    match (lang, mark) {
        (Lang::Ru, "🟢") => "Онлайн".into(),
        (Lang::Ru, "🟡") => "Не подключался".into(),
        (Lang::Ru, _) => "Оффлайн".into(),
        (Lang::En, "🟢") => "Online".into(),
        (Lang::En, "🟡") => "Never connected".into(),
        (Lang::En, _) => "Offline".into(),
    }
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
        (Lang::Ru, Error::ClientNotFound(_)) => "⚠️ Клиент не найден.",
        (Lang::En, Error::ClientNotFound(_)) => "⚠️ Client not found.",
        (Lang::Ru, Error::RestoreRolledBack) => {
            "⚠️ Восстановление провалилось. Конфиг откачен к предыдущему состоянию."
        }
        (Lang::En, Error::RestoreRolledBack) => "⚠️ Restore failed. Configuration was rolled back.",
        (Lang::Ru, _) => "❌ Ошибка выполнения операции.",
        (Lang::En, _) => "❌ Operation error.",
    }
    .to_string()
}

// --- modify ---
pub fn btn_modify(lang: Lang) -> String {
    match lang {
        Lang::Ru => "⚙️ Изменить",
        Lang::En => "⚙️ Modify",
    }
    .to_string()
}
pub fn btn_history(lang: Lang) -> String {
    match lang {
        Lang::Ru => "📜 История",
        Lang::En => "📜 History",
    }
    .to_string()
}
/// Кнопка «Назад» экрана «История» — возвращает к карточке клиента (не к
/// главному меню, в отличие от `btn_back`), поэтому текст короче.
pub fn btn_history_back(lang: Lang) -> String {
    match lang {
        Lang::Ru => "⬅️ Назад",
        Lang::En => "⬅️ Back",
    }
    .to_string()
}
pub fn modify_param_label(lang: Lang, p: crate::vpn::validate::ModifyParam) -> String {
    use crate::vpn::validate::ModifyParam;
    match (lang, p) {
        (Lang::Ru, ModifyParam::Keepalive) => "⏱ Keepalive",
        (Lang::En, ModifyParam::Keepalive) => "⏱ Keepalive",
        (Lang::Ru, ModifyParam::Dns) => "🌐 DNS",
        (Lang::En, ModifyParam::Dns) => "🌐 DNS",
        (Lang::Ru, ModifyParam::AllowedIps) => "🔗 AllowedIPs",
        (Lang::En, ModifyParam::AllowedIps) => "🔗 AllowedIPs",
        (Lang::Ru, ModifyParam::Endpoint) => "📡 Endpoint",
        (Lang::En, ModifyParam::Endpoint) => "📡 Endpoint",
    }
    .to_string()
}
pub fn ask_modify_param(lang: Lang, p: crate::vpn::validate::ModifyParam) -> String {
    use crate::vpn::validate::ModifyParam;
    let hint = match (lang, p) {
        (Lang::Ru, ModifyParam::Keepalive) => "Введите Keepalive в секундах (0–65535, 0 = выкл):",
        (Lang::En, ModifyParam::Keepalive) => "Enter Keepalive in seconds (0–65535, 0 = off):",
        (Lang::Ru, ModifyParam::Dns) => "Введите DNS (через запятую, до 4 адресов):",
        (Lang::En, ModifyParam::Dns) => "Enter DNS (comma-separated, up to 4):",
        (Lang::Ru, ModifyParam::AllowedIps) => "Введите AllowedIPs (CIDR через запятую):",
        (Lang::En, ModifyParam::AllowedIps) => "Enter AllowedIPs (comma-separated CIDR):",
        (Lang::Ru, ModifyParam::Endpoint) => "Введите Endpoint (host:port):",
        (Lang::En, ModifyParam::Endpoint) => "Enter Endpoint (host:port):",
    };
    hint.to_string()
}
pub fn modify_done(lang: Lang, p: crate::vpn::validate::ModifyParam, value: &str) -> String {
    let label = modify_param_label(lang, p);
    let v = html_escape(value);
    match lang {
        Lang::Ru => format!("✅ {label} изменён на <code>{v}</code>"),
        Lang::En => format!("✅ {label} changed to <code>{v}</code>"),
    }
}
pub fn modify_param_select_title(lang: Lang) -> String {
    match lang {
        Lang::Ru => "⚙️ Выберите параметр:",
        Lang::En => "⚙️ Choose parameter:",
    }
    .to_string()
}

// --- restart / repair ---
pub fn btn_restart(lang: Lang) -> String {
    match lang {
        Lang::Ru => "🔁 Перезапуск",
        Lang::En => "🔁 Restart",
    }
    .to_string()
}
pub fn btn_repair(lang: Lang) -> String {
    match lang {
        Lang::Ru => "🛠 Починка модуля",
        Lang::En => "🛠 Repair module",
    }
    .to_string()
}
pub fn confirm_restart(lang: Lang) -> String {
    match lang {
        Lang::Ru => {
            "🔁 <b>Перезапустить сервис?</b>\nВсе VPN-подключения будут разорваны на 1–2 секунды."
        }
        Lang::En => "🔁 <b>Restart service?</b>\nAll VPN connections will drop for 1–2 seconds.",
    }
    .to_string()
}
pub fn btn_restart_go(lang: Lang) -> String {
    match lang {
        Lang::Ru => "✅ Перезапустить",
        Lang::En => "✅ Restart",
    }
    .to_string()
}
pub fn restart_done(lang: Lang, active: bool) -> String {
    match (lang, active) {
        (Lang::Ru, true) => "✅ Сервис перезапущен и активен.",
        (Lang::Ru, false) => "⚠️ Сервис перезапущен, но НЕ активен — проверьте логи.",
        (Lang::En, true) => "✅ Service restarted and active.",
        (Lang::En, false) => "⚠️ Service restarted but NOT active — check logs.",
    }
    .to_string()
}
pub fn repair_result(lang: Lang, rc: i32) -> String {
    match (lang, rc) {
        (Lang::Ru, 0) => "✅ Модуль и сервис OK.".to_string(),
        (Lang::En, 0) => "✅ Module and service OK.".to_string(),
        (Lang::Ru, 1) => "❌ Модуль не поднялся. См. логи сервера.".to_string(),
        (Lang::En, 1) => "❌ Module failed to load. See server logs.".to_string(),
        (Lang::Ru, 2) => "⚠️ Модуль OK, но сервис не запустился.".to_string(),
        (Lang::En, 2) => "⚠️ Module OK, but service didn't start.".to_string(),
        (Lang::Ru, _) => "❌ Неизвестный результат repair.".to_string(),
        (Lang::En, _) => "❌ Unknown repair result.".to_string(),
    }
}

// --- check card (структурированный отчёт) ---
fn bool_mark(ok: bool) -> &'static str {
    if ok {
        "✅"
    } else {
        "❌"
    }
}
fn warn_mark(present: bool) -> &'static str {
    if present {
        "✅"
    } else {
        "⚠️"
    }
}
pub fn check_card(lang: Lang, r: &crate::vpn::wire::CheckReport) -> String {
    let header = if r.ok { "✅" } else { "❌" };
    let mut lines = Vec::new();
    match lang {
        Lang::Ru => {
            lines.push(format!("{header} <b>Проверка сервера</b>"));
            lines.push(format!(
                "{} Сервис: {}",
                bool_mark(r.service.active),
                if r.service.active {
                    "активен"
                } else {
                    "НЕ активен"
                }
            ));
            let iface = if r.interface.present {
                let mtu = r
                    .interface
                    .mtu
                    .map(|m| m.to_string())
                    .unwrap_or_else(|| "n/a".to_string());
                let addrs = r.interface.addresses.join(", ");
                format!(
                    "{} Интерфейс {}: MTU {}, {}",
                    bool_mark(true),
                    html_escape(&r.interface.name),
                    mtu,
                    html_escape(&addrs)
                )
            } else {
                "❌ Интерфейс отсутствует".to_string()
            };
            lines.push(iface);
            lines.push(format!(
                "{} Порт {}/{}: {}",
                bool_mark(r.port.listening),
                r.port.number,
                r.port.proto,
                if r.port.listening {
                    "прослушивается"
                } else {
                    "НЕ прослушивается"
                }
            ));
            lines.push(format!(
                "{} Модуль amneziawg: {}",
                warn_mark(r.module.loaded),
                if r.module.loaded {
                    "загружен"
                } else {
                    "не загружен (норма для userspace)"
                }
            ));
            lines.push(format!("👥 Клиентов: {}", r.clients.total));
            let fw = if !r.firewall.ufw_active {
                "⚠️ UFW: не активен".to_string()
            } else if !r.firewall.port_allowed {
                "⚠️ UFW: активен, порт НЕ разрешён".to_string()
            } else {
                "✅ UFW: активен, порт разрешён".to_string()
            };
            lines.push(fw);
        }
        Lang::En => {
            lines.push(format!("{header} <b>Server check</b>"));
            lines.push(format!(
                "{} Service: {}",
                bool_mark(r.service.active),
                if r.service.active {
                    "active"
                } else {
                    "NOT active"
                }
            ));
            let iface = if r.interface.present {
                let mtu = r
                    .interface
                    .mtu
                    .map(|m| m.to_string())
                    .unwrap_or_else(|| "n/a".to_string());
                let addrs = r.interface.addresses.join(", ");
                format!(
                    "{} Interface {}: MTU {}, {}",
                    bool_mark(true),
                    html_escape(&r.interface.name),
                    mtu,
                    html_escape(&addrs)
                )
            } else {
                "❌ Interface missing".to_string()
            };
            lines.push(iface);
            lines.push(format!(
                "{} Port {}/{}: {}",
                bool_mark(r.port.listening),
                r.port.number,
                r.port.proto,
                if r.port.listening {
                    "listening"
                } else {
                    "NOT listening"
                }
            ));
            lines.push(format!(
                "{} amneziawg module: {}",
                warn_mark(r.module.loaded),
                if r.module.loaded {
                    "loaded"
                } else {
                    "not loaded (ok for userspace)"
                }
            ));
            lines.push(format!("👥 Clients: {}", r.clients.total));
            let fw = if !r.firewall.ufw_active {
                "⚠️ UFW: inactive".to_string()
            } else if !r.firewall.port_allowed {
                "⚠️ UFW: active, port NOT allowed".to_string()
            } else {
                "✅ UFW: active, port allowed".to_string()
            };
            lines.push(fw);
        }
    }
    lines.join("\n")
}

// --- группы (#20) ---
pub fn btn_groups(lang: Lang) -> String {
    match lang {
        Lang::Ru => "🗂 Группы",
        Lang::En => "🗂 Groups",
    }
    .to_string()
}
pub fn groups_title(lang: Lang, count: usize) -> String {
    match lang {
        Lang::Ru => format!("🗂 <b>Группы</b> ({count}):"),
        Lang::En => format!("🗂 <b>Groups</b> ({count}):"),
    }
}
pub fn groups_empty(lang: Lang) -> String {
    match lang {
        Lang::Ru => "Групп пока нет.",
        Lang::En => "No groups yet.",
    }
    .to_string()
}
pub fn btn_group_create(lang: Lang) -> String {
    match lang {
        Lang::Ru => "➕ Создать группу",
        Lang::En => "➕ New group",
    }
    .to_string()
}
pub fn ask_group_name(lang: Lang) -> String {
    match lang {
        Lang::Ru => "Введите имя группы:",
        Lang::En => "Enter group name:",
    }
    .to_string()
}
pub fn bad_group_name(lang: Lang) -> String {
    match lang {
        Lang::Ru => "⚠️ Некорректное имя группы (1–32 символа). Введите ещё раз:",
        Lang::En => "⚠️ Invalid group name (1–32 chars). Try again:",
    }
    .to_string()
}
pub fn group_delete_running(lang: Lang) -> String {
    match lang {
        Lang::Ru => "🗑 Удаляю клиентов группы…",
        Lang::En => "🗑 Removing group clients…",
    }
    .to_string()
}
pub fn group_name_taken(lang: Lang, name: &str) -> String {
    let n = html_escape(name);
    match lang {
        Lang::Ru => format!("⚠️ Группа <b>{n}</b> уже существует. Введите другое имя:"),
        Lang::En => format!("⚠️ Group <b>{n}</b> already exists. Enter a different name:"),
    }
}
pub fn group_created(lang: Lang, name: &str) -> String {
    let name = html_escape(name);
    match lang {
        Lang::Ru => format!("✅ Группа <b>{name}</b> создана."),
        Lang::En => format!("✅ Group <b>{name}</b> created."),
    }
}
pub fn group_renamed(lang: Lang, name: &str) -> String {
    let name = html_escape(name);
    match lang {
        Lang::Ru => format!("✅ Группа переименована в <b>{name}</b>."),
        Lang::En => format!("✅ Group renamed to <b>{name}</b>."),
    }
}
/// HTML-карточка группы: имя жирным, число клиентов, лимит (число или
/// «безлимит»/«unlimited» при `None`) и число админов.
pub fn group_card(
    lang: Lang,
    name: &str,
    clients: i64,
    quota: Option<i64>,
    admins: usize,
) -> String {
    let name = html_escape(name);
    match lang {
        Lang::Ru => {
            let limit = match quota {
                Some(q) => q.to_string(),
                None => "безлимит".to_string(),
            };
            format!(
                "🗂 <b>{name}</b>\n👥 Клиенты: {clients}\n🔢 Лимит: {limit}\n👮 Админы: {admins}"
            )
        }
        Lang::En => {
            let limit = match quota {
                Some(q) => q.to_string(),
                None => "unlimited".to_string(),
            };
            format!(
                "🗂 <b>{name}</b>\n👥 Clients: {clients}\n🔢 Quota: {limit}\n👮 Admins: {admins}"
            )
        }
    }
}
pub fn btn_group_rename(lang: Lang) -> String {
    match lang {
        Lang::Ru => "✏️ Переименовать",
        Lang::En => "✏️ Rename",
    }
    .to_string()
}
pub fn btn_group_quota(lang: Lang) -> String {
    match lang {
        Lang::Ru => "🔢 Лимит",
        Lang::En => "🔢 Quota",
    }
    .to_string()
}
pub fn ask_group_quota(lang: Lang) -> String {
    match lang {
        Lang::Ru => "Введите лимит клиентов в группе (0 — безлимит):",
        Lang::En => "Enter the client limit for the group (0 — unlimited):",
    }
    .to_string()
}
pub fn bad_group_quota(lang: Lang) -> String {
    match lang {
        Lang::Ru => "⚠️ Некорректное значение. Введите целое число ≥ 0:",
        Lang::En => "⚠️ Invalid value. Enter an integer ≥ 0:",
    }
    .to_string()
}
pub fn group_quota_set(lang: Lang, quota: Option<i64>) -> String {
    match (lang, quota) {
        (Lang::Ru, Some(q)) => format!("✅ Лимит группы: {q}."),
        (Lang::Ru, None) => "✅ Лимит группы снят (безлимит).".to_string(),
        (Lang::En, Some(q)) => format!("✅ Group limit: {q}."),
        (Lang::En, None) => "✅ Group limit removed (unlimited).".to_string(),
    }
}
pub fn btn_group_admins(lang: Lang) -> String {
    match lang {
        Lang::Ru => "👮 Админы",
        Lang::En => "👮 Admins",
    }
    .to_string()
}
pub fn group_admins_title(lang: Lang, group: &str) -> String {
    let group = html_escape(group);
    match lang {
        Lang::Ru => format!("👮 <b>Админы группы {group}</b>:"),
        Lang::En => format!("👮 <b>Admins of group {group}</b>:"),
    }
}
pub fn group_admins_empty(lang: Lang) -> String {
    match lang {
        Lang::Ru => "В группе пока нет админов.",
        Lang::En => "No admins in this group yet.",
    }
    .to_string()
}
pub fn btn_group_invite(lang: Lang) -> String {
    match lang {
        Lang::Ru => "🔗 Пригласить админа",
        Lang::En => "🔗 Invite admin",
    }
    .to_string()
}
/// Текст с одноразовой ссылкой-приглашением и сроком её действия в часах.
pub fn invite_link_text(lang: Lang, url: &str, hours: i64) -> String {
    match lang {
        Lang::Ru => format!(
            "🔗 Одноразовая ссылка-приглашение (действует {hours} ч):\n<code>{url}</code>\nПерешлите её будущему админу группы."
        ),
        Lang::En => format!(
            "🔗 One-time invite link (valid for {hours} h):\n<code>{url}</code>\nForward it to the future group admin."
        ),
    }
}
pub fn btn_invite_revoke(lang: Lang) -> String {
    match lang {
        Lang::Ru => "🚫 Отозвать ссылку",
        Lang::En => "🚫 Revoke link",
    }
    .to_string()
}
pub fn invite_revoked(lang: Lang) -> String {
    match lang {
        Lang::Ru => "🚫 Ссылка отозвана.",
        Lang::En => "🚫 Link revoked.",
    }
    .to_string()
}
pub fn btn_admin_by_id(lang: Lang) -> String {
    match lang {
        Lang::Ru => "🆔 Ввести user ID",
        Lang::En => "🆔 Enter user ID",
    }
    .to_string()
}
pub fn ask_admin_id(lang: Lang) -> String {
    match lang {
        Lang::Ru => "Введите user ID нового админа:",
        Lang::En => "Enter the new admin's user ID:",
    }
    .to_string()
}
pub fn bad_admin_id(lang: Lang) -> String {
    match lang {
        Lang::Ru => "⚠️ Некорректный ID. Введите число:",
        Lang::En => "⚠️ Invalid ID. Enter a number:",
    }
    .to_string()
}
pub fn admin_added(lang: Lang, uid: i64, group: &str) -> String {
    let group = html_escape(group);
    match lang {
        Lang::Ru => format!("✅ Пользователь {uid} добавлен админом группы <b>{group}</b>."),
        Lang::En => format!("✅ User {uid} added as admin of group <b>{group}</b>."),
    }
}
pub fn admin_already(lang: Lang, uid: i64) -> String {
    match lang {
        Lang::Ru => format!("⚠️ Пользователь {uid} уже админ этой группы."),
        Lang::En => format!("⚠️ User {uid} is already an admin of this group."),
    }
}
pub fn admin_removed(lang: Lang, uid: i64) -> String {
    match lang {
        Lang::Ru => format!("🗑 Админ {uid} удалён из группы."),
        Lang::En => format!("🗑 Admin {uid} removed from the group."),
    }
}
/// Приветствие для нового группового админа, присоединившегося по ссылке.
pub fn joined_group(lang: Lang, group: &str) -> String {
    let group = html_escape(group);
    match lang {
        Lang::Ru => format!("🎉 Добро пожаловать! Вы стали админом группы <b>{group}</b>."),
        Lang::En => format!("🎉 Welcome! You are now an admin of group <b>{group}</b>."),
    }
}
pub fn invite_invalid(lang: Lang) -> String {
    match lang {
        Lang::Ru => "⚠️ Ссылка недействительна или истекла.",
        Lang::En => "⚠️ Link is invalid or expired.",
    }
    .to_string()
}
/// Уведомление владельцу о том, что новый пользователь присоединился как
/// админ группы по ссылке-приглашению.
pub fn owner_notified_join(lang: Lang, uid: i64, group: &str) -> String {
    let group = html_escape(group);
    match lang {
        Lang::Ru => format!("ℹ️ Пользователь {uid} присоединился как админ группы <b>{group}</b>."),
        Lang::En => format!("ℹ️ User {uid} joined as admin of group <b>{group}</b>."),
    }
}
pub fn btn_group_delete(lang: Lang) -> String {
    match lang {
        Lang::Ru => "🗑 Удалить группу",
        Lang::En => "🗑 Delete group",
    }
    .to_string()
}
/// Экран выбора: что сделать с клиентами группы перед её удалением.
pub fn group_delete_choice(lang: Lang, name: &str, clients: i64) -> String {
    let name = html_escape(name);
    match lang {
        Lang::Ru => format!(
            "🗑 Удалить группу <b>{name}</b>?\nВ ней {clients} клиент(ов). Выберите, что сделать с клиентами:"
        ),
        Lang::En => format!(
            "🗑 Delete group <b>{name}</b>?\nIt has {clients} client(s). Choose what to do with them:"
        ),
    }
}
pub fn btn_delete_detach(lang: Lang) -> String {
    match lang {
        Lang::Ru => "🔓 Отвязать клиентов",
        Lang::En => "🔓 Detach clients",
    }
    .to_string()
}
pub fn btn_delete_with_clients(lang: Lang) -> String {
    match lang {
        Lang::Ru => "🗑 Удалить с клиентами",
        Lang::En => "🗑 Delete with clients",
    }
    .to_string()
}
/// Второе (усиленное) подтверждение перед необратимым удалением группы
/// вместе со всеми её клиентами.
pub fn confirm_delete_group_clients(lang: Lang, name: &str, clients: i64) -> String {
    let name = html_escape(name);
    match lang {
        Lang::Ru => format!(
            "⚠️ Точно удалить группу <b>{name}</b> вместе со всеми {clients} клиентами? Это необратимо."
        ),
        Lang::En => format!(
            "⚠️ Really delete group <b>{name}</b> along with all {clients} clients? This cannot be undone."
        ),
    }
}
pub fn group_deleted(lang: Lang, name: &str) -> String {
    let name = html_escape(name);
    match lang {
        Lang::Ru => format!("✅ Группа <b>{name}</b> удалена."),
        Lang::En => format!("✅ Group <b>{name}</b> deleted."),
    }
}
pub fn btn_group_regen(lang: Lang) -> String {
    match lang {
        Lang::Ru => "♻️ Перевыпустить всех",
        Lang::En => "♻️ Reissue all",
    }
    .to_string()
}
pub fn confirm_group_regen(lang: Lang, name: &str, clients: i64) -> String {
    let name = html_escape(name);
    match lang {
        Lang::Ru => format!(
            "♻️ <b>Перевыпустить конфиги всех клиентов группы {name}?</b>\nВсего клиентов: {clients}. Файлы и QR будут перегенерированы, ключи и IP сохранятся."
        ),
        Lang::En => format!(
            "♻️ <b>Reissue configs for all clients in group {name}?</b>\nTotal clients: {clients}. Files and QR codes will be regenerated; keys and IPs are preserved."
        ),
    }
}
pub fn group_regen_done(lang: Lang, ok: usize, failed: usize) -> String {
    match lang {
        Lang::Ru if failed == 0 => {
            format!("✅ Перевыпущено клиентов: {ok}.\nКонфиги — в карточках клиентов.")
        }
        Lang::Ru => format!(
            "⚠️ Перевыпущено: {ok}, ошибок: {failed}. Проверьте логи сервера.\nКонфиги — в карточках клиентов."
        ),
        Lang::En if failed == 0 => {
            format!("✅ Reissued {ok} clients.\nConfigs are in the client cards.")
        }
        Lang::En => format!(
            "⚠️ Reissued: {ok}, failed: {failed}. Check the server logs.\nConfigs are in the client cards."
        ),
    }
}
pub fn select_group_title(lang: Lang) -> String {
    match lang {
        Lang::Ru => "Выберите группу:",
        Lang::En => "Choose a group:",
    }
    .to_string()
}
pub fn btn_switch_group(lang: Lang) -> String {
    match lang {
        Lang::Ru => "🔀 Сменить группу",
        Lang::En => "🔀 Switch group",
    }
    .to_string()
}
/// Заголовок главного меню для группового админа (аналог `menu_title` с
/// указанием текущей группы).
pub fn ga_menu_title(lang: Lang, group: &str) -> String {
    let group = html_escape(group);
    match lang {
        Lang::Ru => format!("🔐 <b>AmneziaWG</b> · 🗂 {group}"),
        Lang::En => format!("🔐 <b>AmneziaWG</b> · 🗂 {group}"),
    }
}
pub fn quota_reached(lang: Lang, quota: i64) -> String {
    match lang {
        Lang::Ru => format!("⚠️ Достигнут лимит группы ({quota})."),
        Lang::En => format!("⚠️ Group limit reached ({quota})."),
    }
}
pub fn btn_client_move(lang: Lang) -> String {
    match lang {
        Lang::Ru => "🗂 Группа",
        Lang::En => "🗂 Group",
    }
    .to_string()
}
pub fn move_client_title(lang: Lang, name: &str) -> String {
    let name = html_escape(name);
    match lang {
        Lang::Ru => format!("🗂 Перенести <b>{name}</b> в группу:"),
        Lang::En => format!("🗂 Move <b>{name}</b> to group:"),
    }
}
pub fn no_group_label(lang: Lang) -> String {
    match lang {
        Lang::Ru => "Без группы",
        Lang::En => "No group",
    }
    .to_string()
}
/// Заголовок экрана выбора группового фильтра списка клиентов (владелец).
pub fn scope_title(lang: Lang) -> String {
    match lang {
        Lang::Ru => "Показывать клиентов:",
        Lang::En => "Show clients:",
    }
    .to_string()
}
/// Кнопка «🗂» в ряду фильтров списка клиентов — открывает `scope_title`
/// (только владельцу, см. `menu::clients_list`'s `can_scope`).
pub fn btn_scope(lang: Lang) -> String {
    let _ = lang;
    "🗂".to_string()
}
pub fn btn_scope_all(lang: Lang) -> String {
    match lang {
        Lang::Ru => "Все группы",
        Lang::En => "All groups",
    }
    .to_string()
}
pub fn client_moved(lang: Lang, name: &str, group: Option<&str>) -> String {
    let name = html_escape(name);
    match (lang, group) {
        (Lang::Ru, Some(g)) => {
            format!(
                "✅ Клиент <b>{name}</b> перенесён в группу <b>{}</b>.",
                html_escape(g)
            )
        }
        (Lang::Ru, None) => format!("✅ Клиент <b>{name}</b> откреплён от группы."),
        (Lang::En, Some(g)) => {
            format!(
                "✅ Client <b>{name}</b> moved to group <b>{}</b>.",
                html_escape(g)
            )
        }
        (Lang::En, None) => format!("✅ Client <b>{name}</b> detached from its group."),
    }
}
/// Рекомендация включить ID-префикс имён — показывается после первого
/// добавления группового админа, т.к. клиентов теперь создают несколько
/// человек и возможны совпадения имён между группами.
pub fn slug_recommend(lang: Lang) -> String {
    match lang {
        Lang::Ru => "💡 Теперь клиентов создают несколько людей. Рекомендуем включить префиксы имён (Настройки → Префиксы), чтобы избежать совпадений имён между группами.".to_string(),
        Lang::En => "💡 Multiple people can now create clients. We recommend enabling name prefixes (Settings → Prefixes) to avoid name collisions between groups.".to_string(),
    }
}
pub fn btn_slug_enable(lang: Lang) -> String {
    match lang {
        Lang::Ru => "✅ Включить префиксы",
        Lang::En => "✅ Enable prefixes",
    }
    .to_string()
}
/// Строка «Группа: X» для карточки клиента (`client_card`). Начинается с
/// перевода строки: карточка кончается строкой трафика без `\n`, строка
/// группы конкатенируется к ней напрямую.
pub fn group_label_line(lang: Lang, group: &str) -> String {
    let group = html_escape(group);
    match lang {
        Lang::Ru => format!("\n🗂 Группа: {group}"),
        Lang::En => format!("\n🗂 Group: {group}"),
    }
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
            assert!(!settings_title(l, true, true, true, true, true).is_empty());
            assert!(!backups_empty(l).is_empty());
            assert!(!restore_done(l).is_empty());
            // карточка: имя экранируется
            let card = client_card(
                l,
                "a<b>",
                "🟢",
                "Активен",
                "10.0.0.2",
                "1 KB",
                "0 B",
                "никогда",
                "бессрочно",
            );
            assert!(card.contains("a&lt;b&gt;"));
            assert!(!card.contains("a<b>"));
            assert!(card.contains("🟢")); // цвет статуса передан напрямую (mark)
        }
    }

    #[test]
    fn status_label_mark_known_marks_translated() {
        assert_eq!(status_label_mark(Lang::Ru, "🟢"), "Онлайн");
        assert_eq!(status_label_mark(Lang::En, "🟢"), "Online");
        assert_eq!(status_label_mark(Lang::Ru, "🟡"), "Не подключался");
        assert_eq!(status_label_mark(Lang::En, "🟡"), "Never connected");
        assert_eq!(status_label_mark(Lang::Ru, "🔴"), "Оффлайн");
        assert_eq!(status_label_mark(Lang::En, "🔴"), "Offline");
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
    fn btn_refresh_nonempty_both_langs() {
        for l in [Lang::Ru, Lang::En] {
            assert!(!btn_refresh(l).is_empty());
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

    #[test]
    fn error_text_covers_new_variants() {
        use crate::error::Error;
        for l in [Lang::Ru, Lang::En] {
            for e in [
                Error::ClientNotFound("ghost".into()),
                Error::RestoreRolledBack,
            ] {
                let t = error_text(l, &e);
                assert!(!t.is_empty());
                assert!(!t.contains("ghost")); // имя не утекает
            }
        }
    }

    #[test]
    fn modify_strings_nonempty_both_langs() {
        use crate::vpn::validate::ModifyParam;
        for l in [Lang::Ru, Lang::En] {
            assert!(!btn_modify(l).is_empty());
            assert!(!ask_modify_param(l, ModifyParam::Keepalive).is_empty());
            assert!(!ask_modify_param(l, ModifyParam::Dns).is_empty());
            assert!(!modify_done(l, ModifyParam::Keepalive, "25").is_empty());
            assert!(!modify_param_label(l, ModifyParam::Keepalive).is_empty());
        }
    }

    #[test]
    fn restart_and_repair_strings_nonempty_both_langs() {
        for l in [Lang::Ru, Lang::En] {
            assert!(!btn_restart(l).is_empty());
            assert!(!btn_repair(l).is_empty());
            assert!(!confirm_restart(l).is_empty());
            assert!(!restart_done(l, true).is_empty());
            assert!(!restart_done(l, false).is_empty());
            assert!(!repair_result(l, 0).is_empty());
            assert!(!repair_result(l, 1).is_empty());
            assert!(!repair_result(l, 2).is_empty());
        }
    }

    #[test]
    fn check_card_renders_status_emojis() {
        use crate::vpn::wire::CheckReport;
        let ok = CheckReport {
            ok: true,
            service: crate::vpn::wire::ServiceBlock {
                unit: "awg-quick@awg0".into(),
                active: true,
            },
            interface: crate::vpn::wire::InterfaceBlock {
                name: "awg0".into(),
                present: true,
                mtu: Some(1280),
                addresses: vec!["10.9.9.1/24".into()],
            },
            port: crate::vpn::wire::PortBlock {
                number: 39743,
                proto: "udp".into(),
                listening: true,
            },
            module: crate::vpn::wire::ModuleBlock { loaded: true },
            clients: crate::vpn::wire::ClientsBlock { total: 5 },
            firewall: crate::vpn::wire::FirewallBlock {
                ufw_active: true,
                port_allowed: true,
            },
        };
        let text = check_card(Lang::Ru, &ok);
        assert!(text.contains("✅"));
        assert!(text.contains("39743"));
        assert!(text.contains("5"));

        let bad = CheckReport {
            ok: false,
            service: ok.service.clone(),
            ..ok.clone()
        };
        let bad_text = check_card(Lang::Ru, &bad);
        assert!(bad_text.contains("❌"));
    }

    #[test]
    fn bulk_strings_nonempty_both_langs() {
        for l in [Lang::Ru, Lang::En] {
            assert!(!ask_bulk_prefix(l).is_empty());
            assert!(!ask_bulk_count(l).is_empty());
            assert!(!btn_bulk(l).is_empty());
            assert!(!bulk_creating(l).is_empty());
        }
    }

    #[test]
    fn bad_bulk_prefix_shows_actual_limit() {
        // Лимит зависит от slug-настройки — сообщение должно показывать
        // фактическую границу, а не захардкоженную.
        assert!(bad_bulk_prefix(Lang::Ru, 29).contains("29"));
        assert!(bad_bulk_prefix(Lang::En, 23).contains("23"));
    }

    #[test]
    fn bulk_result_summary_created_only() {
        use crate::vpn::model::{AddResult, BulkResult};
        let res = BulkResult {
            created: vec![
                AddResult {
                    name: "a".into(),
                    conf_path: "/x".into(),
                    qr_path: "".into(),
                    uri: "".into(),
                },
                AddResult {
                    name: "b".into(),
                    conf_path: "/y".into(),
                    qr_path: "".into(),
                    uri: "".into(),
                },
            ],
            skipped: vec![],
        };
        let ru = bulk_result_summary(Lang::Ru, &res);
        let en = bulk_result_summary(Lang::En, &res);
        assert!(ru.contains("2"));
        assert!(en.contains("2"));
    }

    #[test]
    fn bulk_result_summary_with_skipped() {
        use crate::vpn::model::{AddResult, BulkResult, Skip, SkipReason};
        let res = BulkResult {
            created: vec![AddResult {
                name: "a".into(),
                conf_path: "/x".into(),
                qr_path: "".into(),
                uri: "".into(),
            }],
            skipped: vec![Skip {
                name: "b".into(),
                reason: SkipReason::Exists,
            }],
        };
        let ru = bulk_result_summary(Lang::Ru, &res);
        assert!(ru.contains("1"));
        assert!(ru.contains("b"));
    }

    #[test]
    fn capacity_messages_nonempty_both_langs() {
        for l in [Lang::Ru, Lang::En] {
            assert!(!capacity_insufficient(l, 4, 10).is_empty());
            assert!(!capacity_exhausted(l).is_empty());
            assert!(!capacity_unavailable(l).is_empty());
        }
    }

    #[test]
    fn deliver_toggle_buttons_nonempty() {
        for l in [Lang::Ru, Lang::En] {
            assert!(!btn_conf_toggle(l, true).is_empty());
            assert!(!btn_conf_toggle(l, false).is_empty());
            assert!(!btn_qr_toggle(l, true).is_empty());
            assert!(!btn_link_toggle(l, true).is_empty());
        }
    }

    #[test]
    fn card_artifact_buttons_nonempty() {
        for l in [Lang::Ru, Lang::En] {
            assert!(!btn_card_qr(l).is_empty());
            assert!(!btn_card_link(l).is_empty());
            assert!(!btn_card_all(l).is_empty());
        }
    }

    #[test]
    fn artifact_missing_messages_nonempty() {
        for l in [Lang::Ru, Lang::En] {
            assert!(!qr_not_generated(l).is_empty());
            assert!(!link_unavailable(l).is_empty());
        }
    }

    #[test]
    fn clients_title_all_has_no_filter_label() {
        use crate::vpn::model::ClientFilter;
        // All → как clients_title, без пометки фильтра.
        let ru = clients_title_filtered(Lang::Ru, ClientFilter::All, 5, 10);
        let en = clients_title_filtered(Lang::En, ClientFilter::All, 5, 10);
        assert_eq!(ru, "👥 <b>Клиенты</b>:");
        assert_eq!(en, "👥 <b>Clients</b>:");
    }

    #[test]
    fn clients_title_filtered_shows_label_and_count() {
        use crate::vpn::model::ClientFilter;
        let ru = clients_title_filtered(Lang::Ru, ClientFilter::Online, 3, 10);
        assert!(ru.contains("🟢"));
        assert!(ru.contains("онлайн"));
        assert!(ru.contains("(3 из 10)"));
        let en = clients_title_filtered(Lang::En, ClientFilter::Never, 2, 8);
        assert!(en.contains("🟡"));
        assert!(en.contains("never"));
        assert!(en.contains("(2 of 8)"));
    }

    #[test]
    fn clients_title_filtered_shown_equals_total_drops_label() {
        use crate::vpn::model::ClientFilter;
        // Фильтр активен, но показаны все (напр. 3 из 3 онлайн) → без пометки.
        let ru = clients_title_filtered(Lang::Ru, ClientFilter::Online, 3, 3);
        assert_eq!(ru, "👥 <b>Клиенты</b>:");
    }

    #[test]
    fn group_strings_bilingual_and_escaped() {
        for lang in [Lang::Ru, Lang::En] {
            assert!(!btn_groups(lang).is_empty());
            assert!(!ask_group_name(lang).is_empty());
            assert!(!slug_recommend(lang).is_empty());
        }
        // Имя с HTML-символами экранируется в карточках/сообщениях.
        assert!(group_created(Lang::Ru, "<x>").contains("&lt;x&gt;"));
        assert!(group_card(Lang::En, "<x>", 0, None, 0).contains("&lt;x&gt;"));
        // Квота: None → «безлимит», Some → число.
        assert!(group_card(Lang::Ru, "g", 1, Some(5), 1).contains('5'));
    }

    #[test]
    fn group_label_line_starts_on_new_line() {
        // Карточка клиента кончается строкой трафика без \n — строка группы
        // обязана сама начинаться с перевода строки, иначе приклеится к ней.
        for lang in [Lang::Ru, Lang::En] {
            assert!(group_label_line(lang, "g").starts_with('\n'));
        }
    }
}
