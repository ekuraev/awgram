use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

use crate::i18n::{self, Lang};
use crate::vpn::model::{format_handshake_compact, Client, ClientFilter};
use crate::vpn::BackupFile;

fn cb(text: &str, data: &str) -> InlineKeyboardButton {
    InlineKeyboardButton::callback(text.to_string(), data.to_string())
}

pub fn main_menu(lang: Lang) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![cb(&i18n::btn_clients(lang), "list")],
        vec![cb(&i18n::btn_add(lang), "add")],
        vec![cb(&i18n::btn_bulk(lang), "addbulk")],
        vec![cb(&i18n::btn_stats(lang), "stats")],
        vec![cb(&i18n::btn_backup(lang), "backup")],
        vec![
            cb(&i18n::btn_check(lang), "check"),
            cb(&i18n::btn_diagnose(lang), "diagnose"),
        ],
        vec![
            cb(&i18n::btn_restart(lang), "restart"),
            cb(&i18n::btn_repair(lang), "repair"),
        ],
        vec![cb(&i18n::btn_settings(lang), "settings")],
    ])
}

/// Экран выбора языка при первом запуске — показывает оба варианта
/// одновременно (ещё не знаем предпочтение пользователя), без опоры на `lang`.
pub fn language_select() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![
        cb("🇷🇺 Русский", "lang:ru"),
        cb("🇬🇧 English", "lang:en"),
    ]])
}

pub fn settings_menu(
    lang: Lang,
    psk_default: bool,
    name_slug: bool,
    deliver_conf: bool,
    deliver_qr: bool,
    deliver_link: bool,
) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![
            cb(&i18n::btn_lang_ru(lang), "set:lang:ru"),
            cb(&i18n::btn_lang_en(lang), "set:lang:en"),
        ],
        vec![cb(
            &i18n::btn_psk_toggle(lang, psk_default),
            if psk_default {
                "set:psk:off"
            } else {
                "set:psk:on"
            },
        )],
        vec![cb(
            &i18n::btn_slug_toggle(lang, name_slug),
            if name_slug {
                "set:slug:off"
            } else {
                "set:slug:on"
            },
        )],
        vec![cb(
            &i18n::btn_conf_toggle(lang, deliver_conf),
            if deliver_conf {
                "set:conf:off"
            } else {
                "set:conf:on"
            },
        )],
        vec![cb(
            &i18n::btn_qr_toggle(lang, deliver_qr),
            if deliver_qr {
                "set:qr:off"
            } else {
                "set:qr:on"
            },
        )],
        vec![cb(
            &i18n::btn_link_toggle(lang, deliver_link),
            if deliver_link {
                "set:link:off"
            } else {
                "set:link:on"
            },
        )],
        vec![cb(&i18n::btn_back(lang), "menu")],
    ])
}

// Подписи пресетов срока действия не входят в каталог `i18n` (см. brief задачи
// 5) — локализуются здесь напрямую, без изменения `i18n.rs`.
fn day_label(lang: Lang, days: u32) -> String {
    match lang {
        Lang::Ru => format!("{days}д"),
        Lang::En => format!("{days}d"),
    }
}

pub fn expiry_menu(lang: Lang) -> InlineKeyboardMarkup {
    let none_txt = match lang {
        Lang::Ru => "Без срока",
        Lang::En => "No expiry",
    };
    let custom_txt = match lang {
        Lang::Ru => "✏️ Свой",
        Lang::En => "✏️ Custom",
    };
    InlineKeyboardMarkup::new(vec![
        vec![cb(none_txt, "exp:none")],
        vec![
            cb(&day_label(lang, 1), "exp:1d"),
            cb(&day_label(lang, 7), "exp:7d"),
            cb(&day_label(lang, 14), "exp:14d"),
        ],
        vec![
            cb(&day_label(lang, 30), "exp:30d"),
            cb(&day_label(lang, 90), "exp:90d"),
            cb(&day_label(lang, 180), "exp:180d"),
        ],
        vec![
            cb(&day_label(lang, 365), "exp:365d"),
            cb(custom_txt, "exp:custom"),
        ],
    ])
}

/// Экран выбора количества для массовой генерации: пресеты 1/3/5/10 (cap=10 —
/// лимит альбома Telegram). Callback `bulk:N`.
pub fn bulk_count_menu(lang: Lang) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![
            cb("1", "bulk:1"),
            cb("3", "bulk:3"),
            cb("5", "bulk:5"),
            cb("10", "bulk:10"),
        ],
        vec![cb(&i18n::btn_back(lang), "menu")],
    ])
}

