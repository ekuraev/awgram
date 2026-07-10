use std::sync::Arc;

use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::dispatching::{HandlerExt, UpdateFilterExt};
use teloxide::prelude::*;
use teloxide::types::CallbackQuery;

use crate::auth::is_admin;
use crate::bot::menu;
use crate::bot::render::{self, format_client_card, format_stats};
use crate::bot::State;
use crate::config::Config;
use crate::vpn::Vpn;

#[derive(Debug, PartialEq)]
pub enum Action {
    Menu,
    List,
    Add,
    Stats,
    Page(usize),
    ShowClient(String),
    SendConf(String),
    AskDelete(String),
    ConfirmDelete(String),
    Expiry(String), // "none" | "1d" | ... | "custom"
    Unknown,
}

fn parse_callback(data: &str) -> Action {
    match data {
        "menu" => Action::Menu,
        "list" => Action::List,
        "add" => Action::Add,
        "stats" => Action::Stats,
        _ => {
            if let Some(v) = data.strip_prefix("page:") {
                v.parse().map(Action::Page).unwrap_or(Action::Unknown)
            } else if let Some(v) = data.strip_prefix("client:") {
                Action::ShowClient(v.to_string())
            } else if let Some(v) = data.strip_prefix("conf:") {
                Action::SendConf(v.to_string())
            } else if let Some(v) = data.strip_prefix("delyes:") {
                // Must be checked before "del:" — otherwise "del:" prefix-matches
                // "delyes:..." and confirmed deletes get misparsed as delete-asks.
                Action::ConfirmDelete(v.to_string())
            } else if let Some(v) = data.strip_prefix("del:") {
                Action::AskDelete(v.to_string())
            } else if let Some(v) = data.strip_prefix("exp:") {
                Action::Expiry(v.to_string())
            } else {
                Action::Unknown
            }
        }
    }
}

type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
type MyDialogue = Dialogue<State, InMemStorage<State>>;

fn user_id_of_msg(msg: &Message) -> Option<i64> {
    msg.from.as_ref().map(|u| u.id.0 as i64)
}

fn user_id_of_cb(q: &CallbackQuery) -> i64 {
    q.from.id.0 as i64
}

async fn message_handler(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    cfg: Arc<Config>,
    vpn: Arc<Vpn>,
) -> HandlerResult {
    let uid = user_id_of_msg(&msg).unwrap_or(0);
    if !is_admin(uid, &cfg.admin_ids) {
        tracing::warn!(user_id = uid, "отклонён доступ (message)");
        bot.send_message(msg.chat.id, "⛔ Доступ запрещён.").await?;
        return Ok(());
    }

    let state = dialogue.get().await?.unwrap_or_default();
    match state {
        State::AwaitingName => {
            let name = msg.text().unwrap_or_default().to_string();
            match crate::vpn::validate::validate_name(&name) {
                Ok(valid) => {
                    bot.send_message(msg.chat.id, format!("Клиент: {valid}\nВыберите срок действия:"))
                        .reply_markup(menu::expiry_menu())
                        .await?;
                    dialogue.update(State::AwaitingExpiry { name: valid }).await?;
                }
                Err(e) => {
                    bot.send_message(msg.chat.id, format!("⚠️ {e}\nВведите имя ещё раз:")).await?;
                }
            }
        }
        State::AwaitingCustomExpiry { name } => {
            let raw = msg.text().unwrap_or_default().to_string();
            match crate::vpn::validate::validate_expiry(&raw) {
                Ok(exp) => {
                    finish_add(&bot, msg.chat.id, &vpn, &name, Some(&exp)).await;
                    dialogue.exit().await?;
                }
                Err(e) => {
                    bot.send_message(msg.chat.id, format!("⚠️ {e}")).await?;
                }
            }
        }
        _ => {
            // /start и всё прочее — показать меню
            bot.send_message(msg.chat.id, "🔐 AmneziaWG — управление VPN")
                .reply_markup(menu::main_menu())
                .await?;
            dialogue.update(State::Idle).await?;
        }
    }
    Ok(())
}

