use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

use crate::vpn::model::Client;

fn cb(text: &str, data: &str) -> InlineKeyboardButton {
    InlineKeyboardButton::callback(text.to_string(), data.to_string())
}

pub fn main_menu() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![cb("👥 Клиенты", "list")],
        vec![cb("➕ Добавить", "add")],
        vec![cb("📊 Статистика", "stats")],
    ])
}

pub fn expiry_menu() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![cb("Без срока", "exp:none")],
        vec![cb("1д", "exp:1d"), cb("7д", "exp:7d"), cb("14д", "exp:14d")],
        vec![cb("30д", "exp:30d"), cb("90д", "exp:90d"), cb("180д", "exp:180d")],
        vec![cb("365д", "exp:365d"), cb("✏️ Свой", "exp:custom")],
    ])
}

pub fn clients_list(clients: &[Client], page: usize, per_page: usize) -> InlineKeyboardMarkup {
    if per_page == 0 {
        return InlineKeyboardMarkup::new(vec![vec![cb("⬅️ В меню", "menu")]]);
    }

    let start = page * per_page;
    let slice = clients.iter().skip(start).take(per_page);
    let mut rows: Vec<Vec<InlineKeyboardButton>> = slice
        .map(|c| {
            let mark = if c.active { "🟢" } else { "🔴" };
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
    rows.push(vec![cb("⬅️ В меню", "menu")]);
    InlineKeyboardMarkup::new(rows)
}

pub fn client_card(name: &str) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![cb("📄 Конфиг", &format!("conf:{name}")), cb("🗑 Удалить", &format!("del:{name}"))],
        vec![cb("⬅️ В меню", "menu")],
    ])
}

pub fn confirm_delete(name: &str) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![
        cb("✅ Да, удалить", &format!("delyes:{name}")),
        cb("⬅️ Отмена", "menu"),
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
        let data = all_callback_data(&main_menu());
        for expected in ["list", "add", "stats"] {
            assert!(data.contains(&expected.to_string()), "missing {expected}");
        }
    }

    #[test]
    fn expiry_menu_has_custom_and_presets() {
        let data = all_callback_data(&expiry_menu());
        assert!(data.contains(&"exp:none".to_string()));
        assert!(data.contains(&"exp:30d".to_string()));
        assert!(data.contains(&"exp:custom".to_string()));
    }

    #[test]
    fn client_card_encodes_name() {
        let data = all_callback_data(&client_card("alice"));
        assert!(data.contains(&"conf:alice".to_string()));
        assert!(data.contains(&"del:alice".to_string()));
    }

    #[test]
    fn confirm_delete_encodes_name() {
        let data = all_callback_data(&confirm_delete("bob"));
        assert!(data.contains(&"delyes:bob".to_string()));
    }

    #[test]
    fn clients_list_one_button_per_client() {
        let clients = vec![
            Client { name: "a".into(), active: true, expires_at: None, rx_bytes: 0, tx_bytes: 0, last_handshake: None },
            Client { name: "b".into(), active: false, expires_at: None, rx_bytes: 0, tx_bytes: 0, last_handshake: None },
        ];
        let data = all_callback_data(&clients_list(&clients, 0, 10));
        assert!(data.contains(&"client:a".to_string()));
        assert!(data.contains(&"client:b".to_string()));
    }

    #[test]
    fn clients_list_zero_per_page_no_panic() {
        // Test with empty clients
        let empty_clients: Vec<Client> = vec![];
        let kb_empty = clients_list(&empty_clients, 0, 0);
        let data_empty = all_callback_data(&kb_empty);
        assert_eq!(data_empty, vec!["menu"], "empty clients with per_page=0 should have only menu callback");

        // Test with non-empty clients
        let clients = vec![
            Client { name: "a".into(), active: true, expires_at: None, rx_bytes: 0, tx_bytes: 0, last_handshake: None },
            Client { name: "b".into(), active: false, expires_at: None, rx_bytes: 0, tx_bytes: 0, last_handshake: None },
        ];
        let kb_filled = clients_list(&clients, 0, 0);
        let data_filled = all_callback_data(&kb_filled);
        assert_eq!(data_filled, vec!["menu"], "non-empty clients with per_page=0 should have only menu callback");
    }
}
