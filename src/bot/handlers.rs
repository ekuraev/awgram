use std::sync::Arc;

use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::dispatching::{HandlerExt, UpdateFilterExt};
use teloxide::prelude::*;
use teloxide::types::{CallbackQuery, ParseMode};

use crate::auth::is_admin;
use crate::bot::menu;
use crate::bot::render::{self, format_client_card, format_stats};
use crate::bot::State;
use crate::config::Config;
use crate::i18n::{self, Lang};
use crate::settings::SettingsStore;
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
    Lang(String),   // "ru" | "en" — язык-гейт при первом /start
    Settings,
    SetLang(String), // "ru" | "en" — смена языка из экрана настроек
    SetPsk(bool),
    Unknown,
}

fn parse_callback(data: &str) -> Action {
    match data {
        "menu" => Action::Menu,
        "list" => Action::List,
        "add" => Action::Add,
        "stats" => Action::Stats,
        "settings" => Action::Settings,
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
            } else if let Some(v) = data.strip_prefix("set:lang:") {
                // Must be checked before the general "lang:" prefix — same reason
                // as delyes:/del: above ("set:lang:ru" also starts with "set:").
                Action::SetLang(v.to_string())
            } else if let Some(v) = data.strip_prefix("set:psk:") {
                Action::SetPsk(v == "on")
            } else if let Some(v) = data.strip_prefix("lang:") {
                Action::Lang(v.to_string())
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

/// Локальный текст сессии-таймаута: не входит в каталог `i18n` (см. brief
/// задачи 5 — новые фичи в других задачах), но всё равно локализуется, чтобы
/// не оставлять непереведённых строк в слое `bot/`.
fn session_expired_text(lang: Lang) -> &'static str {
    match lang {
        Lang::Ru => "Сессия устарела. Начните заново.",
        Lang::En => "Session expired. Start again.",
    }
}

fn unknown_action_text(lang: Lang) -> &'static str {
    match lang {
        Lang::Ru => "Неизвестное действие.",
        Lang::En => "Unknown action.",
    }
}

async fn message_handler(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    cfg: Arc<Config>,
    vpn: Arc<Vpn>,
    settings: Arc<SettingsStore>,
) -> HandlerResult {
    let uid = user_id_of_msg(&msg).unwrap_or(0);
    if !is_admin(uid, &cfg.admin_ids) {
        tracing::warn!(user_id = uid, "отклонён доступ (message)");
        let lang = settings.lang(uid);
        bot.send_message(msg.chat.id, i18n::access_denied(lang)).await?;
        return Ok(());
    }
    let lang = settings.lang(uid);

    let state = dialogue.get().await?.unwrap_or_default();
    match state {
        State::AwaitingName => {
            let name = msg.text().unwrap_or_default().to_string();
            match crate::vpn::validate::validate_name(&name) {
                Ok(valid) => {
                    let confirm_line = match lang {
                        Lang::Ru => format!("Клиент: {valid}"),
                        Lang::En => format!("Client: {valid}"),
                    };
                    bot.send_message(msg.chat.id, format!("{confirm_line}\n{}", i18n::ask_expiry(lang)))
                        .reply_markup(menu::expiry_menu(lang))
                        .await?;
                    dialogue.update(State::AwaitingExpiry { name: valid }).await?;
                }
                Err(_e) => {
                    bot.send_message(msg.chat.id, i18n::bad_name(lang)).await?;
                }
            }
        }
        State::AwaitingCustomExpiry { name } => {
            let raw = msg.text().unwrap_or_default().to_string();
            match crate::vpn::validate::validate_expiry(&raw) {
                Ok(exp) => {
                    finish_add(&bot, msg.chat.id, &vpn, lang, &name, Some(&exp)).await;
                    dialogue.exit().await?;
                }
                Err(_e) => {
                    bot.send_message(msg.chat.id, i18n::bad_expiry(lang)).await?;
                }
            }
        }
        _ => {
            // /start и всё прочее.
            if !settings.has_lang(uid) {
                // Язык-гейт: пользователь ещё не выбрал язык — показать выбор
                // без parse_mode (choose_language() не содержит HTML-разметки).
                bot.send_message(msg.chat.id, i18n::choose_language())
                    .reply_markup(menu::language_select())
                    .await?;
            } else {
                bot.send_message(msg.chat.id, i18n::menu_title(lang))
                    .reply_markup(menu::main_menu(lang))
                    .parse_mode(ParseMode::Html)
                    .await?;
            }
            dialogue.update(State::Idle).await?;
        }
    }
    Ok(())
}

