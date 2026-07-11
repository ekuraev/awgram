use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

use crate::i18n::{self, Lang};
use crate::vpn::model::Client;
use crate::vpn::BackupFile;

fn cb(text: &str, data: &str) -> InlineKeyboardButton {
    InlineKeyboardButton::callback(text.to_string(), data.to_string())
}

pub fn main_menu(lang: Lang) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![cb(&i18n::btn_clients(lang), "list")],
        vec![cb(&i18n::btn_add(lang), "add")],
        vec![cb(&i18n::btn_stats(lang), "stats")],
        vec![cb(&i18n::btn_backup(lang), "backup")],
        vec![cb(&i18n::btn_check(lang), "check")],
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

pub fn settings_menu(lang: Lang, psk_default: bool) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![
            cb(&i18n::btn_lang_ru(lang), "set:lang:ru"),
            cb(&i18n::btn_lang_en(lang), "set:lang:en"),
        ],
        vec![cb(
            &i18n::btn_psk_toggle(lang, psk_default),
            if psk_default { "set:psk:off" } else { "set:psk:on" },
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
    let none_txt = match lang { Lang::Ru => "Без срока", Lang::En => "No expiry" };
    let custom_txt = match lang { Lang::Ru => "✏️ Свой", Lang::En => "✏️ Custom" };
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
        vec![cb(&day_label(lang, 365), "exp:365d"), cb(custom_txt, "exp:custom")],
    ])
}

/// Шаг выбора PSK в диалоге `add` — дефолтная опция (по настройке
/// `settings.psk_default()`) идёт первой кнопкой.
pub fn psk_step(lang: Lang, default_on: bool) -> InlineKeyboardMarkup {
    let (first, second) = if default_on {
        (cb(&i18n::btn_create_with_psk(lang), "add:psk:on"), cb(&i18n::btn_create_no_psk(lang), "add:psk:off"))
    } else {
        (cb(&i18n::btn_create_no_psk(lang), "add:psk:off"), cb(&i18n::btn_create_with_psk(lang), "add:psk:on"))
    };
    InlineKeyboardMarkup::new(vec![vec![first, second], vec![cb(&i18n::btn_back(lang), "menu")]])
}

pub fn clients_list(lang: Lang, clients: &[Client], page: usize, per_page: usize) -> InlineKeyboardMarkup {
    if per_page == 0 {
        return InlineKeyboardMarkup::new(vec![vec![cb(&i18n::btn_back(lang), "menu")]]);
    }

    let start = page * per_page;
    let slice = clients.iter().skip(start).take(per_page);
    let mut rows: Vec<Vec<InlineKeyboardButton>> = slice
        .map(|c| {
            let mark = if c.active() { "🟢" } else { "🔴" };
            vec![cb(&format!("{mark} {}", c.name), &format!("client:{}", c.name))]
        })
        .collect();

    let total_pages = clients.len().div_ceil(per_page).max(1);
    let mut nav = Vec::new();
    if page > 0 {
        nav.push(cb("◀️", &format!("page:{}", page - 1)));
    }
    if page + 1 < total_pages {
        nav.push(cb("▶️", &format!("page:{}", page + 1)));
    }
    if !nav.is_empty() {
        rows.push(nav);
    }
    rows.push(vec![cb(&i18n::btn_back(lang), "menu")]);
    InlineKeyboardMarkup::new(rows)
}

pub fn client_card(lang: Lang, name: &str) -> InlineKeyboardMarkup {
    let conf_txt = match lang { Lang::Ru => "📄 Конфиг", Lang::En => "📄 Config" };
    let del_txt = match lang { Lang::Ru => "🗑 Удалить", Lang::En => "🗑 Delete" };
    InlineKeyboardMarkup::new(vec![
        vec![cb(conf_txt, &format!("conf:{name}")), cb(del_txt, &format!("del:{name}"))],
        vec![cb(&i18n::btn_back(lang), "menu")],
    ])
}

pub fn confirm_delete(lang: Lang, name: &str) -> InlineKeyboardMarkup {
    let yes_txt = match lang { Lang::Ru => "✅ Да, удалить", Lang::En => "✅ Yes, delete" };
    InlineKeyboardMarkup::new(vec![vec![
        cb(yes_txt, &format!("delyes:{name}")),
        cb(&i18n::btn_back(lang), "menu"),
    ]])
}

