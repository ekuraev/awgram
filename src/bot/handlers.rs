use std::sync::Arc;

use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::dispatching::{HandlerExt, UpdateFilterExt};
use teloxide::prelude::*;
use teloxide::types::{CallbackQuery, InlineKeyboardMarkup, InputFile, MessageId, ParseMode};

use crate::auth::{resolve_role, Role};
use crate::bot::menu;
use crate::bot::render::{self, format_client_card, format_stats};
use crate::bot::State;
use crate::config::Config;
use crate::i18n::{self, Lang};
use crate::store::{EventKind, Store};
use crate::vpn::Vpn;

#[derive(Debug, PartialEq)]
pub enum Action {
    Menu,
    List,
    Add,
    Stats,
    Page(usize),
    ShowClient(String),
    ClientHistory(String),
    SendConf(String),
    AskDelete(String),
    ConfirmDelete(String),
    Recreate(String),
    Regen(String),
    RegenAll,
    RegenAllRun(bool), // true = --reset-routes
    Expiry(String),    // "none" | "1d" | ... | "custom"
    Lang(String),      // "ru" | "en" — язык-гейт при первом /start
    Settings,
    SetLang(String), // "ru" | "en" — смена языка из экрана настроек
    SetPsk(bool),
    SetSlug(bool),
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
    Modify(String),
    ModifyParam(String, crate::vpn::validate::ModifyParam),
    Restart,
    RestartRun,
    RepairModule,
    // --- Массовая генерация (#22) ---
    AddBulk,
    AddBulkRun(usize), // N клиентов для генерации (1..=MAX_BULK, валидируется в обработчике)
    BulkExpiry(String), // "none" | "1d" | ... | "custom" — общий срок для всей пачки
    AddBulkPsk(bool),  // true = включить PSK для генерируемых клиентов
    // --- Артефакты существующего клиента (повторная выдача) ---
    SendQr(String),
    SendLink(String),
    SendAll(String),
    // --- Тумблеры выдачи артефактов в настройках ---
    SetConf(bool),
    SetQr(bool),
    SetLink(bool),
    // --- Фильтр списка клиентов (#28) ---
    SetListFilter(crate::vpn::model::ClientFilter),
    // --- Группы (#20): делегирование управления групповым админам ---
    Groups,
    GroupCreate,
    GroupCard(i64),
    GroupRenameAsk(i64),
    GroupQuotaAsk(i64),
    GroupAdmins(i64),
    GroupAdminRemove(i64, i64),
    GroupInvite(i64),
    GroupInviteRevoke(i64),
    GroupAdminById(i64),
    GroupDeleteAsk(i64),
    GroupDeleteDetach(i64),
    GroupDeleteAllAsk(i64),
    GroupDeleteAllYes(i64),
    GroupRegenAsk(i64),
    GroupRegenRun(i64),
    GroupSelect(i64),
    GroupSelectMenu,
    MoveClientAsk(String),
    MoveClientTo(Option<i64>, String),
    Unknown,
}