/// Экран выбора срока для массовой генерации. Копия `expiry_menu`, но с
/// `bulkexp:` префиксами (чтобы bulk- и одиночный-потоки шли через разные
/// Action без условной логики в общем обработчике `Expiry`).
pub fn bulk_expiry_menu(lang: Lang) -> InlineKeyboardMarkup {
    let none_txt = match lang {
        Lang::Ru => "Без срока",
        Lang::En => "No expiry",
    };
    let custom_txt = match lang {
        Lang::Ru => "✏️ Свой",
        Lang::En => "✏️ Custom",
    };
    InlineKeyboardMarkup::new(vec![
        vec![cb(none_txt, "bulkexp:none")],
        vec![
            cb(&day_label(lang, 1), "bulkexp:1d"),
            cb(&day_label(lang, 7), "bulkexp:7d"),
            cb(&day_label(lang, 14), "bulkexp:14d"),
        ],
        vec![
            cb(&day_label(lang, 30), "bulkexp:30d"),
            cb(&day_label(lang, 90), "bulkexp:90d"),
            cb(&day_label(lang, 180), "bulkexp:180d"),
        ],
        vec![
            cb(&day_label(lang, 365), "bulkexp:365d"),
            cb(custom_txt, "bulkexp:custom"),
        ],
    ])
}

/// Шаг выбора PSK в диалоге `add` — дефолтная опция (по настройке
/// `settings.psk_default()`) идёт первой кнопкой.
pub fn psk_step(lang: Lang, default_on: bool) -> InlineKeyboardMarkup {
    let (first, second) = if default_on {
        (
            cb(&i18n::btn_create_with_psk(lang), "add:psk:on"),
            cb(&i18n::btn_create_no_psk(lang), "add:psk:off"),
        )
    } else {
        (
            cb(&i18n::btn_create_no_psk(lang), "add:psk:off"),
            cb(&i18n::btn_create_with_psk(lang), "add:psk:on"),
        )
    };
    InlineKeyboardMarkup::new(vec![
        vec![first, second],
        vec![cb(&i18n::btn_back(lang), "menu")],
    ])
}

/// Шаг выбора PSK в bulk-диалоге — как `psk_step`, но с `bulkadd:psk:` callback'ами
/// (чтобы попасть в Action::AddBulkPsk, а не в одиночный Action::AddPsk).
pub fn bulk_psk_step(lang: Lang, default_on: bool) -> InlineKeyboardMarkup {
    let (first, second) = if default_on {
        (
            cb(&i18n::btn_create_with_psk(lang), "bulkadd:psk:on"),
            cb(&i18n::btn_create_no_psk(lang), "bulkadd:psk:off"),
        )
    } else {
        (
            cb(&i18n::btn_create_no_psk(lang), "bulkadd:psk:off"),
            cb(&i18n::btn_create_with_psk(lang), "bulkadd:psk:on"),
        )
    };
    InlineKeyboardMarkup::new(vec![
        vec![first, second],
        vec![cb(&i18n::btn_back(lang), "menu")],
    ])
}

/// Ряд фильтра списка: [Все] [🟢 Онлайн] [🔴 Оффлайн] [🟡 Никогда].
/// Активный фильтр помечается ✅-префиксом. Подписи локализуются здесь
/// (как day_label), не входят в каталог i18n. Callback `listfilter:{as_str}`.
fn filter_label(lang: Lang, f: ClientFilter) -> String {
    let mark = f.mark();
    let name = match (lang, f) {
        (Lang::Ru, ClientFilter::All) => "Все",
        (Lang::En, ClientFilter::All) => "All",
        (Lang::Ru, ClientFilter::Online) => "Онлайн",
        (Lang::En, ClientFilter::Online) => "Online",
        (Lang::Ru, ClientFilter::Offline) => "Оффлайн",
        (Lang::En, ClientFilter::Offline) => "Offline",
        (Lang::Ru, ClientFilter::Never) => "Никогда",
        (Lang::En, ClientFilter::Never) => "Never",
    };
    format!("{mark} {name}")
}

fn filter_row(lang: Lang, current: ClientFilter) -> Vec<InlineKeyboardButton> {
    [
        ClientFilter::All,
        ClientFilter::Online,
        ClientFilter::Offline,
        ClientFilter::Never,
    ]
    .iter()
    .map(|&f| {
        // Активный фильтр помечается ✅ — кнопка-переключатель, неубираемый
        // индикатор текущего режима просмотра списка.
        let prefix = if f == current { "✅ " } else { "" };
        cb(
            &format!("{prefix}{}", filter_label(lang, f)),
            &format!("listfilter:{}", f.as_str()),
        )
    })
    .collect()
}

