use std::sync::Arc;

use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::dispatching::{HandlerExt, UpdateFilterExt};
use teloxide::prelude::*;
use teloxide::types::{CallbackQuery, InputFile, ParseMode};

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
    Recreate(String),
    Regen(String),
    Expiry(String), // "none" | "1d" | ... | "custom"
    Lang(String),   // "ru" | "en" — язык-гейт при первом /start
    Settings,
    SetLang(String), // "ru" | "en" — смена языка из экрана настроек
    SetPsk(bool),
    AddPsk(bool),
    Backup,
    BackupNew,
    BackupList,
    BackupCard(usize),
    BackupDownload(usize),
    Restore(usize),
    RestoreYes(usize),
    Check,
    Diagnose,
    Unknown,
}

fn parse_callback(data: &str) -> Action {
    match data {
        "menu" => Action::Menu,
        "list" => Action::List,
        "add" => Action::Add,
        "stats" => Action::Stats,
        "settings" => Action::Settings,
        "backup" => Action::Backup,
        "bk:new" => Action::BackupNew,
        "bk:list" => Action::BackupList,
        "check" => Action::Check,
        "diagnose" => Action::Diagnose,
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
            } else if let Some(v) = data.strip_prefix("recreate:") {
                Action::Recreate(v.to_string())
            } else if let Some(v) = data.strip_prefix("regen:") {
                Action::Regen(v.to_string())
            } else if let Some(v) = data.strip_prefix("exp:") {
                Action::Expiry(v.to_string())
            } else if let Some(v) = data.strip_prefix("add:psk:") {
                // No collision with the exact-match "add" arm above (that's a
                // full-string match, not a prefix), but kept ahead of any
                // future generic "add:" prefix for the same reason as
                // delyes:/del: and set:lang:/lang: below.
                Action::AddPsk(v == "on")
            } else if let Some(v) = data.strip_prefix("set:lang:") {
                // Must be checked before the general "lang:" prefix — same reason
                // as delyes:/del: above ("set:lang:ru" also starts with "set:").
                Action::SetLang(v.to_string())
            } else if let Some(v) = data.strip_prefix("set:psk:") {
                Action::SetPsk(v == "on")
            } else if let Some(v) = data.strip_prefix("lang:") {
                Action::Lang(v.to_string())
            } else if let Some(v) = data.strip_prefix("bk:restore_yes:") {
                // Must be checked before "bk:restore:" — otherwise "bk:restore:"
                // prefix-matches "bk:restore_yes:..." and confirmed restores get
                // misparsed as restore-asks (same pattern as delyes:/del:).
                v.parse().map(Action::RestoreYes).unwrap_or(Action::Unknown)
            } else if let Some(v) = data.strip_prefix("bk:restore:") {
                v.parse().map(Action::Restore).unwrap_or(Action::Unknown)
            } else if let Some(v) = data.strip_prefix("bk:card:") {
                v.parse().map(Action::BackupCard).unwrap_or(Action::Unknown)
            } else if let Some(v) = data.strip_prefix("bk:dl:") {
                v.parse().map(Action::BackupDownload).unwrap_or(Action::Unknown)
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

fn now_epoch() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Обрезает вывод скрипта до лимита Telegram-сообщения (3500 байт, с запасом
/// на HTML-обёртку), округляя вниз до границы UTF-8-символа — байтовый индекс
/// может попасть внутрь многобайтового символа (кириллица в выводе скрипта).
fn truncate_for_message(body: String) -> String {
    if body.len() <= 3500 {
        return body;
    }
    let mut cut = 3500;
    while !body.is_char_boundary(cut) {
        cut -= 1;
    }
    format!("{}\n…", &body[..cut])
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
    if !msg.chat.is_private() {
        // Бот доставляет секреты (конфиги, QR, ссылки, бэкапы, диагностику) в чат
        // апдейта, а авторизует по user_id — в группе это грозит утечкой всем
        // участникам. Отклоняем до auth-гейта, чтобы вообще не трогать VPN/settings.
        bot.send_message(msg.chat.id, i18n::private_only()).await?;
        return Ok(());
    }

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
                    match vpn.exists(&valid).await {
                        Ok(false) => {
                            let confirm_line = match lang {
                                Lang::Ru => format!("Клиент: {valid}"),
                                Lang::En => format!("Client: {valid}"),
                            };
                            bot.send_message(msg.chat.id, format!("{confirm_line}\n{}", i18n::ask_expiry(lang)))
                                .reply_markup(menu::expiry_menu(lang))
                                .await?;
                            dialogue.update(State::AwaitingExpiry { name: valid, recreate: false }).await?;
                        }
                        Ok(true) => {
                            bot.send_message(msg.chat.id, i18n::client_exists(lang, &valid))
                                .reply_markup(menu::confirm_recreate(lang, &valid))
                                .parse_mode(ParseMode::Html)
                                .await?;
                            dialogue.update(State::Idle).await?;
                        }
                        Err(e) => {
                            // list --json упал — не блокируем создание (fail-open).
                            tracing::warn!(error = %e, "exists check failed, proceeding without duplicate guard");
                            bot.send_message(msg.chat.id, i18n::ask_expiry(lang))
                                .reply_markup(menu::expiry_menu(lang))
                                .await?;
                            dialogue.update(State::AwaitingExpiry { name: valid, recreate: false }).await?;
                        }
                    }
                }
                Err(_e) => {
                    bot.send_message(msg.chat.id, i18n::bad_name(lang)).await?;
                }
            }
        }
        State::AwaitingCustomExpiry { name, recreate } => {
            let raw = msg.text().unwrap_or_default().to_string();
            match crate::vpn::validate::validate_expiry(&raw) {
                Ok(exp) => {
                    bot.send_message(msg.chat.id, i18n::psk_step(lang, settings.psk_default()))
                        .reply_markup(menu::psk_step(lang, settings.psk_default()))
                        .parse_mode(ParseMode::Html)
                        .await?;
                    dialogue.update(State::AwaitingPsk { name, expires: Some(exp), recreate }).await?;
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

#[allow(clippy::too_many_arguments)]
async fn finish_add(bot: &Bot, chat: ChatId, vpn: &Vpn, lang: Lang, name: &str, expires: Option<&str>, psk: bool, recreate: bool) {
    let waiting = bot.send_message(chat, i18n::creating(lang)).await.ok();
    if recreate {
        // Удаляем старого клиента перед созданием нового. Если remove упадёт —
        // не создаём нового, показываем ошибку; старый клиент остаётся.
        if let Err(e) = vpn.remove(name).await {
            tracing::error!(error = %e, "remove перед recreate провалился");
            if let Some(m) = waiting {
                let _ = bot.delete_message(chat, m.id).await;
            }
            let _ = bot.send_message(chat, i18n::error_text(lang, &e)).await;
            return;
        }
    }
    match vpn.add(name, expires, psk).await {
        Ok(res) => {
            if let Err(e) = render::send_client_files(bot, chat, lang, &res).await {
                tracing::error!(error = %e, "не удалось отправить файлы клиента");
                let _ = bot.send_message(chat, i18n::error_text(lang, &e)).await;
            }
        }
        // Гонка: клиент появился между проверкой exists() и add — скрипт молча
        // пропустил создание (rc 0). Показываем то же предупреждение с кнопкой
        // пересоздания, что и при обычном совпадении имени.
        Err(crate::error::Error::ClientExists(_)) => {
            if let Some(m) = waiting {
                let _ = bot.delete_message(chat, m.id).await;
            }
            let _ = bot
                .send_message(chat, i18n::client_exists(lang, name))
                .reply_markup(menu::confirm_recreate(lang, name))
                .parse_mode(ParseMode::Html)
                .await;
            return;
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

    let chat = match &q.message {
        Some(m) => m.chat(),
        None => return Ok(()),
    };
    if !chat.is_private() {
        // Секреты (конфиги, QR, ссылки, бэкапы, диагностика) уходят в чат
        // апдейта — в группе они утекли бы всем участникам. Callback уже
        // отвечен выше, тут просто молча отказываем без запуска VPN-действий.
        return Ok(());
    }
    let chat = chat.id;

    let uid = user_id_of_cb(&q);
    if !is_admin(uid, &cfg.admin_ids) {
        tracing::warn!(user_id = uid, "отклонён доступ (callback)");
        return Ok(());
    }
    let lang = settings.lang(uid);

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
                let expiries: Vec<Option<i64>> =
                    clients.iter().map(|c| vpn.client_expiry(&c.name)).collect();
                bot.send_message(chat, i18n::clients_title(lang))
                    .reply_markup(menu::clients_list(lang, &clients, &expiries, now_epoch(), 0, 8))
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
                let expiries: Vec<Option<i64>> =
                    clients.iter().map(|c| vpn.client_expiry(&c.name)).collect();
                bot.send_message(chat, i18n::clients_title(lang))
                    .reply_markup(menu::clients_list(lang, &clients, &expiries, now_epoch(), p, 8))
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
                    let now = now_epoch();
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
        Action::Recreate(name) => {
            bot.send_message(chat, i18n::ask_expiry(lang))
                .reply_markup(menu::expiry_menu(lang))
                .await?;
            dialogue.update(State::AwaitingExpiry { name, recreate: true }).await?;
        }
        Action::Regen(name) => {
            let waiting = bot.send_message(chat, i18n::regen_running(lang)).await.ok();
            match vpn.regen_client(&name).await {
                Ok(res) => {
                    if let Err(e) = render::send_client_files(&bot, chat, lang, &res).await {
                        tracing::error!(error = %e, "не удалось отправить файлы после regen");
                        bot.send_message(chat, i18n::error_text(lang, &e)).await?;
                    } else {
                        bot.send_message(chat, i18n::done(lang))
                            .reply_markup(menu::main_menu(lang))
                            .parse_mode(ParseMode::Html)
                            .await?;
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, "regen провалился");
                    bot.send_message(chat, i18n::error_text(lang, &e)).await?;
                }
            }
            if let Some(m) = waiting {
                let _ = bot.delete_message(chat, m.id).await;
            }
        }
        Action::Add => {
            bot.send_message(chat, i18n::ask_client_name(lang)).await?;
            dialogue.update(State::AwaitingName).await?;
        }
        Action::Expiry(kind) => {
            let (name, recreate) = match dialogue.get().await?.unwrap_or_default() {
                State::AwaitingExpiry { name, recreate } => (name, recreate),
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
                dialogue.update(State::AwaitingCustomExpiry { name, recreate }).await?;
            } else {
                let expires = if kind == "none" { None } else { Some(kind.clone()) };
                bot.send_message(chat, i18n::psk_step(lang, settings.psk_default()))
                    .reply_markup(menu::psk_step(lang, settings.psk_default()))
                    .parse_mode(ParseMode::Html)
                    .await?;
                dialogue.update(State::AwaitingPsk { name, expires, recreate }).await?;
            }
        }
        Action::AddPsk(psk) => {
            let (name, expires, recreate) = match dialogue.get().await?.unwrap_or_default() {
                State::AwaitingPsk { name, expires, recreate } => (name, expires, recreate),
                _ => {
                    bot.send_message(chat, session_expired_text(lang))
                        .reply_markup(menu::main_menu(lang))
                        .parse_mode(ParseMode::Html)
                        .await?;
                    return Ok(());
                }
            };
            finish_add(&bot, chat, &vpn, lang, &name, expires.as_deref(), psk, recreate).await;
            dialogue.exit().await?;
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
        Action::Backup => {
            bot.send_message(chat, i18n::backup_menu_title(lang))
                .reply_markup(menu::backup_menu(lang))
                .parse_mode(ParseMode::Html)
                .await?;
        }
        Action::BackupNew => {
            let waiting = bot.send_message(chat, i18n::backup_creating(lang)).await.ok();
            match vpn.backup().await {
                Ok(bf) => {
                    // Свежесозданный бэкап — самый новый по mtime, т.е. индекс 0 в list_backups().
                    bot.send_message(chat, i18n::backup_done(lang, &bf.name))
                        .reply_markup(menu::backup_card(lang, 0))
                        .parse_mode(ParseMode::Html)
                        .await?;
                }
                Err(e) => {
                    tracing::error!(error = %e, "backup провалился");
                    bot.send_message(chat, i18n::error_text(lang, &e)).await?;
                }
            }
            if let Some(m) = waiting {
                let _ = bot.delete_message(chat, m.id).await;
            }
        }
        Action::BackupList => match vpn.list_backups() {
            Ok(list) if list.is_empty() => {
                bot.send_message(chat, i18n::backups_empty(lang))
                    .reply_markup(menu::main_menu(lang))
                    .await?;
            }
            Ok(list) => {
                bot.send_message(chat, i18n::backups_list_title(lang))
                    .reply_markup(menu::backups_list(lang, &list))
                    .parse_mode(ParseMode::Html)
                    .await?;
            }
            Err(e) => {
                bot.send_message(chat, i18n::error_text(lang, &e)).await?;
            }
        },
        Action::BackupCard(idx) => match vpn.list_backups() {
            Ok(list) => match list.get(idx) {
                Some(bf) => {
                    let text = format!("<code>{}</code>", i18n::html_escape(&bf.name));
                    bot.send_message(chat, text)
                        .reply_markup(menu::backup_card(lang, idx))
                        .parse_mode(ParseMode::Html)
                        .await?;
                }
                None => {
                    bot.send_message(chat, i18n::backup_not_found(lang))
                        .reply_markup(menu::main_menu(lang))
                        .await?;
                }
            },
            Err(e) => {
                bot.send_message(chat, i18n::error_text(lang, &e)).await?;
            }
        },
        Action::BackupDownload(idx) => match vpn.list_backups() {
            Ok(list) => match list.get(idx) {
                Some(bf) => {
                    if let Err(e) = bot.send_document(chat, InputFile::file(&bf.path)).await {
                        tracing::error!(error = %e, "send_document провалился");
                        let err = crate::error::Error::Telegram(e.to_string());
                        bot.send_message(chat, i18n::error_text(lang, &err)).await?;
                    }
                }
                None => {
                    bot.send_message(chat, i18n::backup_not_found(lang))
                        .reply_markup(menu::main_menu(lang))
                        .await?;
                }
            },
            Err(e) => {
                bot.send_message(chat, i18n::error_text(lang, &e)).await?;
            }
        },
        Action::Restore(idx) => match vpn.list_backups() {
            Ok(list) => match list.get(idx) {
                Some(bf) => {
                    bot.send_message(chat, i18n::confirm_restore(lang, &bf.name))
                        .reply_markup(menu::confirm_restore(lang, idx))
                        .parse_mode(ParseMode::Html)
                        .await?;
                }
                None => {
                    bot.send_message(chat, i18n::backup_not_found(lang))
                        .reply_markup(menu::main_menu(lang))
                        .await?;
                }
            },
            Err(e) => {
                bot.send_message(chat, i18n::error_text(lang, &e)).await?;
            }
        },
        Action::RestoreYes(idx) => {
            let waiting = bot.send_message(chat, i18n::restoring(lang)).await.ok();
            match vpn.restore(idx).await {
                Ok(()) => {
                    bot.send_message(chat, i18n::restore_done(lang))
                        .reply_markup(menu::main_menu(lang))
                        .parse_mode(ParseMode::Html)
                        .await?;
                }
                Err(e) => {
                    tracing::error!(error = %e, "restore провалился");
                    bot.send_message(chat, i18n::error_text(lang, &e)).await?;
                }
            }
            if let Some(m) = waiting {
                let _ = bot.delete_message(chat, m.id).await;
            }
        }
        Action::Check => {
            let waiting = bot.send_message(chat, i18n::check_running(lang)).await.ok();
            match vpn.check().await {
                Ok(body) => {
                    let body = truncate_for_message(body);
                    bot.send_message(chat, i18n::check_result(lang, &body))
                        .parse_mode(ParseMode::Html)
                        .reply_markup(menu::main_menu(lang))
                        .await?;
                }
                Err(e) => {
                    tracing::error!(error = %e, "check провалился");
                    bot.send_message(chat, i18n::error_text(lang, &e)).await?;
                }
            }
            if let Some(m) = waiting {
                let _ = bot.delete_message(chat, m.id).await;
            }
        }
        Action::Diagnose => {
            let waiting = bot.send_message(chat, i18n::diagnose_running(lang)).await.ok();
            match vpn.diagnose().await {
                Ok(body) => {
                    let body = truncate_for_message(body);
                    bot.send_message(chat, i18n::diagnose_result(lang, &body))
                        .reply_markup(menu::main_menu(lang))
                        .parse_mode(ParseMode::Html)
                        .await?;
                }
                Err(e) => {
                    tracing::error!(error = %e, "diagnose провалился");
                    bot.send_message(chat, i18n::error_text(lang, &e)).await?;
                }
            }
            if let Some(m) = waiting {
                let _ = bot.delete_message(chat, m.id).await;
            }
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
        assert_eq!(parse_callback("recreate:alice"), Action::Recreate("alice".into()));
        assert_eq!(parse_callback("exp:30d"), Action::Expiry("30d".into()));
        assert_eq!(parse_callback("exp:custom"), Action::Expiry("custom".into()));
        assert_eq!(parse_callback("settings"), Action::Settings);
        assert_eq!(parse_callback("lang:ru"), Action::Lang("ru".into()));
        assert_eq!(parse_callback("lang:en"), Action::Lang("en".into()));
        assert_eq!(parse_callback("set:lang:ru"), Action::SetLang("ru".into()));
        assert_eq!(parse_callback("set:lang:en"), Action::SetLang("en".into()));
        assert_eq!(parse_callback("set:psk:on"), Action::SetPsk(true));
        assert_eq!(parse_callback("set:psk:off"), Action::SetPsk(false));
        assert_eq!(parse_callback("add:psk:on"), Action::AddPsk(true));
        assert_eq!(parse_callback("add:psk:off"), Action::AddPsk(false));
        assert_eq!(parse_callback("backup"), Action::Backup);
        assert_eq!(parse_callback("bk:new"), Action::BackupNew);
        assert_eq!(parse_callback("bk:list"), Action::BackupList);
        assert_eq!(parse_callback("bk:restore_yes:2"), Action::RestoreYes(2));
        assert_eq!(parse_callback("bk:restore:2"), Action::Restore(2));
        assert_eq!(parse_callback("bk:dl:1"), Action::BackupDownload(1));
        assert_eq!(parse_callback("bk:card:0"), Action::BackupCard(0));
        assert_eq!(parse_callback("check"), Action::Check);
        assert_eq!(parse_callback("garbage"), Action::Unknown);
    }

    #[test]
    fn parse_callback_diagnose() {
        assert_eq!(parse_callback("diagnose"), Action::Diagnose);
    }

    #[test]
    fn parse_callback_regen_client() {
        assert_eq!(parse_callback("regen:alice"), Action::Regen("alice".into()));
    }

    #[test]
    fn truncate_for_message_respects_char_boundary() {
        // Трёхбайтовый символ: 3500 не кратно 3 → индекс попадает внутрь
        // символа, обрезка должна откатиться к границе, а не паниковать.
        let long = "€".repeat(1500); // 4500 байт
        let cut = truncate_for_message(long);
        assert!(cut.ends_with('…'));
        assert!(cut.len() <= 3504); // ≤3500 (до границы символа) + "\n…" (4 байта)
        let short = "ok".to_string();
        assert_eq!(truncate_for_message(short), "ok");
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

        let sample_backup =
            crate::vpn::BackupFile { name: "awg_backup_x.tar.gz".into(), path: "x.tar.gz".into(), size: 1, mtime: 1 };

        let keyboards = vec![
            menu::main_menu(Lang::Ru),
            menu::expiry_menu(Lang::Ru),
            menu::client_card(Lang::Ru, "alice"),
            menu::confirm_delete(Lang::Ru, "bob"),
            menu::confirm_recreate(Lang::Ru, "alice"),
            menu::clients_list(Lang::Ru, &[sample_client], &[], 0, 0, 8),
            menu::language_select(),
            menu::settings_menu(Lang::Ru, false),
            menu::settings_menu(Lang::Ru, true),
            menu::psk_step(Lang::Ru, false),
            menu::psk_step(Lang::Ru, true),
            menu::backup_menu(Lang::Ru),
            menu::backups_list(Lang::Ru, &[sample_backup]),
            menu::backup_card(Lang::Ru, 0),
            menu::confirm_restore(Lang::Ru, 0),
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