async fn finish_add(bot: &Bot, chat: ChatId, vpn: &Vpn, lang: Lang, name: &str, expires: Option<&str>) {
    let waiting = bot.send_message(chat, i18n::creating(lang)).await.ok();
    match vpn.add(name, expires).await {
        Ok(res) => {
            if let Err(e) = render::send_client_files(bot, chat, lang, &res).await {
                tracing::error!(error = %e, "не удалось отправить файлы клиента");
                let _ = bot.send_message(chat, i18n::error_text(lang, &e)).await;
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "add провалился");
            let _ = bot.send_message(chat, i18n::error_text(lang, &e)).await;
        }
    }
    if let Some(m) = waiting {
        let _ = bot.delete_message(chat, m.id).await;
    }
    let _ = bot
        .send_message(chat, i18n::done(lang))
        .reply_markup(menu::main_menu(lang))
        .parse_mode(ParseMode::Html)
        .await;
}

async fn callback_handler(
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
    cfg: Arc<Config>,
    vpn: Arc<Vpn>,
    settings: Arc<SettingsStore>,
) -> HandlerResult {
    bot.answer_callback_query(q.id.clone()).await.ok();

    let uid = user_id_of_cb(&q);
    if !is_admin(uid, &cfg.admin_ids) {
        tracing::warn!(user_id = uid, "отклонён доступ (callback)");
        return Ok(());
    }
    let lang = settings.lang(uid);

    let chat = match &q.message {
        Some(m) => m.chat().id,
        None => return Ok(()),
    };

    let data = q.data.clone().unwrap_or_default();
    match parse_callback(&data) {
        Action::Menu => {
            dialogue.update(State::Idle).await?;
            bot.send_message(chat, i18n::menu_title(lang))
                .reply_markup(menu::main_menu(lang))
                .parse_mode(ParseMode::Html)
                .await?;
        }
        Action::List => match vpn.list().await {
            Ok(clients) if clients.is_empty() => {
                bot.send_message(chat, i18n::clients_empty(lang))
                    .reply_markup(menu::main_menu(lang))
                    .await?;
            }
            Ok(clients) => {
                bot.send_message(chat, i18n::clients_title(lang))
                    .reply_markup(menu::clients_list(lang, &clients, 0, 8))
                    .parse_mode(ParseMode::Html)
                    .await?;
            }
            Err(e) => {
                tracing::error!(error = %e, "list провалился");
                bot.send_message(chat, i18n::error_text(lang, &e)).await?;
            }
        },
        Action::Page(p) => match vpn.list().await {
            Ok(clients) => {
                bot.send_message(chat, i18n::clients_title(lang))
                    .reply_markup(menu::clients_list(lang, &clients, p, 8))
                    .parse_mode(ParseMode::Html)
                    .await?;
            }
            Err(e) => {
                bot.send_message(chat, i18n::error_text(lang, &e)).await?;
            }
        },
        Action::Stats => match vpn.stats().await {
            Ok(clients) => {
                bot.send_message(chat, format_stats(lang, &clients))
                    .reply_markup(menu::main_menu(lang))
                    .parse_mode(ParseMode::Html)
                    .await?;
            }
            Err(e) => {
                bot.send_message(chat, i18n::error_text(lang, &e)).await?;
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
                    bot.send_message(chat, format_client_card(lang, c, now, expiry))
                        .reply_markup(menu::client_card(lang, &name))
                        .parse_mode(ParseMode::Html)
                        .await?;
                }
                None => {
                    bot.send_message(chat, i18n::not_found(lang)).await?;
                }
            },
            Err(e) => {
                bot.send_message(chat, i18n::error_text(lang, &e)).await?;
            }
        },
        Action::SendConf(name) => {
            // Повторная выдача: читаем уже существующие .conf/.png/.vpnuri из clients_dir.
            match vpn.existing_files(&name) {
                Ok(res) => {
                    if let Err(e) = render::send_client_files(&bot, chat, lang, &res).await {
                        bot.send_message(chat, i18n::error_text(lang, &e)).await?;
                    }
                }
                Err(e) => {
                    bot.send_message(chat, i18n::error_text(lang, &e)).await?;
                }
            }
        }
        Action::AskDelete(name) => {
            bot.send_message(chat, i18n::confirm_delete(lang, &name))
                .reply_markup(menu::confirm_delete(lang, &name))
                .parse_mode(ParseMode::Html)
                .await?;
        }
        Action::ConfirmDelete(name) => match vpn.remove(&name).await {
            Ok(()) => {
                bot.send_message(chat, i18n::deleted(lang, &name))
                    .reply_markup(menu::main_menu(lang))
                    .parse_mode(ParseMode::Html)
                    .await?;
            }
            Err(e) => {
                tracing::error!(error = %e, "remove провалился");
                bot.send_message(chat, i18n::error_text(lang, &e)).await?;
            }
        },
        Action::Add => {
            bot.send_message(chat, i18n::ask_client_name(lang)).await?;
            dialogue.update(State::AwaitingName).await?;
        }
        Action::Expiry(kind) => {
            let name = match dialogue.get().await?.unwrap_or_default() {
                State::AwaitingExpiry { name } => name,
                _ => {
                    bot.send_message(chat, session_expired_text(lang))
                        .reply_markup(menu::main_menu(lang))
                        .parse_mode(ParseMode::Html)
                        .await?;
                    return Ok(());
                }
            };
            if kind == "custom" {
                bot.send_message(chat, i18n::ask_custom_expiry(lang)).await?;
                dialogue.update(State::AwaitingCustomExpiry { name }).await?;
            } else {
                let expires = if kind == "none" { None } else { Some(kind.as_str()) };
                finish_add(&bot, chat, &vpn, lang, &name, expires).await;
                dialogue.exit().await?;
            }
        }
        Action::Settings => {
            bot.send_message(chat, i18n::settings_title(lang, settings.psk_default()))
                .reply_markup(menu::settings_menu(lang, settings.psk_default()))
                .parse_mode(ParseMode::Html)
                .await?;
        }
        Action::Lang(code) => {
            if let Some(l) = i18n::parse_lang(&code) {
                settings.set_lang(uid, l);
            }
            let lang = settings.lang(uid);
            bot.send_message(chat, i18n::menu_title(lang))
                .reply_markup(menu::main_menu(lang))
                .parse_mode(ParseMode::Html)
                .await?;
        }
        Action::SetLang(code) => {
            if let Some(l) = i18n::parse_lang(&code) {
                settings.set_lang(uid, l);
            }
            let lang = settings.lang(uid);
            bot.send_message(chat, i18n::settings_title(lang, settings.psk_default()))
                .reply_markup(menu::settings_menu(lang, settings.psk_default()))
                .parse_mode(ParseMode::Html)
                .await?;
        }
        Action::SetPsk(on) => {
            settings.set_psk_default(on);
            bot.send_message(chat, i18n::settings_title(lang, settings.psk_default()))
                .reply_markup(menu::settings_menu(lang, settings.psk_default()))
                .parse_mode(ParseMode::Html)
                .await?;
        }
        Action::Unknown => {
            bot.send_message(chat, unknown_action_text(lang)).await?;
        }
    }
    Ok(())
}