pub fn clients_list(
    lang: Lang,
    clients: &[Client],
    expiries: &[Option<i64>],
    now: i64,
    page: usize,
    per_page: usize,
    current_filter: ClientFilter,
) -> InlineKeyboardMarkup {
    if per_page == 0 {
        return InlineKeyboardMarkup::new(vec![vec![cb(&i18n::btn_back(lang), "menu")]]);
    }

    let start = page * per_page;
    let mut rows: Vec<Vec<InlineKeyboardButton>> = clients
        .iter()
        .enumerate()
        .skip(start)
        .take(per_page)
        .map(|(i, c)| {
            let mark = c.mark(now);
            // Компактный handshake («2 мин», «никогда») — требуется stats()
            // (last_handshake есть только в stats --json, не в list --json).
            let hs = format_handshake_compact(lang, now, c.last_handshake.unwrap_or(0));
            let exp = expiries.get(i).copied().flatten();
            let label = match crate::vpn::model::format_expiry_badge(lang, now, exp) {
                Some(badge) => format!("{mark} {} · {hs} {badge}", c.name),
                None => format!("{mark} {} · {hs}", c.name),
            };
            vec![cb(&label, &format!("client:{}", c.name))]
        })
        .collect();

    let total_pages = clients.len().div_ceil(per_page).max(1);
    // 🔄 всегда в nav-ряду: перерисовывает ТЕКУЩУЮ страницу со свежими данными.
    // Callback `page:{page}` → Action::Page (он заново зовёт vpn.stats() —
    // список переключён на stats ради last_handshake в кнопках), поэтому
    // refresh сохраняет страницу, а не сбрасывает на 0. На одностраничном списке
    // это единственная кнопка ряда; на многостраничном встаёт между пагинацией:
    // [◀️] [🔄] [▶️].
    let mut nav = Vec::new();
    if page > 0 {
        nav.push(cb("◀️", &format!("page:{}", page - 1)));
    }
    nav.push(cb(&i18n::btn_refresh(lang), &format!("page:{page}")));
    if page + 1 < total_pages {
        nav.push(cb("▶️", &format!("page:{}", page + 1)));
    }
    rows.push(nav);
    rows.push(filter_row(lang, current_filter));
    rows.push(vec![cb(&i18n::btn_regen_all(lang), "regen_all")]);
    rows.push(vec![cb(&i18n::btn_back(lang), "menu")]);
    InlineKeyboardMarkup::new(rows)
}

pub fn client_card(lang: Lang, name: &str) -> InlineKeyboardMarkup {
    let conf_txt = match lang {
        Lang::Ru => "📄 Конфиг",
        Lang::En => "📄 Config",
    };
    let del_txt = match lang {
        Lang::Ru => "🗑 Удалить",
        Lang::En => "🗑 Delete",
    };
    InlineKeyboardMarkup::new(vec![
        vec![
            cb(conf_txt, &format!("conf:{name}")),
            cb(&i18n::btn_card_qr(lang), &format!("qr:{name}")),
        ],
        vec![
            cb(&i18n::btn_card_link(lang), &format!("uri:{name}")),
            cb(&i18n::btn_card_all(lang), &format!("all:{name}")),
        ],
        vec![
            cb(&i18n::btn_regen(lang), &format!("regen:{name}")),
            cb(del_txt, &format!("del:{name}")),
        ],
        vec![cb(&i18n::btn_modify(lang), &format!("mod:{name}"))],
        vec![cb(&i18n::btn_back(lang), "menu")],
    ])
}

pub fn confirm_delete(lang: Lang, name: &str) -> InlineKeyboardMarkup {
    let yes_txt = match lang {
        Lang::Ru => "✅ Да, удалить",
        Lang::En => "✅ Yes, delete",
    };
    InlineKeyboardMarkup::new(vec![vec![
        cb(yes_txt, &format!("delyes:{name}")),
        cb(&i18n::btn_back(lang), "menu"),
    ]])
}

pub fn confirm_recreate(lang: Lang, name: &str) -> InlineKeyboardMarkup {
    let yes_txt = match lang {
        Lang::Ru => "♻️ Пересоздать",
        Lang::En => "♻️ Recreate",
    };
    InlineKeyboardMarkup::new(vec![vec![
        cb(yes_txt, &format!("recreate:{name}")),
        cb(&i18n::btn_back(lang), "menu"),
    ]])
}

pub fn confirm_regen_all(lang: Lang) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![cb(&i18n::btn_regen_all_go(lang), "regen_all_go")],
        vec![cb(&i18n::btn_regen_all_routes(lang), "regen_all_routes")],
        vec![cb(&i18n::btn_back(lang), "menu")],
    ])
}

pub fn modify_param_menu(lang: Lang, name: &str) -> InlineKeyboardMarkup {
    use crate::vpn::validate::ModifyParam;
    InlineKeyboardMarkup::new(vec![
        vec![
            cb(
                &i18n::modify_param_label(lang, ModifyParam::Keepalive),
                &format!("modparam:{name}:keepalive"),
            ),
            cb(
                &i18n::modify_param_label(lang, ModifyParam::Dns),
                &format!("modparam:{name}:dns"),
            ),
        ],
        vec![
            cb(
                &i18n::modify_param_label(lang, ModifyParam::AllowedIps),
                &format!("modparam:{name}:allowedips"),
            ),
            cb(
                &i18n::modify_param_label(lang, ModifyParam::Endpoint),
                &format!("modparam:{name}:endpoint"),
            ),
        ],
        vec![cb(&i18n::btn_back(lang), "menu")],
    ])
}