async fn finish_add(bot: &Bot, chat: ChatId, vpn: &Vpn, name: &str, expires: Option<&str>) {
    let waiting = bot.send_message(chat, "⏳ Создаю клиента…").await.ok();
    match vpn.add(name, expires).await {
        Ok(res) => {
            if let Err(e) = render::send_client_files(bot, chat, &res).await {
                tracing::error!(error = %e, "не удалось отправить файлы клиента");
                let _ = bot.send_message(chat, e.user_message()).await;
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "add провалился");
            let _ = bot.send_message(chat, e.user_message()).await;
        }
    }
    if let Some(m) = waiting {
        let _ = bot.delete_message(chat, m.id).await;
    }
    let _ = bot.send_message(chat, "Готово.").reply_markup(menu::main_menu()).await;
}

async fn callback_handler(
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
    cfg: Arc<Config>,
    vpn: Arc<Vpn>,
) -> HandlerResult {
    bot.answer_callback_query(q.id.clone()).await.ok();

    let uid = user_id_of_cb(&q);
    if !is_admin(uid, &cfg.admin_ids) {
        tracing::warn!(user_id = uid, "отклонён доступ (callback)");
        return Ok(());
    }

    let chat = match &q.message {
        Some(m) => m.chat().id,
        None => return Ok(()),
    };

    let data = q.data.clone().unwrap_or_default();
    match parse_callback(&data) {
        Action::Menu => {
            dialogue.update(State::Idle).await?;
            bot.send_message(chat, "🔐 AmneziaWG").reply_markup(menu::main_menu()).await?;
        }
        Action::List => match vpn.list().await {
            Ok(clients) if clients.is_empty() => {
                bot.send_message(chat, "Пока нет клиентов.").reply_markup(menu::main_menu()).await?;
            }
            Ok(clients) => {
                bot.send_message(chat, "👥 Клиенты:")
                    .reply_markup(menu::clients_list(&clients, 0, 8))
                    .await?;
            }
            Err(e) => {
                tracing::error!(error = %e, "list провалился");
                bot.send_message(chat, e.user_message()).await?;
            }
        },
        Action::Page(p) => match vpn.list().await {
            Ok(clients) => {
                bot.send_message(chat, "👥 Клиенты:")
                    .reply_markup(menu::clients_list(&clients, p, 8))
                    .await?;
            }
            Err(e) => {
                bot.send_message(chat, e.user_message()).await?;
            }
        },
        Action::Stats => match vpn.stats().await {
            Ok(clients) => {
                bot.send_message(chat, format_stats(&clients))
                    .reply_markup(menu::main_menu())
                    .await?;
            }
            Err(e) => {
                bot.send_message(chat, e.user_message()).await?;
            }
        },
        Action::ShowClient(name) => match vpn.stats().await {
            Ok(clients) => match clients.iter().find(|c| c.name == name) {
                Some(c) => {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                    let expiry = vpn.client_expiry(&name);
                    bot.send_message(chat, format_client_card(c, now, expiry))
                        .reply_markup(menu::client_card(&name))
                        .await?;
                }
                None => {
                    bot.send_message(chat, "Клиент не найден.").await?;
                }
            },
            Err(e) => {
                bot.send_message(chat, e.user_message()).await?;
            }
        },
        Action::SendConf(name) => {
            // Повторная выдача: читаем уже существующие .conf/.png/.vpnuri из clients_dir.
            match vpn.existing_files(&name) {
                Ok(res) => {
                    if let Err(e) = render::send_client_files(&bot, chat, &res).await {
                        bot.send_message(chat, e.user_message()).await?;
                    }
                }
                Err(e) => {
                    bot.send_message(chat, e.user_message()).await?;
                }
            }
        }
        Action::AskDelete(name) => {
            bot.send_message(chat, format!("Точно удалить {name}?"))
                .reply_markup(menu::confirm_delete(&name))
                .await?;
        }
        Action::ConfirmDelete(name) => match vpn.remove(&name).await {
            Ok(()) => {
                bot.send_message(chat, format!("🗑 Клиент {name} удалён."))
                    .reply_markup(menu::main_menu())
                    .await?;
            }
            Err(e) => {
                tracing::error!(error = %e, "remove провалился");
                bot.send_message(chat, e.user_message()).await?;
            }
        },
        Action::Add => {
            bot.send_message(chat, "Введите имя клиента:").await?;
            dialogue.update(State::AwaitingName).await?;
        }
        Action::Expiry(kind) => {
            let name = match dialogue.get().await?.unwrap_or_default() {
                State::AwaitingExpiry { name } => name,
                _ => {
                    bot.send_message(chat, "Сессия устарела. Начните заново.")
                        .reply_markup(menu::main_menu())
                        .await?;
                    return Ok(());
                }
            };
            if kind == "custom" {
                bot.send_message(chat, "Введите срок (например 10d, 12h, 3w):").await?;
                dialogue.update(State::AwaitingCustomExpiry { name }).await?;
            } else {
                let expires = if kind == "none" { None } else { Some(kind.as_str()) };
                finish_add(&bot, chat, &vpn, &name, expires).await;
                dialogue.exit().await?;
            }
        }
        Action::Unknown => {
            bot.send_message(chat, "Неизвестное действие.").await?;
        }
    }
    Ok(())
}