fn parse_callback(data: &str) -> Action {
    match data {
        "menu" => Action::Menu,
        "list" => Action::List,
        "add" => Action::Add,
        "addbulk" => Action::AddBulk,
        "stats" => Action::Stats,
        "settings" => Action::Settings,
        "backup" => Action::Backup,
        "bk:new" => Action::BackupNew,
        "bk:list" => Action::BackupList,
        "check" => Action::Check,
        "diagnose" => Action::Diagnose,
        "regen_all" => Action::RegenAll,
        "regen_all_go" => Action::RegenAllRun(false),
        "regen_all_routes" => Action::RegenAllRun(true),
        "restart" => Action::Restart,
        "restart_go" => Action::RestartRun,
        "repair" => Action::RepairModule,
        "groups" => Action::Groups,
        "g:new" => Action::GroupCreate,
        "g:selmenu" => Action::GroupSelectMenu,
        _ => {
            if let Some(v) = data.strip_prefix("g:card:") {
                v.parse().map(Action::GroupCard).unwrap_or(Action::Unknown)
            } else if let Some(v) = data.strip_prefix("g:ren:") {
                v.parse()
                    .map(Action::GroupRenameAsk)
                    .unwrap_or(Action::Unknown)
            } else if let Some(v) = data.strip_prefix("g:quota:") {
                v.parse()
                    .map(Action::GroupQuotaAsk)
                    .unwrap_or(Action::Unknown)
            } else if let Some(v) = data.strip_prefix("g:admdel:") {
                // ДО g:adm: (g:admdel:… тоже начинается с g:adm) — как delyes:/del:.
                let parts: Vec<&str> = v.splitn(2, ':').collect();
                match (
                    parts.first().and_then(|p| p.parse().ok()),
                    parts.get(1).and_then(|p| p.parse().ok()),
                ) {
                    (Some(g), Some(u)) => Action::GroupAdminRemove(g, u),
                    _ => Action::Unknown,
                }
            } else if let Some(v) = data.strip_prefix("g:admid:") {
                // ДО g:adm: — тот же принцип.
                v.parse()
                    .map(Action::GroupAdminById)
                    .unwrap_or(Action::Unknown)
            } else if let Some(v) = data.strip_prefix("g:adm:") {
                v.parse()
                    .map(Action::GroupAdmins)
                    .unwrap_or(Action::Unknown)
            } else if let Some(v) = data.strip_prefix("g:invrev:") {
                // ДО g:inv: — тот же принцип.
                v.parse()
                    .map(Action::GroupInviteRevoke)
                    .unwrap_or(Action::Unknown)
            } else if let Some(v) = data.strip_prefix("g:inv:") {
                v.parse()
                    .map(Action::GroupInvite)
                    .unwrap_or(Action::Unknown)
            } else if let Some(v) = data.strip_prefix("g:delallyes:") {
                // ДО g:delall: — тот же принцип.
                v.parse()
                    .map(Action::GroupDeleteAllYes)
                    .unwrap_or(Action::Unknown)
            } else if let Some(v) = data.strip_prefix("g:delall:") {
                // ДО g:del: — тот же принцип.
                v.parse()
                    .map(Action::GroupDeleteAllAsk)
                    .unwrap_or(Action::Unknown)
            } else if let Some(v) = data.strip_prefix("g:deldetach:") {
                // ДО g:del: — тот же принцип.
                v.parse()
                    .map(Action::GroupDeleteDetach)
                    .unwrap_or(Action::Unknown)
            } else if let Some(v) = data.strip_prefix("g:del:") {
                v.parse()
                    .map(Action::GroupDeleteAsk)
                    .unwrap_or(Action::Unknown)
            } else if let Some(v) = data.strip_prefix("g:regengo:") {
                // ДО g:regen: — тот же принцип.
                v.parse()
                    .map(Action::GroupRegenRun)
                    .unwrap_or(Action::Unknown)
            } else if let Some(v) = data.strip_prefix("g:regen:") {
                v.parse()
                    .map(Action::GroupRegenAsk)
                    .unwrap_or(Action::Unknown)
            } else if let Some(v) = data.strip_prefix("g:sel:") {
                v.parse()
                    .map(Action::GroupSelect)
                    .unwrap_or(Action::Unknown)
            } else if let Some(v) = data.strip_prefix("gmoveto:") {
                // ДО gmove: (gmoveto:… начинается с gmove) — как delyes:/del:.
                let parts: Vec<&str> = v.splitn(2, ':').collect();
                match (parts.first(), parts.get(1)) {
                    (Some(&"none"), Some(name)) => Action::MoveClientTo(None, name.to_string()),
                    (Some(id), Some(name)) => id
                        .parse()
                        .map(|g| Action::MoveClientTo(Some(g), name.to_string()))
                        .unwrap_or(Action::Unknown),
                    _ => Action::Unknown,
                }
            } else if let Some(v) = data.strip_prefix("gmove:") {
                Action::MoveClientAsk(v.to_string())
            } else if let Some(v) = data.strip_prefix("page:") {
                v.parse().map(Action::Page).unwrap_or(Action::Unknown)
            } else if let Some(v) = data.strip_prefix("client:") {
                Action::ShowClient(v.to_string())
            } else if let Some(v) = data.strip_prefix("conf:") {
                Action::SendConf(v.to_string())
            } else if let Some(v) = data.strip_prefix("qr:") {
                Action::SendQr(v.to_string())
            } else if let Some(v) = data.strip_prefix("uri:") {
                Action::SendLink(v.to_string())
            } else if let Some(v) = data.strip_prefix("all:") {
                Action::SendAll(v.to_string())
            } else if let Some(v) = data.strip_prefix("history:") {
                // "history" ничей не префикс среди существующих веток и сам не
                // конфликтует ни с одной из них — порядок относительно соседей
                // произвольный.
                Action::ClientHistory(v.to_string())
            } else if let Some(v) = data.strip_prefix("bulkadd:psk:") {
                // Must be checked before "bulk:" — same reason as delyes:/del:
                // ("bulkadd:..." also starts with "bulk", so "bulk:" would
                // prefix-match it and misparse as AddBulkRun("add:psk:on")).
                Action::AddBulkPsk(v == "on")
            } else if let Some(v) = data.strip_prefix("bulkexp:") {
                // Must be checked before "bulk:" — same reason as delyes:/del:
                // ("bulkexp:..." also starts with "bulk").
                Action::BulkExpiry(v.to_string())
            } else if let Some(v) = data.strip_prefix("bulk:") {
                // Проверяется ПОСЛЕ bulkadd:/bulkexp: (см. выше).
                v.parse().map(Action::AddBulkRun).unwrap_or(Action::Unknown)
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
            } else if let Some(v) = data.strip_prefix("set:slug:") {
                Action::SetSlug(v == "on")
            } else if let Some(v) = data.strip_prefix("set:conf:") {
                Action::SetConf(v == "on")
            } else if let Some(v) = data.strip_prefix("set:qr:") {
                Action::SetQr(v == "on")
            } else if let Some(v) = data.strip_prefix("set:link:") {
                Action::SetLink(v == "on")
            } else if let Some(v) = data.strip_prefix("listfilter:") {
                // Фильтр списка клиентов (#28). "list" — точный match выше (не
                // префикс), так что listfilter: с ним не коллизирует.
                crate::vpn::model::ClientFilter::parse_str(v)
                    .map(Action::SetListFilter)
                    .unwrap_or(Action::Unknown)
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
                v.parse()
                    .map(Action::BackupDownload)
                    .unwrap_or(Action::Unknown)
            } else if let Some(v) = data.strip_prefix("modparam:") {
                // ДО mod: — modparam:... тоже начинается с "mod", но другой разделитель.
                let parts: Vec<&str> = v.splitn(2, ':').collect();
                if parts.len() != 2 {
                    return Action::Unknown;
                }
                let name = parts[0].to_string();
                let param = match parts[1] {
                    "keepalive" => crate::vpn::validate::ModifyParam::Keepalive,
                    "dns" => crate::vpn::validate::ModifyParam::Dns,
                    "allowedips" => crate::vpn::validate::ModifyParam::AllowedIps,
                    "endpoint" => crate::vpn::validate::ModifyParam::Endpoint,
                    _ => return Action::Unknown,
                };
                Action::ModifyParam(name, param)
            } else if let Some(v) = data.strip_prefix("mod:") {
                Action::Modify(v.to_string())
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

/// Рендер экрана навигации (меню / список клиентов / страница списка)
/// редактированием сообщения, на кнопке которого нажали (`msg_id` — это
/// `q.message`). Применяется в `Action::Menu`, `Action::List`, `Action::Page`:
/// меню↔список↔пагинация эволюционируют в одном сообщении — без спама и без
/// глобального HashMap с message_id (его предлагал issue #16, но источник
/// редактируемого сообщения у нас уже есть — это само `q.message`).
///
/// Поведение при ошибках:
/// · `MessageNotModified` (контент не изменился — напр. 🔄 без изменений)
///   — глотаем, это успешный no-op;
/// · любая иная ошибка (сообщение удалено/не текст/устарело) — откат к
///   `send_message`, а со старого сообщения снимается inline-клавиатура
///   (пустой markup), чтобы в чате не висели две живых клавиатуры. Если
///   старое уже удалено — `edit_message_reply_markup` тоже упадёт, ошибку
///   игнорируем.
async fn edit_or_send(
    bot: &Bot,
    chat: ChatId,
    msg_id: MessageId,
    text: String,
    kb: InlineKeyboardMarkup,
) {
    let edit = bot
        .edit_message_text(chat, msg_id, text.clone())
        .reply_markup(kb.clone())
        .parse_mode(ParseMode::Html)
        .await;
    if let Err(e) = edit {
        match e {
            teloxide::errors::RequestError::Api(teloxide::errors::ApiError::MessageNotModified) => {
                // Контент не изменился (нажали 🔄 без изменений) — норма.
            }
            e => {
                tracing::debug!(error = %e, "edit_message_text не удался — отправляю новое");
                // Снимаем клавиатуру со старого сообщения — ниже уйдёт новое с
                // живой клавиатурой, и двух активных рядом быть не должно.
                let _ = bot
                    .edit_message_reply_markup(chat, msg_id)
                    .reply_markup(InlineKeyboardMarkup::default())
                    .await;
                let _ = bot
                    .send_message(chat, text)
                    .reply_markup(kb)
                    .parse_mode(ParseMode::Html)
                    .await;
            }
        }
    }
}

/// Домашняя клавиатура по роли: владельцу — полное меню, групповому админу —
/// его сокращённое (кнопка смены группы при нескольких группах).
pub fn home_menu(role: &Role, lang: Lang) -> InlineKeyboardMarkup {
    match role {
        Role::GroupAdmin(groups) => menu::ga_main_menu(lang, groups.len() > 1),
        _ => menu::main_menu(lang),
    }
}

/// Рабочая группа группового админа: единственная — сразу она; из нескольких —
/// сохранённый выбор, если он всё ещё валиден; иначе None (нужен экран выбора).
pub fn current_ga_group(settings: &Store, uid: i64, groups: &[i64]) -> Option<i64> {
    match groups {
        [only] => Some(*only),
        _ => settings.current_group(uid).filter(|g| groups.contains(g)),
    }
}

/// Перерисовывает экран настроек: заголовок и клавиатура собираются из одних
/// и тех же текущих значений тумблеров (единственное место, где они читаются
/// для этого экрана).
async fn show_settings(bot: &Bot, chat: ChatId, msg_id: MessageId, lang: Lang, settings: &Store) {
    edit_or_send(
        bot,
        chat,
        msg_id,
        i18n::settings_title(
            lang,
            settings.psk_default(),
            settings.name_slug(),
            settings.deliver_conf(),
            settings.deliver_qr(),
            settings.deliver_link(),
        ),
        menu::settings_menu(
            lang,
            settings.psk_default(),
            settings.name_slug(),
            settings.deliver_conf(),
            settings.deliver_qr(),
            settings.deliver_link(),
        ),
    )
    .await;
}

/// Перерисовывает карточку группы: заголовок и клавиатура из одного чтения БД.
async fn show_group_card(
    bot: &Bot,
    chat: ChatId,
    msg_id: MessageId,
    lang: Lang,
    settings: &Store,
    id: i64,
) {
    let Some(g) = settings.group(id) else {
        let _ = bot.send_message(chat, i18n::not_found(lang)).await;
        return;
    };
    let count = settings.group_client_count(id);
    let admins = settings.group_admin_ids(id);
    let has_invite = settings.active_invite(id, now_epoch()).is_some();
    edit_or_send(
        bot,
        chat,
        msg_id,
        i18n::group_card(lang, &g.name, count, g.max_clients, admins.len()),
        menu::group_card_menu(lang, id, has_invite),
    )
    .await;
}

async fn message_handler(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    cfg: Arc<Config>,
    vpn: Arc<Vpn>,
    settings: Arc<Store>,
) -> HandlerResult {
    if !msg.chat.is_private() {
        // Бот доставляет секреты (конфиги, QR, ссылки, бэкапы, диагностику) в чат
        // апдейта, а авторизует по user_id — в группе это грозит утечкой всем
        // участникам. Отклоняем до auth-гейта, чтобы вообще не трогать VPN/settings.
        bot.send_message(msg.chat.id, i18n::private_only()).await?;
        return Ok(());
    }

    // Инвайт-ссылка: /start inv_<token>. Обрабатывается ДО роль-гейта —
    // приглашённый ещё не имеет никакой роли. Токен одноразовый с TTL, так что
    // подбор мусорных токенов даёт лишь "ссылка недействительна".
    if let Some(payload) = msg
        .text()
        .and_then(|t| t.strip_prefix("/start inv_"))
        .map(str::trim)
    {
        let uid = user_id_of_msg(&msg).unwrap_or(0);
        let lang = settings.lang(uid);
        match settings.use_invite(payload, uid, now_epoch()) {
            crate::store::InviteUse::Joined(gid) => {
                let gname = settings.group(gid).map(|g| g.name).unwrap_or_default();
                settings.log_event(
                    now_epoch(),
                    EventKind::InviteUse,
                    None,
                    Some(uid),
                    Some(&format!("group={gid}")),
                );
                settings.log_event(
                    now_epoch(),
                    EventKind::AdminAdd,
                    None,
                    Some(uid),
                    Some(&format!("group={gid} via=invite")),
                );
                settings.set_current_group(uid, gid);
                bot.send_message(msg.chat.id, i18n::joined_group(lang, &gname))
                    .parse_mode(ParseMode::Html)
                    .reply_markup(menu::ga_main_menu(lang, false))
                    .await?;
                // Уведомить владельцев о новом админе.
                for owner in &cfg.admin_ids {
                    let _ = bot
                        .send_message(
                            ChatId(*owner),
                            i18n::owner_notified_join(settings.lang(*owner), uid, &gname),
                        )
                        .parse_mode(ParseMode::Html)
                        .await;
                }
            }
            crate::store::InviteUse::AlreadyAdmin(gid) => {
                let gname = settings.group(gid).map(|g| g.name).unwrap_or_default();
                bot.send_message(msg.chat.id, i18n::joined_group(lang, &gname))
                    .parse_mode(ParseMode::Html)
                    .await?;
            }
            crate::store::InviteUse::Invalid => {
                bot.send_message(msg.chat.id, i18n::invite_invalid(lang))
                    .await?;
            }
        }
        dialogue.update(State::Idle).await?;
        return Ok(());
    }

    let uid = user_id_of_msg(&msg).unwrap_or(0);
    let role = resolve_role(uid, &cfg.admin_ids, &settings);
    if role == Role::Denied {
        tracing::warn!(user_id = uid, "отклонён доступ (message)");
        let lang = settings.lang(uid);
        bot.send_message(msg.chat.id, i18n::access_denied(lang))
            .await?;
        return Ok(());
    }
    let lang = settings.lang(uid);

    let state = dialogue.get().await?.unwrap_or_default();
    match state {
        State::AwaitingName => {
            let name = msg.text().unwrap_or_default().to_string();
            let slug = if settings.name_slug() {
                Some(crate::vpn::validate::gen_slug())
            } else {
                None
            };
            match crate::vpn::validate::normalize_name(&name, slug.as_deref()) {
                Ok(valid) => {
                    match vpn.exists(&valid).await {
                        Ok(false) => {
                            let confirm_line = match lang {
                                Lang::Ru => format!("Клиент: {valid}"),
                                Lang::En => format!("Client: {valid}"),
                            };
                            bot.send_message(
                                msg.chat.id,
                                format!("{confirm_line}\n{}", i18n::ask_expiry(lang)),
                            )
                            .reply_markup(menu::expiry_menu(lang))
                            .await?;
                            dialogue
                                .update(State::AwaitingExpiry {
                                    name: valid,
                                    recreate: false,
                                })
                                .await?;
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
                            dialogue
                                .update(State::AwaitingExpiry {
                                    name: valid,
                                    recreate: false,
                                })
                                .await?;
                        }
                    }
                }
                Err(_e) => {
                    bot.send_message(msg.chat.id, i18n::bad_name(lang, settings.name_slug()))
                        .await?;
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
                    dialogue
                        .update(State::AwaitingPsk {
                            name,
                            expires: Some(exp),
                            recreate,
                        })
                        .await?;
                }
                Err(_e) => {
                    bot.send_message(msg.chat.id, i18n::bad_expiry(lang))
                        .await?;
                }
            }
        }
        State::AwaitingModifyValue { name, param } => {
            let raw = msg.text().unwrap_or_default().to_string();
            match crate::vpn::validate::parse_modify_value(param, &raw) {
                Ok(value) => {
                    let waiting = bot
                        .send_message(msg.chat.id, i18n::creating(lang))
                        .await
                        .ok();
                    match vpn.modify(&name, param, &value).await {
                        Ok(out) => {
                            settings.log_event(
                                now_epoch(),
                                EventKind::Modify,
                                Some(&name),
                                Some(uid),
                                Some(param.as_str()),
                            );
                            if let Some(m) = waiting {
                                let _ = bot.delete_message(msg.chat.id, m.id).await;
                            }
                            bot.send_message(
                                msg.chat.id,
                                i18n::modify_done(lang, param, &out.value),
                            )
                            .reply_markup(menu::main_menu(lang))
                            .parse_mode(ParseMode::Html)
                            .await?;
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "modify провалился");
                            if let Some(m) = waiting {
                                let _ = bot.delete_message(msg.chat.id, m.id).await;
                            }
                            bot.send_message(msg.chat.id, i18n::error_text(lang, &e))
                                .await?;
                        }
                    }
                    dialogue.exit().await?;
                }
                Err(_e) => {
                    // Невалидный ввод — остаёмся в том же state, даём попробовать снова.
                    bot.send_message(
                        msg.chat.id,
                        format!("⚠️ {}", i18n::ask_modify_param(lang, param)),
                    )
                    .await?;
                }
            }
        }
        State::AwaitingModifyParam { name } => {
            // Пользователь ввёл текст вместо нажатия кнопки выбора параметра —
            // не сбрасываем диалог, переспрашиваем с подсказкой.
            bot.send_message(
                msg.chat.id,
                format!("{} {}", i18n::modify_param_select_title(lang), name),
            )
            .reply_markup(menu::modify_param_menu(lang, &name))
            .parse_mode(ParseMode::Html)
            .await?;
        }
        State::AwaitingBulkPrefix => {
            let prefix = msg.text().unwrap_or_default().to_string();
            // Худший случай сразу (count=MAX_BULK, slug по текущей настройке):
            // слишком длинный префикс отбивается на первом шаге, а не после
            // выбора срока и PSK в finish_bulk.
            let slug_enabled = settings.name_slug();
            match crate::vpn::validate::validate_bulk_prefix(prefix.trim(), slug_enabled) {
                Ok(()) => {
                    bot.send_message(msg.chat.id, i18n::ask_bulk_count(lang))
                        .reply_markup(menu::bulk_count_menu(lang))
                        .await?;
                    dialogue
                        .update(State::AwaitingBulkCount {
                            prefix: prefix.trim().to_string(),
                        })
                        .await?;
                }
                Err(_) => {
                    let max = crate::vpn::validate::max_bulk_prefix_len(slug_enabled);
                    bot.send_message(msg.chat.id, i18n::bad_bulk_prefix(lang, max))
                        .await?;
                }
            }
        }
        State::AwaitingBulkCustomExpiry { prefix, count } => {
            let raw = msg.text().unwrap_or_default().to_string();
            match crate::vpn::validate::validate_expiry(&raw) {
                Ok(exp) => {
                    bot.send_message(msg.chat.id, i18n::psk_step(lang, settings.psk_default()))
                        .reply_markup(menu::bulk_psk_step(lang, settings.psk_default()))
                        .parse_mode(ParseMode::Html)
                        .await?;
                    dialogue
                        .update(State::AwaitingBulkPsk {
                            prefix,
                            count,
                            expires: Some(exp),
                        })
                        .await?;
                }
                Err(_) => {
                    bot.send_message(msg.chat.id, i18n::bad_expiry(lang))
                        .await?;
                }
            }
        }
        State::AwaitingGroupName => {
            if !role.is_owner() {
                dialogue.update(State::Idle).await?;
                return Ok(());
            }
            let raw = msg.text().unwrap_or_default().trim().to_string();
            if raw.is_empty() || raw.chars().count() > 32 {
                bot.send_message(msg.chat.id, i18n::bad_group_name(lang))
                    .await?;
            } else {
                match settings.create_group(&raw, now_epoch()) {
                    Ok(_) => {
                        settings.log_event(
                            now_epoch(),
                            EventKind::GroupCreate,
                            None,
                            Some(uid),
                            Some(&raw),
                        );
                        bot.send_message(msg.chat.id, i18n::group_created(lang, &raw))
                            .parse_mode(ParseMode::Html)
                            .reply_markup(menu::main_menu(lang))
                            .await?;
                        dialogue.update(State::Idle).await?;
                    }
                    Err(crate::store::GroupError::NameTaken) => {
                        bot.send_message(msg.chat.id, i18n::group_name_taken(lang, &raw))
                            .parse_mode(ParseMode::Html)
                            .await?;
                    }
                    Err(crate::store::GroupError::Db) => {
                        let err = crate::error::Error::Telegram("db".into());
                        bot.send_message(msg.chat.id, i18n::error_text(lang, &err))
                            .await?;
                        dialogue.update(State::Idle).await?;
                    }
                }
            }
        }
        State::AwaitingGroupRename { id } => {
            if !role.is_owner() {
                dialogue.update(State::Idle).await?;
                return Ok(());
            }
            let raw = msg.text().unwrap_or_default().trim().to_string();
            if raw.is_empty() || raw.chars().count() > 32 {
                bot.send_message(msg.chat.id, i18n::bad_group_name(lang))
                    .await?;
            } else {
                match settings.rename_group(id, &raw) {
                    Ok(()) => {
                        settings.log_event(
                            now_epoch(),
                            EventKind::GroupRename,
                            None,
                            Some(uid),
                            Some(&raw),
                        );
                        bot.send_message(msg.chat.id, i18n::group_renamed(lang, &raw))
                            .parse_mode(ParseMode::Html)
                            .reply_markup(menu::main_menu(lang))
                            .await?;
                        dialogue.update(State::Idle).await?;
                    }
                    Err(crate::store::GroupError::NameTaken) => {
                        bot.send_message(msg.chat.id, i18n::group_name_taken(lang, &raw))
                            .parse_mode(ParseMode::Html)
                            .await?;
                    }
                    Err(crate::store::GroupError::Db) => {
                        let err = crate::error::Error::Telegram("db".into());
                        bot.send_message(msg.chat.id, i18n::error_text(lang, &err))
                            .await?;
                        dialogue.update(State::Idle).await?;
                    }
                }
            }
        }
        State::AwaitingGroupQuota { id } => {
            if !role.is_owner() {
                dialogue.update(State::Idle).await?;
                return Ok(());
            }
            let raw = msg.text().unwrap_or_default().trim().to_string();
            match raw.parse::<i64>() {
                Ok(n) if (0..=100_000).contains(&n) => {
                    let quota = if n == 0 { None } else { Some(n) };
                    settings.set_group_quota(id, quota);
                    bot.send_message(msg.chat.id, i18n::group_quota_set(lang, quota))
                        .reply_markup(menu::main_menu(lang))
                        .parse_mode(ParseMode::Html)
                        .await?;
                    dialogue.update(State::Idle).await?;
                }
                _ => {
                    bot.send_message(msg.chat.id, i18n::bad_group_quota(lang))
                        .await?;
                }
            }
        }
        State::AwaitingGroupAdminId { id } => {
            if !role.is_owner() {
                dialogue.update(State::Idle).await?;
                return Ok(());
            }
            let raw = msg.text().unwrap_or_default().trim().to_string();
            match raw.parse::<i64>() {
                Ok(new_admin) if new_admin > 0 => {
                    let first_admin_ever = !settings.has_any_group_admin();
                    let gname = settings.group(id).map(|g| g.name).unwrap_or_default();
                    if settings.add_group_admin(id, new_admin, uid, now_epoch()) {
                        settings.log_event(
                            now_epoch(),
                            EventKind::AdminAdd,
                            None,
                            Some(uid),
                            Some(&format!("group={id} user={new_admin} via=manual")),
                        );
                        bot.send_message(msg.chat.id, i18n::admin_added(lang, new_admin, &gname))
                            .parse_mode(ParseMode::Html)
                            .reply_markup(menu::main_menu(lang))
                            .await?;
                        if first_admin_ever && !settings.name_slug() {
                            bot.send_message(msg.chat.id, i18n::slug_recommend(lang))
                                .reply_markup(menu::slug_recommend_menu(lang))
                                .await?;
                        }
                    } else {
                        bot.send_message(msg.chat.id, i18n::admin_already(lang, new_admin))
                            .await?;
                    }
                    dialogue.update(State::Idle).await?;
                }
                _ => {
                    bot.send_message(msg.chat.id, i18n::bad_admin_id(lang))
                        .await?;
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
async fn finish_add(
    bot: &Bot,
    chat: ChatId,
    vpn: &Vpn,
    settings: &Store,
    lang: Lang,
    name: &str,
    expires: Option<&str>,
    psk: bool,
    recreate: bool,
    uid: i64,
) {
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
            settings.log_event(
                now_epoch(),
                EventKind::ClientAdd,
                Some(name),
                Some(uid),
                None,
            );
            // Фильтр выдачи по тумблерам настроек (deliver_conf/qr/link): после
            // создания шлём только включённые артефакты. Ручная повторная выдача
            // через карточку клиента (SendConf/SendQr/SendLink/SendAll) фильтр
            // игнорирует — это явный запрос конкретного файла.
            if let Err(e) = render::send_client_files_filtered(
                bot,
                chat,
                lang,
                &res,
                settings.deliver_conf(),
                settings.deliver_qr(),
                settings.deliver_link(),
            )
            .await
            {
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

/// Завершающий шаг массовой генерации: превентивные проверки (вместо длинной
/// паузы в скрипте, после которой пользователь видит «ошибка»), затем один
/// вызов `add_many` и выдача альбома .conf одним `sendMediaGroup`.
///
/// Превентивный гейт состоит из трёх проверок до запуска скрипта:
/// · **имена**: `gen_bulk_names` с актуальным slug (smoke-проверка длины/символов);
/// · **capacity**: `vpn.capacity()` — `free == 0` или `free < count` не даём
///   начинать (неинформативно падать внутри add-many-цикла);
/// · **коллизии**: `vpn.list()` ∩ сгенерированные имена — хоть add_many и
///   превратит коллизии в `Skip`, лучше подсветить это ДО создания (fail-fast),
///   чтобы пользователь мог сменить префикс. `list` fail-open (warn + continue):
///   временную недоступность check/list не превращаем в молчаливый отказ.
///
/// Сами клиенты создаются через `add_many` (один вызов скрипта, один apply_config
/// в конце). Альбом .conf шлём только если включён тумблер `deliver_conf` и есть
/// хоть один созданный клиент (пустой альбом Telegram отклонит).
#[allow(clippy::too_many_arguments)]
async fn finish_bulk(
    bot: &Bot,
    chat: ChatId,
    vpn: &Vpn,
    settings: &Store,
    lang: Lang,
    prefix: &str,
    count: usize,
    expires: Option<&str>,
    psk: bool,
    uid: i64,
) {
    let waiting = bot.send_message(chat, i18n::bulk_creating(lang)).await.ok();

    // 1. Генерация имён (slug из настроек — единый для всей пачки, как в add).
    let slug = if settings.name_slug() {
        Some(crate::vpn::validate::gen_slug())
    } else {
        None
    };
    let names = match crate::vpn::validate::gen_bulk_names(prefix, count as u32, slug.as_deref()) {
        Ok(n) => n,
        Err(_) => {
            if let Some(m) = waiting {
                let _ = bot.delete_message(chat, m.id).await;
            }
            let max = crate::vpn::validate::max_bulk_prefix_len(slug.is_some());
            let _ = bot
                .send_message(chat, i18n::bad_bulk_prefix(lang, max))
                .await;
            return;
        }
    };

    // 2. Превентивная проверка свободных адресов (capacity учитывает v4-подсеть).
    match vpn.capacity().await {
        Ok(cap) => {
            if cap.free == 0 {
                if let Some(m) = waiting {
                    let _ = bot.delete_message(chat, m.id).await;
                }
                let _ = bot.send_message(chat, i18n::capacity_exhausted(lang)).await;
                return;
            }
            if (cap.free as usize) < count {
                if let Some(m) = waiting {
                    let _ = bot.delete_message(chat, m.id).await;
                }
                let _ = bot
                    .send_message(
                        chat,
                        i18n::capacity_insufficient(lang, cap.free, count as u32),
                    )
                    .await;
                return;
            }
        }
        Err(_) => {
            if let Some(m) = waiting {
                let _ = bot.delete_message(chat, m.id).await;
            }
            let _ = bot
                .send_message(chat, i18n::capacity_unavailable(lang))
                .await;
            return;
        }
    }

    // 3. Превентивная проверка коллизий (сгенерированные имена ∩ существующие).
    // list() fail-open: если упал, не блокируем создание (add_many сам соберёт
    // коллизии в skipped и вернёт осмысленный результат).
    match vpn.list().await {
        Ok(existing) => {
            let existing_names: std::collections::HashSet<&str> =
                existing.iter().map(|c| c.name.as_str()).collect();
            if let Some(collision) = names.iter().find(|n| existing_names.contains(n.as_str())) {
                if let Some(m) = waiting {
                    let _ = bot.delete_message(chat, m.id).await;
                }
                let _ = bot
                    .send_message(chat, i18n::client_exists(lang, collision))
                    .parse_mode(ParseMode::Html)
                    .await;
                return;
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "list для проверки коллизий упал — продолжаем (fail-open)");
        }
    }

    // 4. Один вызов add_many (сразу со всеми именами). add_many возвращает
    // BulkResult{created, skipped} — все результаты, не только первый.
    match vpn.add_many(&names, expires, psk).await {
        Ok(res) => {
            for r in &res.created {
                settings.log_event(
                    now_epoch(),
                    EventKind::ClientAdd,
                    Some(&r.name),
                    Some(uid),
                    Some("bulk"),
                );
            }
            // 5. Альбом .conf — одним sendMediaGroup (только если включён и есть
            // что отправлять; пустой альбом Telegram отклонит).
            if settings.deliver_conf() && !res.created.is_empty() {
                let conf_paths: Vec<String> =
                    res.created.iter().map(|c| c.conf_path.clone()).collect();
                if let Err(e) = render::send_album(bot, chat, &conf_paths).await {
                    tracing::error!(error = %e, "альбом .conf не отправлен");
                    // Файлы созданы, но не доставлены — сообщаем как ошибку.
                    let _ = bot.send_message(chat, i18n::error_text(lang, &e)).await;
                }
            }
            // 6. Итог: «Создано N» (+ список пропущенных с причинами, если есть).
            let _ = bot
                .send_message(chat, i18n::bulk_result_summary(lang, &res))
                .parse_mode(ParseMode::Html)
                .reply_markup(menu::main_menu(lang))
                .await;
        }
        Err(e) => {
            tracing::error!(error = %e, "add_many провалился");
            let _ = bot.send_message(chat, i18n::error_text(lang, &e)).await;
        }
    }
    if let Some(m) = waiting {
        let _ = bot.delete_message(chat, m.id).await;
    }
}

/// Рендерит экран списка клиентов: list_enriched → filter+sort → expiries →
/// title → clients_list. Общая логика для Action::List / Action::Page /
/// Action::SetListFilter (различаются только страницей и тем, кто читает/
/// устанавливает фильтр из настроек). Пустой список (до или после фильтра) →
/// friendly-сообщение + главное меню.
async fn render_clients_list(
    bot: &Bot,
    chat: ChatId,
    msg_id: MessageId,
    lang: Lang,
    vpn: &Vpn,
    settings: &Store,
    page: usize,
) {
    // list_enriched = status_code из list (корректная трёхцветная классификация)
    // + last_handshake/rx/tx из stats (метка времени для кнопки). Чистый stats
    // (#27) терял жёлтый статус никогда не подключавшихся клиентов (inactive 🔴
    // вместо no_handshake 🟡) — см. vpn::Vpn::list_enriched.
    match vpn.list_enriched().await {
        Ok(all_clients) => {
            // Фильтр + сортировка «онлайн вперёд» (🟢 → 🔴 → 🟡, внутри — по имени).
            // apply_filter_and_sort возвращает owned Vec — clients_list берёт срез по странице.
            let filter = settings.client_filter();
            let clients =
                crate::vpn::model::apply_filter_and_sort(&all_clients, filter, now_epoch());
            if clients.is_empty() {
                edit_or_send(
                    bot,
                    chat,
                    msg_id,
                    i18n::clients_empty(lang),
                    menu::main_menu(lang),
                )
                .await;
                return;
            }
            // Полный вектор (не страница): clients_list индексирует expiries[i]
            // по глобальному i, срез по странице дал бы сдвиг меток на страницах > 0.
            let expiries: Vec<Option<i64>> =
                clients.iter().map(|c| vpn.client_expiry(&c.name)).collect();
            let title =
                i18n::clients_title_filtered(lang, filter, clients.len(), all_clients.len());
            edit_or_send(
                bot,
                chat,
                msg_id,
                title,
                menu::clients_list(lang, &clients, &expiries, now_epoch(), page, 8, filter),
            )
            .await;
        }
        Err(e) => {
            tracing::error!(error = %e, "stats провалился");
            let _ = bot.send_message(chat, i18n::error_text(lang, &e)).await;
        }
    }
}

async fn callback_handler(
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
    cfg: Arc<Config>,
    vpn: Arc<Vpn>,
    settings: Arc<Store>,
) -> HandlerResult {
    bot.answer_callback_query(q.id.clone()).await.ok();

    let src = match &q.message {
        Some(m) => m,
        None => return Ok(()),
    };
    if !src.chat().is_private() {
        // Секреты (конфиги, QR, ссылки, бэкапы, диагностика) уходят в чат
        // апдейта — в группе они утекли бы всем участникам. Callback уже
        // отвечен выше, тут просто молча отказываем без запуска VPN-действий.
        return Ok(());
    }
    let chat = src.chat().id;
    // Сообщение-источник кнопки: навигация (меню/список/страница) редактирует
    // его на месте через edit_or_send, а не отправляет новое — так меню↔список
    // живут в одном сообщении без спама и без глобального хранилища message_id.
    let msg_id = src.id();

    let uid = user_id_of_cb(&q);
    let role = resolve_role(uid, &cfg.admin_ids, &settings);
    if role == Role::Denied {
        tracing::warn!(user_id = uid, "отклонён доступ (callback)");
        return Ok(());
    }
    let lang = settings.lang(uid);

    let data = q.data.clone().unwrap_or_default();
    match parse_callback(&data) {
        Action::Menu => {
            dialogue.update(State::Idle).await?;
            edit_or_send(
                &bot,
                chat,
                msg_id,
                i18n::menu_title(lang),
                menu::main_menu(lang),
            )
            .await;
        }
        Action::List => {
            // Экран списка: stats → filter+sort (фильтр из настроек) → рендер.
            // Фильтр/сортировку см. render_clients_list.
            render_clients_list(&bot, chat, msg_id, lang, &vpn, &settings, 0).await;
        }
        Action::Page(p) => {
            // Пагинация: тот же рендер, но страница p. Фильтр из настроек —
            // переживает навигацию по страницам (Action::Page его не меняет).
            render_clients_list(&bot, chat, msg_id, lang, &vpn, &settings, p).await;
        }
        Action::Stats => match vpn.list_enriched().await {
            Ok(clients) => {
                let now = now_epoch();
                let summary = settings.traffic_summary(None, now);
                let top = settings.top_clients(7, 5, now);
                edit_or_send(
                    &bot,
                    chat,
                    msg_id,
                    format_stats(lang, &clients, now, &summary, &top),
                    menu::main_menu(lang),
                )
                .await;
            }
            Err(e) => {
                bot.send_message(chat, i18n::error_text(lang, &e)).await?;
            }
        },
        Action::ShowClient(name) => match vpn.list_enriched().await {
            Ok(clients) => match clients.iter().find(|c| c.name == name) {
                Some(c) => {
                    let now = now_epoch();
                    let expiry = vpn.client_expiry(&name);
                    let traffic = settings.traffic_summary(Some(&name), now);
                    edit_or_send(
                        &bot,
                        chat,
                        msg_id,
                        format_client_card(lang, c, now, expiry, &traffic),
                        menu::client_card(lang, &name, role.is_owner()),
                    )
                    .await;
                }
                None => {
                    bot.send_message(chat, i18n::not_found(lang)).await?;
                }
            },
            Err(e) => {
                bot.send_message(chat, i18n::error_text(lang, &e)).await?;
            }
        },
        Action::ClientHistory(name) => {
            let now = now_epoch();
            let events = settings.client_events(&name, 10);
            edit_or_send(
                &bot,
                chat,
                msg_id,
                render::format_client_history(lang, &name, &events, now),
                menu::client_history(lang, &name),
            )
            .await;
        }
        Action::SendConf(name) => {
            // 📄 Конфиг — только .conf, без QR/ссылки (фильтр выдачи не применяется:
            // это ручная повторная выдача конкретного артефакта).
            match vpn.existing_files(&name) {
                Ok(res) => {
                    if let Err(e) = bot
                        .send_document(chat, InputFile::file(&res.conf_path))
                        .await
                    {
                        let err = crate::error::Error::Telegram(e.to_string());
                        bot.send_message(chat, i18n::error_text(lang, &err)).await?;
                    }
                }
                Err(e) => {
                    bot.send_message(chat, i18n::error_text(lang, &e)).await?;
                }
            }
        }
        Action::SendQr(name) => {
            // 🖼 QR — опционален (qrencode может отсутствовать на сервере).
            match vpn.existing_files(&name) {
                Ok(res) if std::path::Path::new(&res.qr_path).exists() => {
                    if let Err(e) = bot.send_photo(chat, InputFile::file(&res.qr_path)).await {
                        let err = crate::error::Error::Telegram(e.to_string());
                        bot.send_message(chat, i18n::error_text(lang, &err)).await?;
                    }
                }
                Ok(_) => {
                    bot.send_message(chat, i18n::qr_not_generated(lang)).await?;
                }
                Err(e) => {
                    bot.send_message(chat, i18n::error_text(lang, &e)).await?;
                }
            }
        }
        Action::SendLink(name) => {
            // 🔗 Ссылка vpn:// — опциональна (qrencode генерирует её заодно с QR).
            match vpn.existing_files(&name) {
                Ok(res) if !res.uri.is_empty() => {
                    bot.send_message(chat, i18n::import_link(lang, &res.uri))
                        .parse_mode(ParseMode::Html)
                        .await?;
                }
                Ok(_) => {
                    bot.send_message(chat, i18n::link_unavailable(lang)).await?;
                }
                Err(e) => {
                    bot.send_message(chat, i18n::error_text(lang, &e)).await?;
                }
            }
        }
        Action::SendAll(name) => {
            // 📦 Всё — безусловная выдача conf+QR+ссылка (фильтр настроек игнорируется:
            // пользователь явно запросил всё через карточку клиента).
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
            edit_or_send(
                &bot,
                chat,
                msg_id,
                i18n::confirm_delete(lang, &name),
                menu::confirm_delete(lang, &name),
            )
            .await;
        }
        Action::ConfirmDelete(name) => match vpn.remove(&name).await {
            Ok(()) => {
                settings.log_event(
                    now_epoch(),
                    EventKind::ClientRemove,
                    Some(&name),
                    Some(uid),
                    None,
                );
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
            dialogue
                .update(State::AwaitingExpiry {
                    name,
                    recreate: true,
                })
                .await?;
        }
        Action::Regen(name) => {
            let waiting = bot.send_message(chat, i18n::regen_running(lang)).await.ok();
            match vpn.regen_client(&name).await {
                Ok(res) => {
                    settings.log_event(now_epoch(), EventKind::Regen, Some(&name), Some(uid), None);
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
        Action::RegenAll => {
            if !role.is_owner() {
                return Ok(());
            }
            edit_or_send(
                &bot,
                chat,
                msg_id,
                i18n::confirm_regen_all(lang),
                menu::confirm_regen_all(lang),
            )
            .await;
        }
        Action::RegenAllRun(reset_routes) => {
            if !role.is_owner() {
                return Ok(());
            }
            let waiting = bot
                .send_message(chat, i18n::regen_all_running(lang))
                .await
                .ok();
            let regen_all_result = vpn.regen_all(reset_routes).await;
            if regen_all_result.is_ok() {
                settings.log_event(now_epoch(), EventKind::RegenAll, None, Some(uid), None);
            }
            match regen_all_result {
                Ok(crate::vpn::RegenAllOutcome::NoClients) => {
                    bot.send_message(chat, i18n::clients_empty(lang))
                        .reply_markup(menu::main_menu(lang))
                        .parse_mode(ParseMode::Html)
                        .await?;
                }
                Ok(crate::vpn::RegenAllOutcome::Done(_n)) => {
                    bot.send_message(chat, i18n::regen_all_done(lang))
                        .reply_markup(menu::main_menu(lang))
                        .parse_mode(ParseMode::Html)
                        .await?;
                }
                Ok(crate::vpn::RegenAllOutcome::Partial { .. }) => {
                    bot.send_message(chat, i18n::regen_all_partial(lang))
                        .reply_markup(menu::main_menu(lang))
                        .parse_mode(ParseMode::Html)
                        .await?;
                }
                Err(e) => {
                    tracing::error!(error = %e, "массовый regen провалился");
                    bot.send_message(chat, i18n::error_text(lang, &e)).await?;
                }
            }
            if let Some(m) = waiting {
                let _ = bot.delete_message(chat, m.id).await;
            }
        }
        Action::Add => {
            bot.send_message(chat, i18n::ask_client_name(lang, settings.name_slug()))
                .await?;
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
                bot.send_message(chat, i18n::ask_custom_expiry(lang))
                    .await?;
                dialogue
                    .update(State::AwaitingCustomExpiry { name, recreate })
                    .await?;
            } else {
                let expires = if kind == "none" {
                    None
                } else {
                    Some(kind.clone())
                };
                bot.send_message(chat, i18n::psk_step(lang, settings.psk_default()))
                    .reply_markup(menu::psk_step(lang, settings.psk_default()))
                    .parse_mode(ParseMode::Html)
                    .await?;
                dialogue
                    .update(State::AwaitingPsk {
                        name,
                        expires,
                        recreate,
                    })
                    .await?;
            }
        }
        Action::AddPsk(psk) => {
            let (name, expires, recreate) = match dialogue.get().await?.unwrap_or_default() {
                State::AwaitingPsk {
                    name,
                    expires,
                    recreate,
                } => (name, expires, recreate),
                _ => {
                    bot.send_message(chat, session_expired_text(lang))
                        .reply_markup(menu::main_menu(lang))
                        .parse_mode(ParseMode::Html)
                        .await?;
                    return Ok(());
                }
            };
            finish_add(
                &bot,
                chat,
                &vpn,
                &settings,
                lang,
                &name,
                expires.as_deref(),
                psk,
                recreate,
                uid,
            )
            .await;
            dialogue.exit().await?;
        }
        Action::AddBulk => {
            if !role.is_owner() {
                return Ok(());
            }
            // Шаг 1/4 массового диалога: запрос префикса (текстовый ввод, а не
            // кнопка). Валидация префикса — на следующем шаге (gen_bulk_names с
            // count=1 как smoke-проверка), тут только приглашение к вводу.
            bot.send_message(chat, i18n::ask_bulk_prefix(lang)).await?;
            dialogue.update(State::AwaitingBulkPrefix).await?;
        }
        Action::AddBulkRun(count) => {
            if !role.is_owner() {
                return Ok(());
            }
            // callback_data — untrusted input (craftable). Клавиатура эмитит
            // только 1/3/5/10, но защищаемся от crafted bulk:N извне.
            if count == 0 || count > crate::vpn::validate::MAX_BULK as usize {
                bot.send_message(chat, session_expired_text(lang))
                    .reply_markup(menu::main_menu(lang))
                    .parse_mode(ParseMode::Html)
                    .await?;
                return Ok(());
            }
            // Шаг 2/4: префикс уже введён (AwaitingBulkCount хранит его) —
            // переходим к выбору срока. Кол-во пришло из кнопки bulk_count_menu
            // (1/3/5/10 — префикс уже валиден, max=MAX_BULK держит клавиатура).
            let prefix = match dialogue.get().await?.unwrap_or_default() {
                State::AwaitingBulkCount { prefix } => prefix,
                _ => {
                    bot.send_message(chat, session_expired_text(lang))
                        .reply_markup(menu::main_menu(lang))
                        .parse_mode(ParseMode::Html)
                        .await?;
                    return Ok(());
                }
            };
            bot.send_message(chat, i18n::ask_expiry(lang))
                .reply_markup(menu::bulk_expiry_menu(lang))
                .await?;
            dialogue
                .update(State::AwaitingBulkExpiry { prefix, count })
                .await?;
        }
        Action::BulkExpiry(kind) => {
            if !role.is_owner() {
                return Ok(());
            }
            // Шаг 3/4: срок выбран. «custom» → текстовый ввод срока,
            // иначе — переход к выбору PSK с уже готовым expires.
            let (prefix, count) = match dialogue.get().await?.unwrap_or_default() {
                State::AwaitingBulkExpiry { prefix, count } => (prefix, count),
                _ => {
                    bot.send_message(chat, session_expired_text(lang))
                        .reply_markup(menu::main_menu(lang))
                        .parse_mode(ParseMode::Html)
                        .await?;
                    return Ok(());
                }
            };
            if kind == "custom" {
                bot.send_message(chat, i18n::ask_custom_expiry(lang))
                    .await?;
                dialogue
                    .update(State::AwaitingBulkCustomExpiry { prefix, count })
                    .await?;
            } else {
                let expires = if kind == "none" {
                    None
                } else {
                    Some(kind.clone())
                };
                bot.send_message(chat, i18n::psk_step(lang, settings.psk_default()))
                    .reply_markup(menu::bulk_psk_step(lang, settings.psk_default()))
                    .parse_mode(ParseMode::Html)
                    .await?;
                dialogue
                    .update(State::AwaitingBulkPsk {
                        prefix,
                        count,
                        expires,
                    })
                    .await?;
            }
        }
        Action::AddBulkPsk(psk) => {
            if !role.is_owner() {
                return Ok(());
            }
            // Шаг 4/4: PSK выбран — финальный забег (превентивные проверки +
            // add_many + альбом). После finish_bulk диалог закрывается.
            let (prefix, count, expires) = match dialogue.get().await?.unwrap_or_default() {
                State::AwaitingBulkPsk {
                    prefix,
                    count,
                    expires,
                } => (prefix, count, expires),
                _ => {
                    bot.send_message(chat, session_expired_text(lang))
                        .reply_markup(menu::main_menu(lang))
                        .parse_mode(ParseMode::Html)
                        .await?;
                    return Ok(());
                }
            };
            finish_bulk(
                &bot,
                chat,
                &vpn,
                &settings,
                lang,
                &prefix,
                count,
                expires.as_deref(),
                psk,
                uid,
            )
            .await;
            dialogue.exit().await?;
        }
        Action::Settings => {
            if !role.is_owner() {
                return Ok(());
            }
            show_settings(&bot, chat, msg_id, lang, &settings).await;
        }
        Action::Modify(name) => {
            if !role.is_owner() {
                return Ok(());
            }
            edit_or_send(
                &bot,
                chat,
                msg_id,
                i18n::modify_param_select_title(lang),
                menu::modify_param_menu(lang, &name),
            )
            .await;
            dialogue.update(State::AwaitingModifyParam { name }).await?;
        }
        Action::ModifyParam(name, param) => {
            if !role.is_owner() {
                return Ok(());
            }
            bot.send_message(chat, i18n::ask_modify_param(lang, param))
                .await?;
            dialogue
                .update(State::AwaitingModifyValue { name, param })
                .await?;
        }
        Action::Restart => {
            if !role.is_owner() {
                return Ok(());
            }
            edit_or_send(
                &bot,
                chat,
                msg_id,
                i18n::confirm_restart(lang),
                menu::confirm_restart_menu(lang),
            )
            .await;
        }
        Action::RestartRun => {
            if !role.is_owner() {
                return Ok(());
            }
            let waiting = bot.send_message(chat, i18n::creating(lang)).await.ok();
            match vpn.restart().await {
                Ok(out) => {
                    settings.log_event(now_epoch(), EventKind::Restart, None, Some(uid), None);
                    if let Some(m) = waiting {
                        let _ = bot.delete_message(chat, m.id).await;
                    }
                    bot.send_message(chat, i18n::restart_done(lang, out.active))
                        .reply_markup(menu::main_menu(lang))
                        .parse_mode(ParseMode::Html)
                        .await?;
                }
                Err(e) => {
                    tracing::error!(error = %e, "restart провалился");
                    if let Some(m) = waiting {
                        let _ = bot.delete_message(chat, m.id).await;
                    }
                    bot.send_message(chat, i18n::error_text(lang, &e)).await?;
                }
            }
        }
        Action::RepairModule => {
            if !role.is_owner() {
                return Ok(());
            }
            let waiting = bot.send_message(chat, i18n::creating(lang)).await.ok();
            match vpn.repair_module().await {
                Ok(out) => {
                    settings.log_event(now_epoch(), EventKind::Repair, None, Some(uid), None);
                    if let Some(m) = waiting {
                        let _ = bot.delete_message(chat, m.id).await;
                    }
                    bot.send_message(chat, i18n::repair_result(lang, out.rc))
                        .reply_markup(menu::main_menu(lang))
                        .parse_mode(ParseMode::Html)
                        .await?;
                }
                Err(e) => {
                    tracing::error!(error = %e, "repair-module провалился");
                    if let Some(m) = waiting {
                        let _ = bot.delete_message(chat, m.id).await;
                    }
                    bot.send_message(chat, i18n::error_text(lang, &e)).await?;
                }
            }
        }
        Action::Lang(code) => {
            if let Some(l) = i18n::parse_lang(&code) {
                settings.set_lang(uid, l);
            }
            let lang = settings.lang(uid);
            edit_or_send(
                &bot,
                chat,
                msg_id,
                i18n::menu_title(lang),
                menu::main_menu(lang),
            )
            .await;
        }
        Action::SetLang(code) => {
            if !role.is_owner() {
                return Ok(());
            }
            if let Some(l) = i18n::parse_lang(&code) {
                settings.set_lang(uid, l);
            }
            let lang = settings.lang(uid);
            show_settings(&bot, chat, msg_id, lang, &settings).await;
        }
        Action::SetPsk(on) => {
            if !role.is_owner() {
                return Ok(());
            }
            settings.set_psk_default(on);
            show_settings(&bot, chat, msg_id, lang, &settings).await;
        }
        Action::SetSlug(on) => {
            if !role.is_owner() {
                return Ok(());
            }
            settings.set_name_slug(on);
            show_settings(&bot, chat, msg_id, lang, &settings).await;
        }
        Action::SetConf(on) => {
            if !role.is_owner() {
                return Ok(());
            }
            settings.set_deliver_conf(on);
            show_settings(&bot, chat, msg_id, lang, &settings).await;
        }
        Action::SetQr(on) => {
            if !role.is_owner() {
                return Ok(());
            }
            settings.set_deliver_qr(on);
            show_settings(&bot, chat, msg_id, lang, &settings).await;
        }
        Action::SetLink(on) => {
            if !role.is_owner() {
                return Ok(());
            }
            settings.set_deliver_link(on);
            show_settings(&bot, chat, msg_id, lang, &settings).await;
        }
        Action::SetListFilter(f) => {
            if !role.is_owner() {
                return Ok(());
            }
            // Сохраняем фильтр персистентно, затем перерисовываем список с
            // НУЛЕВОЙ страницей — содержимое сменилось, старая страница могла
            // стать невалидной (напр. был на стр.2 оффлайн, переключил на онлайн).
            settings.set_client_filter(f);
            render_clients_list(&bot, chat, msg_id, lang, &vpn, &settings, 0).await;
        }
        Action::Backup => {
            if !role.is_owner() {
                return Ok(());
            }
            edit_or_send(
                &bot,
                chat,
                msg_id,
                i18n::backup_menu_title(lang),
                menu::backup_menu(lang),
            )
            .await;
        }
        Action::BackupNew => {
            if !role.is_owner() {
                return Ok(());
            }
            let waiting = bot
                .send_message(chat, i18n::backup_creating(lang))
                .await
                .ok();
            match vpn.backup().await {
                Ok(bf) => {
                    settings.log_event(now_epoch(), EventKind::Backup, None, Some(uid), None);
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
        Action::BackupList => {
            if !role.is_owner() {
                return Ok(());
            }
            match vpn.list_backups() {
                Ok(list) if list.is_empty() => {
                    edit_or_send(
                        &bot,
                        chat,
                        msg_id,
                        i18n::backups_empty(lang),
                        menu::main_menu(lang),
                    )
                    .await;
                }
                Ok(list) => {
                    edit_or_send(
                        &bot,
                        chat,
                        msg_id,
                        i18n::backups_list_title(lang),
                        menu::backups_list(lang, &list),
                    )
                    .await;
                }
                Err(e) => {
                    bot.send_message(chat, i18n::error_text(lang, &e)).await?;
                }
            }
        }
        Action::BackupCard(idx) => {
            if !role.is_owner() {
                return Ok(());
            }
            match vpn.list_backups() {
                Ok(list) => match list.get(idx) {
                    Some(bf) => {
                        let text = format!("<code>{}</code>", i18n::html_escape(&bf.name));
                        edit_or_send(&bot, chat, msg_id, text, menu::backup_card(lang, idx)).await;
                    }
                    None => {
                        edit_or_send(
                            &bot,
                            chat,
                            msg_id,
                            i18n::backup_not_found(lang),
                            menu::main_menu(lang),
                        )
                        .await;
                    }
                },
                Err(e) => {
                    bot.send_message(chat, i18n::error_text(lang, &e)).await?;
                }
            }
        }
        Action::BackupDownload(idx) => {
            if !role.is_owner() {
                return Ok(());
            }
            match vpn.list_backups() {
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
            }
        }
        Action::Restore(idx) => {
            if !role.is_owner() {
                return Ok(());
            }
            match vpn.list_backups() {
                Ok(list) => match list.get(idx) {
                    Some(bf) => {
                        edit_or_send(
                            &bot,
                            chat,
                            msg_id,
                            i18n::confirm_restore(lang, &bf.name),
                            menu::confirm_restore(lang, idx),
                        )
                        .await;
                    }
                    None => {
                        edit_or_send(
                            &bot,
                            chat,
                            msg_id,
                            i18n::backup_not_found(lang),
                            menu::main_menu(lang),
                        )
                        .await;
                    }
                },
                Err(e) => {
                    bot.send_message(chat, i18n::error_text(lang, &e)).await?;
                }
            }
        }
        Action::RestoreYes(idx) => {
            if !role.is_owner() {
                return Ok(());
            }
            let waiting = bot.send_message(chat, i18n::restoring(lang)).await.ok();
            match vpn.restore(idx).await {
                Ok(()) => {
                    settings.log_event(now_epoch(), EventKind::Restore, None, Some(uid), None);
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
            if !role.is_owner() {
                return Ok(());
            }
            let waiting = bot.send_message(chat, i18n::check_running(lang)).await.ok();
            match vpn.check().await {
                Ok(report) => {
                    let body = i18n::check_card(lang, &report);
                    bot.send_message(chat, body)
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
            if !role.is_owner() {
                return Ok(());
            }
            let waiting = bot
                .send_message(chat, i18n::diagnose_running(lang))
                .await
                .ok();
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
        Action::Groups => {
            if !role.is_owner() {
                return Ok(());
            }
            let groups: Vec<(crate::store::GroupRow, i64)> = settings
                .list_groups()
                .into_iter()
                .map(|g| {
                    let n = settings.group_client_count(g.id);
                    (g, n)
                })
                .collect();
            let title = if groups.is_empty() {
                i18n::groups_empty(lang)
            } else {
                i18n::groups_title(lang, groups.len())
            };
            edit_or_send(&bot, chat, msg_id, title, menu::groups_menu(lang, &groups)).await;
        }
        Action::GroupCreate => {
            if !role.is_owner() {
                return Ok(());
            }
            bot.send_message(chat, i18n::ask_group_name(lang)).await?;
            dialogue.update(State::AwaitingGroupName).await?;
        }
        Action::GroupCard(id) => {
            if !role.is_owner() {
                return Ok(());
            }
            show_group_card(&bot, chat, msg_id, lang, &settings, id).await;
        }
        Action::GroupRenameAsk(id) => {
            if !role.is_owner() {
                return Ok(());
            }
            bot.send_message(chat, i18n::ask_group_name(lang)).await?;
            dialogue.update(State::AwaitingGroupRename { id }).await?;
        }
        Action::GroupQuotaAsk(id) => {
            if !role.is_owner() {
                return Ok(());
            }
            bot.send_message(chat, i18n::ask_group_quota(lang)).await?;
            dialogue.update(State::AwaitingGroupQuota { id }).await?;
        }
        Action::GroupAdmins(id) => {
            if !role.is_owner() {
                return Ok(());
            }
            let admins = settings.group_admin_ids(id);
            let name = settings.group(id).map(|g| g.name).unwrap_or_default();
            let title = if admins.is_empty() {
                i18n::group_admins_empty(lang)
            } else {
                i18n::group_admins_title(lang, &name)
            };
            edit_or_send(
                &bot,
                chat,
                msg_id,
                title,
                menu::group_admins_menu(lang, id, &admins),
            )
            .await;
        }
        Action::GroupAdminRemove(id, admin_uid) => {
            if !role.is_owner() {
                return Ok(());
            }
            settings.remove_group_admin(id, admin_uid);
            settings.log_event(
                now_epoch(),
                EventKind::AdminRemove,
                None,
                Some(uid),
                Some(&format!("group={id} user={admin_uid}")),
            );
            bot.send_message(chat, i18n::admin_removed(lang, admin_uid))
                .await?;
            show_group_card(&bot, chat, msg_id, lang, &settings, id).await;
        }
        Action::GroupDeleteAsk(id) => {
            if !role.is_owner() {
                return Ok(());
            }
            let name = settings.group(id).map(|g| g.name).unwrap_or_default();
            let count = settings.group_client_count(id);
            edit_or_send(
                &bot,
                chat,
                msg_id,
                i18n::group_delete_choice(lang, &name, count),
                menu::group_delete_choice_menu(lang, id),
            )
            .await;
        }
        Action::GroupDeleteDetach(id) => {
            if !role.is_owner() {
                return Ok(());
            }
            let name = settings.group(id).map(|g| g.name).unwrap_or_default();
            settings.delete_group(id);
            settings.log_event(
                now_epoch(),
                EventKind::GroupDelete,
                None,
                Some(uid),
                Some(&format!("detach {name}")),
            );
            bot.send_message(chat, i18n::group_deleted(lang, &name))
                .parse_mode(ParseMode::Html)
                .reply_markup(menu::main_menu(lang))
                .await?;
        }
        Action::GroupDeleteAllAsk(id) => {
            if !role.is_owner() {
                return Ok(());
            }
            let name = settings.group(id).map(|g| g.name).unwrap_or_default();
            let count = settings.group_client_count(id);
            edit_or_send(
                &bot,
                chat,
                msg_id,
                i18n::confirm_delete_group_clients(lang, &name, count),
                menu::confirm_group_delete_clients_menu(lang, id),
            )
            .await;
        }
        Action::GroupDeleteAllYes(id) => {
            if !role.is_owner() {
                return Ok(());
            }
            let name = settings.group(id).map(|g| g.name).unwrap_or_default();
            let clients = settings.group_client_names(id);
            let waiting = bot
                .send_message(chat, i18n::regen_all_running(lang))
                .await
                .ok();
            let mut failed = 0usize;
            for c in &clients {
                match vpn.remove(c).await {
                    Ok(()) => {
                        settings.log_event(
                            now_epoch(),
                            EventKind::ClientRemove,
                            Some(c),
                            Some(uid),
                            Some("group_delete"),
                        );
                    }
                    Err(e) => {
                        failed += 1;
                        tracing::error!(error = %e, client = %c, "remove при удалении группы провалился");
                    }
                }
            }
            if let Some(m) = waiting {
                let _ = bot.delete_message(chat, m.id).await;
            }
            if failed == 0 {
                settings.delete_group(id);
                settings.log_event(
                    now_epoch(),
                    EventKind::GroupDelete,
                    None,
                    Some(uid),
                    Some(&format!("with_clients {name}")),
                );
                bot.send_message(chat, i18n::group_deleted(lang, &name))
                    .parse_mode(ParseMode::Html)
                    .reply_markup(menu::main_menu(lang))
                    .await?;
            } else {
                // Часть клиентов не удалилась — группу не трогаем, чтобы не
                // потерять привязку выживших. Владелец повторит после починки.
                let err = crate::error::Error::Telegram(format!("{failed} clients not removed"));
                bot.send_message(chat, i18n::error_text(lang, &err)).await?;
            }
        }
        Action::GroupInvite(id) => {
            if !role.is_owner() {
                return Ok(());
            }
            let first_admin_ever = !settings.has_any_group_admin();
            let token = settings.create_invite(id, uid, now_epoch());
            settings.log_event(
                now_epoch(),
                EventKind::InviteCreate,
                None,
                Some(uid),
                Some(&format!("group={id}")),
            );
            let me = bot.get_me().await?;
            let username = me.username.clone().unwrap_or_default();
            let url = format!("https://t.me/{username}?start=inv_{token}");
            let hours = crate::store::INVITE_TTL_SECS / 3600;
            bot.send_message(chat, i18n::invite_link_text(lang, &url, hours))
                .parse_mode(ParseMode::Html)
                .await?;
            if first_admin_ever && !settings.name_slug() {
                bot.send_message(chat, i18n::slug_recommend(lang))
                    .reply_markup(menu::slug_recommend_menu(lang))
                    .await?;
            }
            show_group_card(&bot, chat, msg_id, lang, &settings, id).await;
        }
        Action::GroupInviteRevoke(id) => {
            if !role.is_owner() {
                return Ok(());
            }
            settings.revoke_invite(id);
            settings.log_event(
                now_epoch(),
                EventKind::InviteRevoke,
                None,
                Some(uid),
                Some(&format!("group={id}")),
            );
            bot.send_message(chat, i18n::invite_revoked(lang)).await?;
            show_group_card(&bot, chat, msg_id, lang, &settings, id).await;
        }
        Action::GroupAdminById(id) => {
            if !role.is_owner() {
                return Ok(());
            }
            bot.send_message(chat, i18n::ask_admin_id(lang)).await?;
            dialogue.update(State::AwaitingGroupAdminId { id }).await?;
        }
        // Ветки выбора группы/переноса клиента/перегенерации ключа группы
        // реализуются в задаче 12; до тех пор — Unknown-ответ, чтобы бот
        // оставался рабочим.
        Action::GroupRegenAsk(_)
        | Action::GroupRegenRun(_)
        | Action::GroupSelect(_)
        | Action::GroupSelectMenu
        | Action::MoveClientAsk(_)
        | Action::MoveClientTo(_, _) => {
            bot.send_message(chat, unknown_action_text(lang)).await?;
        }
        Action::Unknown => {
            bot.send_message(chat, unknown_action_text(lang)).await?;
        }
    }
    Ok(())
}

/// dptree-схема для `Dispatcher`. Зависимости (`Arc<Vpn>`, `Arc<Config>`,
/// `Arc<Store>`, `InMemStorage<State>`) регистрируются в `main` через
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
        assert_eq!(
            parse_callback("client:alice"),
            Action::ShowClient("alice".into())
        );
        assert_eq!(
            parse_callback("conf:alice"),
            Action::SendConf("alice".into())
        );
        assert_eq!(
            parse_callback("del:alice"),
            Action::AskDelete("alice".into())
        );
        assert_eq!(
            parse_callback("delyes:alice"),
            Action::ConfirmDelete("alice".into())
        );
        assert_eq!(
            parse_callback("recreate:alice"),
            Action::Recreate("alice".into())
        );
        assert_eq!(parse_callback("exp:30d"), Action::Expiry("30d".into()));
        assert_eq!(
            parse_callback("exp:custom"),
            Action::Expiry("custom".into())
        );
        assert_eq!(parse_callback("settings"), Action::Settings);
        assert_eq!(parse_callback("lang:ru"), Action::Lang("ru".into()));
        assert_eq!(parse_callback("lang:en"), Action::Lang("en".into()));
        assert_eq!(parse_callback("set:lang:ru"), Action::SetLang("ru".into()));
        assert_eq!(parse_callback("set:lang:en"), Action::SetLang("en".into()));
        assert_eq!(parse_callback("set:psk:on"), Action::SetPsk(true));
        assert_eq!(parse_callback("set:psk:off"), Action::SetPsk(false));
        assert_eq!(parse_callback("set:slug:on"), Action::SetSlug(true));
        assert_eq!(parse_callback("set:slug:off"), Action::SetSlug(false));
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
    fn parse_history_callback() {
        assert_eq!(
            parse_callback("history:alice"),
            Action::ClientHistory("alice".to_string())
        );
    }

    #[test]
    fn parse_callback_listfilter_variants() {
        use crate::vpn::model::ClientFilter;
        assert_eq!(
            parse_callback("listfilter:all"),
            Action::SetListFilter(ClientFilter::All)
        );
        assert_eq!(
            parse_callback("listfilter:online"),
            Action::SetListFilter(ClientFilter::Online)
        );
        assert_eq!(
            parse_callback("listfilter:offline"),
            Action::SetListFilter(ClientFilter::Offline)
        );
        assert_eq!(
            parse_callback("listfilter:never"),
            Action::SetListFilter(ClientFilter::Never)
        );
        // Неизвестное значение фильтра → Unknown (craftable callback guard).
        assert_eq!(parse_callback("listfilter:garbage"), Action::Unknown);
    }

    #[test]
    fn parse_callback_listfilter_does_not_collide_with_list() {
        // "list" — точный match (Action::List), "listfilter:..." — префикс.
        // Они не должны пересекаться.
        assert_eq!(parse_callback("list"), Action::List);
        assert!(matches!(
            parse_callback("listfilter:all"),
            Action::SetListFilter(_)
        ));
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
    fn parse_callback_regen_all_variants() {
        assert_eq!(parse_callback("regen_all"), Action::RegenAll);
        assert_eq!(parse_callback("regen_all_go"), Action::RegenAllRun(false));
        assert_eq!(
            parse_callback("regen_all_routes"),
            Action::RegenAllRun(true)
        );
        // "regen_all…" не должен съедаться префиксом "regen:" (там двоеточие).
        assert_eq!(parse_callback("regen:alice"), Action::Regen("alice".into()));
    }

    #[test]
    fn parses_bulk_and_artifact_actions() {
        assert_eq!(parse_callback("bulk:1"), Action::AddBulkRun(1));
        assert_eq!(parse_callback("bulk:10"), Action::AddBulkRun(10));
        assert_eq!(parse_callback("qr:alice"), Action::SendQr("alice".into()));
        assert_eq!(
            parse_callback("uri:alice"),
            Action::SendLink("alice".into())
        );
        assert_eq!(parse_callback("all:alice"), Action::SendAll("alice".into()));
        assert_eq!(parse_callback("set:conf:on"), Action::SetConf(true));
        assert_eq!(parse_callback("set:conf:off"), Action::SetConf(false));
        assert_eq!(parse_callback("set:qr:on"), Action::SetQr(true));
        assert_eq!(parse_callback("set:link:on"), Action::SetLink(true));
    }

    #[test]
    fn parse_callback_addbulk_keyword() {
        assert_eq!(parse_callback("addbulk"), Action::AddBulk);
    }

    #[test]
    fn parse_callback_bulk_expiry_and_psk() {
        assert_eq!(
            parse_callback("bulkexp:none"),
            Action::BulkExpiry("none".into())
        );
        assert_eq!(
            parse_callback("bulkexp:30d"),
            Action::BulkExpiry("30d".into())
        );
        assert_eq!(parse_callback("bulkadd:psk:on"), Action::AddBulkPsk(true));
        assert_eq!(parse_callback("bulkadd:psk:off"), Action::AddBulkPsk(false));
    }

    #[test]
    fn parse_callback_no_collision_uri_vs_other_prefixes() {
        // "uri:" не должен коллизировать с существующими префиксами
        assert_eq!(
            parse_callback("uri:alice"),
            Action::SendLink("alice".into())
        );
        // "all:" — тоже уникален
        assert_eq!(parse_callback("all:alice"), Action::SendAll("alice".into()));
    }

    #[test]
    fn parse_callback_modify_and_restart_and_repair() {
        assert_eq!(parse_callback("mod:alice"), Action::Modify("alice".into()));
        // modparam: должен парситься ДО mod: (длинный префикс), но mod:alice не
        // начинается с modparam:, так что отдельная проверка не нужна — проверяем сам modparam:.
        assert!(matches!(
            parse_callback("modparam:alice:keepalive"),
            Action::ModifyParam(_, _)
        ));
        assert_eq!(parse_callback("restart"), Action::Restart);
        assert_eq!(parse_callback("restart_go"), Action::RestartRun);
        assert_eq!(parse_callback("repair"), Action::RepairModule);
    }

    #[test]
    fn parse_callback_modparam_before_mod_prefix() {
        // modparam:... не должен триггерить mod: — но они разные по разделителю.
        // Проверка: modparam:x:y не парсится как Action::Modify.
        let r = parse_callback("modparam:x:keepalive");
        assert!(!matches!(r, Action::Modify(_)));
    }

    #[test]
    fn parse_callback_group_actions() {
        assert_eq!(parse_callback("groups"), Action::Groups);
        assert_eq!(parse_callback("g:new"), Action::GroupCreate);
        assert_eq!(parse_callback("g:card:5"), Action::GroupCard(5));
        assert_eq!(parse_callback("g:ren:5"), Action::GroupRenameAsk(5));
        assert_eq!(parse_callback("g:quota:5"), Action::GroupQuotaAsk(5));
        assert_eq!(parse_callback("g:adm:5"), Action::GroupAdmins(5));
        assert_eq!(
            parse_callback("g:admdel:5:42"),
            Action::GroupAdminRemove(5, 42)
        );
        assert_eq!(parse_callback("g:inv:5"), Action::GroupInvite(5));
        assert_eq!(parse_callback("g:invrev:5"), Action::GroupInviteRevoke(5));
        assert_eq!(parse_callback("g:admid:5"), Action::GroupAdminById(5));
        assert_eq!(parse_callback("g:del:5"), Action::GroupDeleteAsk(5));
        assert_eq!(
            parse_callback("g:deldetach:5"),
            Action::GroupDeleteDetach(5)
        );
        assert_eq!(parse_callback("g:delall:5"), Action::GroupDeleteAllAsk(5));
        assert_eq!(
            parse_callback("g:delallyes:5"),
            Action::GroupDeleteAllYes(5)
        );
        assert_eq!(parse_callback("g:regen:5"), Action::GroupRegenAsk(5));
        assert_eq!(parse_callback("g:regengo:5"), Action::GroupRegenRun(5));
        assert_eq!(parse_callback("g:sel:5"), Action::GroupSelect(5));
        assert_eq!(parse_callback("g:selmenu"), Action::GroupSelectMenu);
        assert_eq!(
            parse_callback("gmove:alice"),
            Action::MoveClientAsk("alice".into())
        );
        assert_eq!(
            parse_callback("gmoveto:none:alice"),
            Action::MoveClientTo(None, "alice".into())
        );
        assert_eq!(
            parse_callback("gmoveto:7:alice"),
            Action::MoveClientTo(Some(7), "alice".into())
        );
        // мусор → Unknown
        assert_eq!(parse_callback("g:card:x"), Action::Unknown);
        assert_eq!(parse_callback("gmoveto:xx:alice"), Action::Unknown);
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

        let sample_backup = crate::vpn::BackupFile {
            name: "awg_backup_x.tar.gz".into(),
            path: "x.tar.gz".into(),
            size: 1,
            mtime: 1,
        };

        fn sample_group() -> crate::store::GroupRow {
            crate::store::GroupRow {
                id: 1,
                name: "family".into(),
                max_clients: None,
                created_at: 0,
            }
        }

        let keyboards = vec![
            menu::main_menu(Lang::Ru),
            menu::expiry_menu(Lang::Ru),
            menu::client_card(Lang::Ru, "alice", true),
            menu::confirm_delete(Lang::Ru, "bob"),
            menu::confirm_recreate(Lang::Ru, "alice"),
            menu::clients_list(
                Lang::Ru,
                &[sample_client],
                &[],
                0,
                0,
                8,
                crate::vpn::model::ClientFilter::All,
            ),
            menu::language_select(),
            menu::settings_menu(Lang::Ru, false, false, false, false, false),
            menu::settings_menu(Lang::Ru, true, true, true, true, true),
            menu::bulk_count_menu(Lang::Ru),
            menu::bulk_expiry_menu(Lang::Ru),
            menu::psk_step(Lang::Ru, false),
            menu::psk_step(Lang::Ru, true),
            menu::backup_menu(Lang::Ru),
            menu::backups_list(Lang::Ru, &[sample_backup]),
            menu::backup_card(Lang::Ru, 0),
            menu::confirm_restore(Lang::Ru, 0),
            menu::modify_param_menu(Lang::Ru, "alice"),
            menu::confirm_restart_menu(Lang::Ru),
            menu::groups_menu(Lang::Ru, &[(sample_group(), 2)]),
            menu::group_card_menu(Lang::Ru, 1, true),
            menu::group_card_menu(Lang::Ru, 1, false),
            menu::group_admins_menu(Lang::Ru, 1, &[42]),
            menu::group_delete_choice_menu(Lang::Ru, 1),
            menu::confirm_group_delete_clients_menu(Lang::Ru, 1),
            menu::confirm_group_regen_menu(Lang::Ru, 1),
            menu::group_select_menu(Lang::Ru, &[sample_group()]),
            menu::ga_main_menu(Lang::Ru, true),
            menu::move_client_menu(Lang::Ru, "alice", &[sample_group()]),
            menu::slug_recommend_menu(Lang::Ru),
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