pub fn confirm_restart_menu(lang: Lang) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![cb(&i18n::btn_restart_go(lang), "restart_go")],
        vec![cb(&i18n::btn_back(lang), "menu")],
    ])
}

pub fn backup_menu(lang: Lang) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![cb(&i18n::btn_backup_new(lang), "bk:new")],
        vec![cb(&i18n::btn_backup_list(lang), "bk:list")],
        vec![cb(&i18n::btn_back(lang), "menu")],
    ])
}

/// Один ряд на бэкап, кнопка ведёт на карточку по индексу в `list_backups()`.
/// Имя файла — обычный текст кнопки (Telegram не рендерит в кнопках HTML,
/// экранирование здесь не нужно, в отличие от текста сообщений).
pub fn backups_list(lang: Lang, backups: &[BackupFile]) -> InlineKeyboardMarkup {
    let mut rows: Vec<Vec<InlineKeyboardButton>> = backups
        .iter()
        .enumerate()
        .map(|(idx, bf)| vec![cb(&bf.name, &format!("bk:card:{idx}"))])
        .collect();
    rows.push(vec![cb(&i18n::btn_back(lang), "menu")]);
    InlineKeyboardMarkup::new(rows)
}

pub fn backup_card(lang: Lang, idx: usize) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![
            cb(&i18n::btn_download(lang), &format!("bk:dl:{idx}")),
            cb(&i18n::btn_restore(lang), &format!("bk:restore:{idx}")),
        ],
        vec![cb(&i18n::btn_back(lang), "menu")],
    ])
}