/// dptree-схема для `Dispatcher`. Зависимости (`Arc<Vpn>`, `Arc<Config>`,
/// `Arc<SettingsStore>`, `InMemStorage<State>`) регистрируются в `main` через
/// `dptree::deps![...]`.
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
        assert_eq!(parse_callback("settings"), Action::Settings);
        assert_eq!(parse_callback("lang:ru"), Action::Lang("ru".into()));
        assert_eq!(parse_callback("lang:en"), Action::Lang("en".into()));
        assert_eq!(parse_callback("set:lang:ru"), Action::SetLang("ru".into()));
        assert_eq!(parse_callback("set:lang:en"), Action::SetLang("en".into()));
        assert_eq!(parse_callback("set:psk:on"), Action::SetPsk(true));
        assert_eq!(parse_callback("set:psk:off"), Action::SetPsk(false));
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
            menu::main_menu(Lang::Ru),
            menu::expiry_menu(Lang::Ru),
            menu::client_card(Lang::Ru, "alice"),
            menu::confirm_delete(Lang::Ru, "bob"),
            menu::clients_list(Lang::Ru, &[sample_client], 0, 8),
            menu::language_select(),
            menu::settings_menu(Lang::Ru, false),
            menu::settings_menu(Lang::Ru, true),
        ];

        for kb in &keyboards {
            for data in all_callback_data(kb) {
                // `backup`/`check` are already emitted by `main_menu` (buttons
                // shipped ahead of their handlers) but their Action variants
                // (`Action::Backup`/`Action::Check`) land in Tasks 7/8. Until
                // then they intentionally parse to `Action::Unknown` — tapping
                // them shows "unknown action" rather than crashing. Restore
                // them to the assertion once Tasks 7/8 land.
                if data == "backup" || data == "check" {
                    assert_eq!(
                        parse_callback(&data),
                        Action::Unknown,
                        "callback data {data:?} expected to be Unknown until Tasks 7/8"
                    );
                    continue;
                }
                assert_ne!(
                    parse_callback(&data),
                    Action::Unknown,
                    "callback data {data:?} did not parse to a known Action"
                );
            }
        }
    }
}