/// dptree-схема для `Dispatcher`. Зависимости (`Arc<Vpn>`, `Arc<Config>`,
/// `InMemStorage<State>`) регистрируются в `main` через `dptree::deps![...]`.
pub fn schema() -> teloxide::dispatching::UpdateHandler<Box<dyn std::error::Error + Send + Sync>> {
    dptree::entry()
        .enter_dialogue::<Update, InMemStorage<State>, State>()
        .branch(Update::filter_message().endpoint(message_handler))
        .branch(Update::filter_callback_query().endpoint(callback_handler))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_actions() {
        assert_eq!(parse_callback("menu"), Action::Menu);
        assert_eq!(parse_callback("list"), Action::List);
        assert_eq!(parse_callback("add"), Action::Add);
        assert_eq!(parse_callback("stats"), Action::Stats);
        assert_eq!(parse_callback("page:3"), Action::Page(3));
        assert_eq!(parse_callback("client:alice"), Action::ShowClient("alice".into()));
        assert_eq!(parse_callback("conf:alice"), Action::SendConf("alice".into()));
        assert_eq!(parse_callback("del:alice"), Action::AskDelete("alice".into()));
        assert_eq!(parse_callback("delyes:alice"), Action::ConfirmDelete("alice".into()));
        assert_eq!(parse_callback("exp:30d"), Action::Expiry("30d".into()));
        assert_eq!(parse_callback("exp:custom"), Action::Expiry("custom".into()));
        assert_eq!(parse_callback("garbage"), Action::Unknown);
    }

    /// Замораживает контракт между слоем клавиатур (`menu`) и парсером
    /// callback-данных (`parse_callback`): каждая строка, которую эмитят
    /// клавиатуры, должна разбираться в осмысленный `Action`, а не в
    /// `Action::Unknown`. Это защищает от расхождения префиксов при
    /// будущих изменениях.
    #[test]
    fn all_menu_callback_data_parse_to_known_actions() {
        use crate::vpn::model::Client;
        use teloxide::types::{InlineKeyboardButtonKind, InlineKeyboardMarkup};

        fn all_callback_data(kb: &InlineKeyboardMarkup) -> Vec<String> {
            kb.inline_keyboard
                .iter()
                .flatten()
                .filter_map(|b| match &b.kind {
                    InlineKeyboardButtonKind::CallbackData(d) => Some(d.clone()),
                    _ => None,
                })
                .collect()
        }

        let sample_client = Client {
            name: "alice".into(),
            ip: String::new(),
            client_ipv6: String::new(),
            status: String::new(),
            status_code: "active".into(),
            rx: 0,
            tx: 0,
            last_handshake: None,
        };

        let keyboards = vec![
            menu::main_menu(),
            menu::expiry_menu(),
            menu::client_card("alice"),
            menu::confirm_delete("bob"),
            menu::clients_list(&[sample_client], 0, 8),
        ];

        for kb in &keyboards {
            for data in all_callback_data(kb) {
                assert_ne!(
                    parse_callback(&data),
                    Action::Unknown,
                    "callback data {data:?} did not parse to a known Action"
                );
            }
        }
    }
}