pub fn confirm_restore(lang: Lang, idx: usize) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![
        cb(&i18n::btn_confirm(lang), &format!("bk:restore_yes:{idx}")),
        cb(&i18n::btn_back(lang), "menu"),
    ]])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_callback_data(kb: &InlineKeyboardMarkup) -> Vec<String> {
        kb.inline_keyboard
            .iter()
            .flatten()
            .filter_map(|b| match &b.kind {
                teloxide::types::InlineKeyboardButtonKind::CallbackData(d) => Some(d.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn main_menu_has_expected_actions() {
        let data = all_callback_data(&main_menu(Lang::Ru));
        for expected in [
            "list", "add", "addbulk", "stats", "backup", "check", "diagnose", "restart", "repair",
            "settings",
        ] {
            assert!(data.contains(&expected.to_string()), "missing {expected}");
        }
    }

    #[test]
    fn bulk_psk_step_emits_bulkadd_callbacks() {
        let data = all_callback_data(&bulk_psk_step(Lang::Ru, true));
        assert!(data.contains(&"bulkadd:psk:on".to_string()));
        assert!(!data.iter().any(|d| d.starts_with("add:psk:")));
    }

    #[test]
    fn main_menu_has_restart_and_repair() {
        let data = all_callback_data(&main_menu(Lang::Ru));
        assert!(data.contains(&"restart".to_string()));
        assert!(data.contains(&"repair".to_string()));
    }

    #[test]
    fn client_card_has_modify_button() {
        let data = all_callback_data(&client_card(Lang::Ru, "alice"));
        assert!(data.contains(&"mod:alice".to_string()));
    }

    #[test]
    fn modify_param_menu_has_four_params_and_back() {
        let data = all_callback_data(&modify_param_menu(Lang::Ru, "alice"));
        assert!(data.contains(&"modparam:alice:keepalive".to_string()));
        assert!(data.contains(&"modparam:alice:dns".to_string()));
        assert!(data.contains(&"modparam:alice:allowedips".to_string()));
        assert!(data.contains(&"modparam:alice:endpoint".to_string()));
        assert!(data.contains(&"menu".to_string()));
    }

    #[test]
    fn confirm_restart_menu_has_go_and_back() {
        let data = all_callback_data(&confirm_restart_menu(Lang::Ru));
        assert!(data.contains(&"restart_go".to_string()));
        assert!(data.contains(&"menu".to_string()));
    }

    #[test]
    fn expiry_menu_has_custom_and_presets() {
        let data = all_callback_data(&expiry_menu(Lang::Ru));
        assert!(data.contains(&"exp:none".to_string()));
        assert!(data.contains(&"exp:30d".to_string()));
        assert!(data.contains(&"exp:custom".to_string()));
    }

    #[test]
    fn client_card_encodes_name() {
        let data = all_callback_data(&client_card(Lang::Ru, "alice"));
        assert!(data.contains(&"conf:alice".to_string()));
        assert!(data.contains(&"regen:alice".to_string()));
    }

    #[test]
    fn confirm_delete_encodes_name() {
        let data = all_callback_data(&confirm_delete(Lang::Ru, "bob"));
        assert!(data.contains(&"delyes:bob".to_string()));
    }

    #[test]
    fn confirm_recreate_encodes_name() {
        let data = all_callback_data(&confirm_recreate(Lang::Ru, "bob"));
        assert!(data.contains(&"recreate:bob".to_string()));
        assert!(data.contains(&"menu".to_string()));
    }

    #[test]
    fn clients_list_has_regen_all_button() {
        let clients = vec![Client {
            name: "a".into(),
            ip: String::new(),
            client_ipv6: String::new(),
            status: String::new(),
            status_code: "active".into(),
            rx: 0,
            tx: 0,
            last_handshake: None,
        }];
        let data = all_callback_data(&clients_list(
            Lang::Ru,
            &clients,
            &[],
            0,
            0,
            10,
            ClientFilter::All,
        ));
        assert!(data.contains(&"regen_all".to_string()));
    }

    #[test]
    fn clients_list_has_refresh_button() {
        // 🔄 «Обновить» эмитит `page:{page}` (Action::Page → edit-in-place с сохранением
        // текущей страницы). На странице 0 это `page:0`. Кнопка должна быть всегда —
        // даже на одностраничном списке (иначе обновить статусы нельзя).
        let clients = vec![Client {
            name: "a".into(),
            ip: String::new(),
            client_ipv6: String::new(),
            status: String::new(),
            status_code: "active".into(),
            rx: 0,
            tx: 0,
            last_handshake: None,
        }];
        let data = all_callback_data(&clients_list(
            Lang::Ru,
            &clients,
            &[],
            0,
            0,
            10,
            ClientFilter::All,
        ));
        assert!(
            data.contains(&"page:0".to_string()),
            "refresh button (page:0) missing: {data:?}"
        );
    }

    #[test]
    fn clients_list_refresh_between_pagination() {
        // На многостраничном списке nav-ряд выглядит [◀️] [🔄] [▶️]:
        let clients: Vec<Client> = (0..20)
            .map(|i| Client {
                name: format!("c{i}"),
                ip: String::new(),
                client_ipv6: String::new(),
                status: String::new(),
                status_code: "active".into(),
                rx: 0,
                tx: 0,
                last_handshake: None,
            })
            .collect();
        let kb = clients_list(Lang::Ru, &clients, &[], 0, 0, 8, ClientFilter::All);
        // nav-ряд — первый после клиентских (8 клиентов на странице → ряд с индексом 8).
        let nav_row = &kb.inline_keyboard[8];
        let data: Vec<&str> = nav_row
            .iter()
            .filter_map(|b| match &b.kind {
                teloxide::types::InlineKeyboardButtonKind::CallbackData(d) => Some(d.as_str()),
                _ => None,
            })
            .collect();
        // [🔄 page:0] [▶️ page:1] — refresh на странице 0 сохраняет её:
        assert_eq!(data, vec!["page:0", "page:1"]);
    }

    #[test]
    fn clients_list_refresh_preserves_page() {
        // 🔄 на странице N эмитит `page:N` — обновляет данные, не сбрасывая на 0.
        // 24 клиента / 8 на странице → 3 страницы; на странице 2 nav-ряд:
        // [◀️ page:1] [🔄 page:2] (▶️ нет, т.к. последняя страница).
        let clients: Vec<Client> = (0..24)
            .map(|i| Client {
                name: format!("c{i}"),
                ip: String::new(),
                client_ipv6: String::new(),
                status: String::new(),
                status_code: "active".into(),
                rx: 0,
                tx: 0,
                last_handshake: None,
            })
            .collect();
        let kb = clients_list(Lang::Ru, &clients, &[], 0, 2, 8, ClientFilter::All);
        // Страница 2: клиентские ряды 16..23 (8 шт.) → nav-ряд с индексом 8.
        let nav_row = &kb.inline_keyboard[8];
        let data: Vec<&str> = nav_row
            .iter()
            .filter_map(|b| match &b.kind {
                teloxide::types::InlineKeyboardButtonKind::CallbackData(d) => Some(d.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(data, vec!["page:1", "page:2"]);
    }

    #[test]
    fn confirm_regen_all_has_three_actions() {
        let data = all_callback_data(&confirm_regen_all(Lang::Ru));
        assert!(data.contains(&"regen_all_go".to_string()));
        assert!(data.contains(&"regen_all_routes".to_string()));
        assert!(data.contains(&"menu".to_string()));
    }

    fn all_button_texts(kb: &InlineKeyboardMarkup) -> Vec<String> {
        kb.inline_keyboard
            .iter()
            .flatten()
            .map(|b| b.text.clone())
            .collect()
    }

    #[test]
    fn clients_list_one_button_per_client() {
        let clients = vec![
            Client {
                name: "a".into(),
                ip: String::new(),
                client_ipv6: String::new(),
                status: String::new(),
                status_code: "active".into(),
                rx: 0,
                tx: 0,
                last_handshake: None,
            },
            Client {
                name: "b".into(),
                ip: String::new(),
                client_ipv6: String::new(),
                status: String::new(),
                status_code: "inactive".into(),
                rx: 0,
                tx: 0,
                last_handshake: None,
            },
        ];
        let data = all_callback_data(&clients_list(
            Lang::Ru,
            &clients,
            &[],
            0,
            0,
            10,
            ClientFilter::All,
        ));
        assert!(data.contains(&"client:a".to_string()));
        assert!(data.contains(&"client:b".to_string()));
    }

    #[test]
    fn clients_list_zero_per_page_no_panic() {
        // Test with empty clients
        let empty_clients: Vec<Client> = vec![];
        let kb_empty = clients_list(Lang::Ru, &empty_clients, &[], 0, 0, 0, ClientFilter::All);
        let data_empty = all_callback_data(&kb_empty);
        assert_eq!(
            data_empty,
            vec!["menu"],
            "empty clients with per_page=0 should have only menu callback"
        );

        // Test with non-empty clients
        let clients = vec![
            Client {
                name: "a".into(),
                ip: String::new(),
                client_ipv6: String::new(),
                status: String::new(),
                status_code: "active".into(),
                rx: 0,
                tx: 0,
                last_handshake: None,
            },
            Client {
                name: "b".into(),
                ip: String::new(),
                client_ipv6: String::new(),
                status: String::new(),
                status_code: "inactive".into(),
                rx: 0,
                tx: 0,
                last_handshake: None,
            },
        ];
        let kb_filled = clients_list(Lang::Ru, &clients, &[], 0, 0, 0, ClientFilter::All);
        let data_filled = all_callback_data(&kb_filled);
        assert_eq!(
            data_filled,
            vec!["menu"],
            "non-empty clients with per_page=0 should have only menu callback"
        );
    }

    #[test]
    fn clients_list_shows_expiry_badge() {
        let clients = vec![
            Client {
                name: "temp".into(),
                ip: String::new(),
                client_ipv6: String::new(),
                status: String::new(),
                status_code: "active".into(),
                rx: 0,
                tx: 0,
                last_handshake: None,
            },
            Client {
                name: "perm".into(),
                ip: String::new(),
                client_ipv6: String::new(),
                status: String::new(),
                status_code: "active".into(),
                rx: 0,
                tx: 0,
                last_handshake: None,
            },
        ];
        let now = 1_700_000_000;
        let expiries = vec![Some(now + 6 * 86400), None];
        let texts = all_button_texts(&clients_list(
            Lang::Ru,
            &clients,
            &expiries,
            now,
            0,
            10,
            ClientFilter::All,
        ));
        assert!(
            texts
                .iter()
                .any(|t| t.contains("temp") && t.contains("⏳ 6д")),
            "temp должен иметь метку: {texts:?}"
        );
        assert!(
            texts
                .iter()
                .any(|t| t.contains("perm") && !t.contains("⏳")),
            "perm должен быть без метки: {texts:?}"
        );
    }

    #[test]
    fn clients_list_three_color_marks_by_status_code() {
        // 🟢 недавний handshake / 🟡 никогда не подключался / 🔴 handshake давно —
        // трёхцветная индикация, цвет считает бот из last_handshake (см. model::mark).
        let now = 1_700_000_000;
        let clients = vec![
            Client {
                name: "online".into(),
                ip: String::new(),
                client_ipv6: String::new(),
                status: String::new(),
                status_code: "active".into(),
                rx: 0,
                tx: 0,
                last_handshake: Some(now - 30), // недавно — онлайн
            },
            Client {
                name: "never".into(),
                ip: String::new(),
                client_ipv6: String::new(),
                status: String::new(),
                status_code: "no_handshake".into(),
                rx: 0,
                tx: 0,
                last_handshake: None,
            },
            Client {
                name: "gone".into(),
                ip: String::new(),
                client_ipv6: String::new(),
                status: String::new(),
                status_code: "inactive".into(),
                rx: 0,
                tx: 0,
                last_handshake: Some(now - 6 * 3600), // был, но давно
            },
        ];
        let texts = all_button_texts(&clients_list(
            Lang::Ru,
            &clients,
            &[],
            now,
            0,
            10,
            ClientFilter::All,
        ));
        assert!(
            texts.iter().any(|t| t.starts_with("🟢 online")),
            "active должен быть зелёным: {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t.starts_with("🟡 never")),
            "no_handshake должен быть жёлтым: {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t.starts_with("🔴 gone")),
            "inactive должен быть красным: {texts:?}"
        );
    }

    #[test]
    fn clients_list_shows_compact_handshake() {
        // handshake в кнопке — компактно («10 мин», «никогда»); last_handshake
        // приходит из stats --json (экран списка переключён на stats).
        let now = 1_700_000_000;
        let clients = vec![
            Client {
                name: "recent".into(),
                ip: String::new(),
                client_ipv6: String::new(),
                status: String::new(),
                status_code: "active".into(),
                rx: 0,
                tx: 0,
                last_handshake: Some(now - 600),
            },
            Client {
                name: "fresh".into(),
                ip: String::new(),
                client_ipv6: String::new(),
                status: String::new(),
                status_code: "no_handshake".into(),
                rx: 0,
                tx: 0,
                last_handshake: Some(0),
            },
        ];
        let texts = all_button_texts(&clients_list(
            Lang::Ru,
            &clients,
            &[],
            now,
            0,
            10,
            ClientFilter::All,
        ));
        assert!(
            texts
                .iter()
                .any(|t| t.contains("recent") && t.contains("10 мин")),
            "recent должен показывать handshake: {texts:?}"
        );
        assert!(
            texts
                .iter()
                .any(|t| t.contains("fresh") && t.contains("никогда")),
            "fresh (last_handshake=0) должен показывать «никогда»: {texts:?}"
        );
    }

    #[test]
    fn clients_list_has_filter_row_with_four_buttons() {
        // Ряд фильтра: [Все] [🟢 Онлайн] [🔴 Оффлайн] [🟡 Никогда] —
        // четыре callback `listfilter:*`.
        let clients = vec![Client {
            name: "a".into(),
            ip: String::new(),
            client_ipv6: String::new(),
            status: String::new(),
            status_code: "active".into(),
            rx: 0,
            tx: 0,
            last_handshake: None,
        }];
        let data = all_callback_data(&clients_list(
            Lang::Ru,
            &clients,
            &[],
            0,
            0,
            10,
            ClientFilter::All,
        ));
        assert!(data.contains(&"listfilter:all".to_string()));
        assert!(data.contains(&"listfilter:online".to_string()));
        assert!(data.contains(&"listfilter:offline".to_string()));
        assert!(data.contains(&"listfilter:never".to_string()));
    }

    #[test]
    fn clients_list_marks_active_filter_with_checkmark() {
        // Активный фильтр помечается ✅-префиксом в подписи кнопки.
        let clients = vec![Client {
            name: "a".into(),
            ip: String::new(),
            client_ipv6: String::new(),
            status: String::new(),
            status_code: "active".into(),
            rx: 0,
            tx: 0,
            last_handshake: None,
        }];
        let texts_online = all_button_texts(&clients_list(
            Lang::Ru,
            &clients,
            &[],
            0,
            0,
            10,
            ClientFilter::Online,
        ));
        assert!(
            texts_online
                .iter()
                .any(|t| t.contains("✅") && t.contains("Онлайн")),
            "активный фильтр Online должен иметь ✅: {texts_online:?}"
        );
        // Все остальные фильтры — без ✅
        assert!(
            texts_online
                .iter()
                .filter(|t| t.contains("Оффлайн"))
                .all(|t| !t.contains("✅")),
            "неактивные фильтры не должны иметь ✅: {texts_online:?}"
        );
    }

    #[test]
    fn language_select_has_both_langs() {
        let data = all_callback_data(&language_select());
        assert!(data.contains(&"lang:ru".to_string()));
        assert!(data.contains(&"lang:en".to_string()));
    }

    #[test]
    fn settings_menu_toggles_psk_data_by_current_value() {
        let data_off = all_callback_data(&settings_menu(Lang::Ru, false, false, true, true, true));
        assert!(data_off.contains(&"set:psk:on".to_string()));
        assert!(!data_off.contains(&"set:psk:off".to_string()));
        let data_on = all_callback_data(&settings_menu(Lang::Ru, true, false, true, true, true));
        assert!(data_on.contains(&"set:psk:off".to_string()));
        assert!(data_on.contains(&"set:lang:ru".to_string()));
        assert!(data_on.contains(&"set:lang:en".to_string()));
        assert!(data_on.contains(&"menu".to_string()));
    }

    #[test]
    fn settings_menu_toggles_slug_data_by_current_value() {
        let data_off = all_callback_data(&settings_menu(Lang::Ru, false, false, true, true, true));
        assert!(data_off.contains(&"set:slug:on".to_string()));
        let data_on = all_callback_data(&settings_menu(Lang::Ru, false, true, true, true, true));
        assert!(data_on.contains(&"set:slug:off".to_string()));
    }

    #[test]
    fn client_card_has_four_artifact_buttons() {
        let data = all_callback_data(&client_card(Lang::Ru, "alice"));
        assert!(data.contains(&"conf:alice".to_string()));
        assert!(data.contains(&"qr:alice".to_string()));
        assert!(data.contains(&"uri:alice".to_string()));
        assert!(data.contains(&"all:alice".to_string()));
        assert!(data.contains(&"del:alice".to_string()));
    }

    #[test]
    fn bulk_count_menu_has_presets_and_back() {
        let data = all_callback_data(&bulk_count_menu(Lang::Ru));
        // пресеты 1/3/5/10 — кодируем как bulk:N
        assert!(data.contains(&"bulk:1".to_string()));
        assert!(data.contains(&"bulk:3".to_string()));
        assert!(data.contains(&"bulk:5".to_string()));
        assert!(data.contains(&"bulk:10".to_string()));
        assert!(data.contains(&"menu".to_string()));
    }

    #[test]
    fn settings_menu_has_deliver_toggles() {
        let data = all_callback_data(&settings_menu(Lang::Ru, true, false, true, false, true));
        assert!(data.contains(&"set:conf:off".to_string())); // on → эмитит off
        assert!(!data.contains(&"set:conf:on".to_string()));
        assert!(data.contains(&"set:qr:on".to_string())); // off → эмитит on
        assert!(data.contains(&"set:link:off".to_string())); // on → эмитит off
    }

    #[test]
    fn psk_step_has_both_options_and_back() {
        let data = all_callback_data(&psk_step(Lang::Ru, false));
        assert!(data.contains(&"add:psk:on".to_string()));
        assert!(data.contains(&"add:psk:off".to_string()));
        assert!(data.contains(&"menu".to_string()));
    }

    #[test]
    fn bulk_expiry_menu_uses_bulkexp_prefix() {
        // Копия expiry_menu, но все callback'и под `bulkexp:` — bulk-поток идёт
        // через отдельный Action без условной логики в общем обработчике Expiry.
        let data = all_callback_data(&bulk_expiry_menu(Lang::Ru));
        assert!(data.contains(&"bulkexp:none".to_string()));
        assert!(data.contains(&"bulkexp:1d".to_string()));
        assert!(data.contains(&"bulkexp:30d".to_string()));
        assert!(data.contains(&"bulkexp:365d".to_string()));
        assert!(data.contains(&"bulkexp:custom".to_string()));
        // Подтверждаем, что bulk-экран НЕ пересекается с одиночным потоком:
        assert!(!data.iter().any(|d| d.starts_with("exp:")));
    }

    #[test]
    fn backup_menu_has_new_list_and_back() {
        let data = all_callback_data(&backup_menu(Lang::Ru));
        assert!(data.contains(&"bk:new".to_string()));
        assert!(data.contains(&"bk:list".to_string()));
        assert!(data.contains(&"menu".to_string()));
    }

    #[test]
    fn backups_list_one_button_per_backup_by_index() {
        let backups = vec![
            BackupFile {
                name: "a.tar.gz".into(),
                path: "a.tar.gz".into(),
                size: 1,
                mtime: 1,
            },
            BackupFile {
                name: "b.tar.gz".into(),
                path: "b.tar.gz".into(),
                size: 2,
                mtime: 2,
            },
        ];
        let data = all_callback_data(&backups_list(Lang::Ru, &backups));
        assert!(data.contains(&"bk:card:0".to_string()));
        assert!(data.contains(&"bk:card:1".to_string()));
        assert!(data.contains(&"menu".to_string()));
    }

    #[test]
    fn backup_card_encodes_index() {
        let data = all_callback_data(&backup_card(Lang::Ru, 2));
        assert!(data.contains(&"bk:dl:2".to_string()));
        assert!(data.contains(&"bk:restore:2".to_string()));
        assert!(data.contains(&"menu".to_string()));
    }

    #[test]
    fn confirm_restore_encodes_index() {
        let data = all_callback_data(&confirm_restore(Lang::Ru, 3));
        assert!(data.contains(&"bk:restore_yes:3".to_string()));
        assert!(data.contains(&"menu".to_string()));
    }

    #[test]
    fn psk_step_default_option_listed_first() {
        let kb_off = psk_step(Lang::Ru, false);
        let first_row_off = &kb_off.inline_keyboard[0];
        match &first_row_off[0].kind {
            teloxide::types::InlineKeyboardButtonKind::CallbackData(d) => {
                assert_eq!(d, "add:psk:off")
            }
            _ => panic!("expected callback data"),
        }

        let kb_on = psk_step(Lang::Ru, true);
        let first_row_on = &kb_on.inline_keyboard[0];
        match &first_row_on[0].kind {
            teloxide::types::InlineKeyboardButtonKind::CallbackData(d) => {
                assert_eq!(d, "add:psk:on")
            }
            _ => panic!("expected callback data"),
        }
    }
}