pub fn confirm_recreate(lang: Lang, name: &str) -> InlineKeyboardMarkup {
    let yes_txt = match lang { Lang::Ru => "♻️ Пересоздать", Lang::En => "♻️ Recreate" };
    InlineKeyboardMarkup::new(vec![vec![
        cb(yes_txt, &format!("recreate:{name}")),
        cb(&i18n::btn_back(lang), "menu"),
    ]])
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
        for expected in ["list", "add", "stats", "backup", "check", "settings"] {
            assert!(data.contains(&expected.to_string()), "missing {expected}");
        }
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
        assert!(data.contains(&"del:alice".to_string()));
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
    fn clients_list_one_button_per_client() {
        let clients = vec![
            Client { name: "a".into(), ip: String::new(), client_ipv6: String::new(), status: String::new(), status_code: "active".into(), rx: 0, tx: 0, last_handshake: None },
            Client { name: "b".into(), ip: String::new(), client_ipv6: String::new(), status: String::new(), status_code: "inactive".into(), rx: 0, tx: 0, last_handshake: None },
        ];
        let data = all_callback_data(&clients_list(Lang::Ru, &clients, 0, 10));
        assert!(data.contains(&"client:a".to_string()));
        assert!(data.contains(&"client:b".to_string()));
    }

    #[test]
    fn clients_list_zero_per_page_no_panic() {
        // Test with empty clients
        let empty_clients: Vec<Client> = vec![];
        let kb_empty = clients_list(Lang::Ru, &empty_clients, 0, 0);
        let data_empty = all_callback_data(&kb_empty);
        assert_eq!(data_empty, vec!["menu"], "empty clients with per_page=0 should have only menu callback");

        // Test with non-empty clients
        let clients = vec![
            Client { name: "a".into(), ip: String::new(), client_ipv6: String::new(), status: String::new(), status_code: "active".into(), rx: 0, tx: 0, last_handshake: None },
            Client { name: "b".into(), ip: String::new(), client_ipv6: String::new(), status: String::new(), status_code: "inactive".into(), rx: 0, tx: 0, last_handshake: None },
        ];
        let kb_filled = clients_list(Lang::Ru, &clients, 0, 0);
        let data_filled = all_callback_data(&kb_filled);
        assert_eq!(data_filled, vec!["menu"], "non-empty clients with per_page=0 should have only menu callback");
    }

    #[test]
    fn language_select_has_both_langs() {
        let data = all_callback_data(&language_select());
        assert!(data.contains(&"lang:ru".to_string()));
        assert!(data.contains(&"lang:en".to_string()));
    }

    #[test]
    fn settings_menu_toggles_psk_data_by_current_value() {
        let data_off = all_callback_data(&settings_menu(Lang::Ru, false));
        assert!(data_off.contains(&"set:psk:on".to_string()));
        assert!(!data_off.contains(&"set:psk:off".to_string()));

        let data_on = all_callback_data(&settings_menu(Lang::Ru, true));
        assert!(data_on.contains(&"set:psk:off".to_string()));
        assert!(!data_on.contains(&"set:psk:on".to_string()));

        assert!(data_off.contains(&"set:lang:ru".to_string()));
        assert!(data_off.contains(&"set:lang:en".to_string()));
        assert!(data_off.contains(&"menu".to_string()));
    }

    #[test]
    fn psk_step_has_both_options_and_back() {
        let data = all_callback_data(&psk_step(Lang::Ru, false));
        assert!(data.contains(&"add:psk:on".to_string()));
        assert!(data.contains(&"add:psk:off".to_string()));
        assert!(data.contains(&"menu".to_string()));
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
            BackupFile { name: "a.tar.gz".into(), path: "a.tar.gz".into(), size: 1, mtime: 1 },
            BackupFile { name: "b.tar.gz".into(), path: "b.tar.gz".into(), size: 2, mtime: 2 },
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
            teloxide::types::InlineKeyboardButtonKind::CallbackData(d) => assert_eq!(d, "add:psk:off"),
            _ => panic!("expected callback data"),
        }

        let kb_on = psk_step(Lang::Ru, true);
        let first_row_on = &kb_on.inline_keyboard[0];
        match &first_row_on[0].kind {
            teloxide::types::InlineKeyboardButtonKind::CallbackData(d) => assert_eq!(d, "add:psk:on"),
            _ => panic!("expected callback data"),
        }
    }
}
