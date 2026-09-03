//! Рассылка владельцам (`admin_ids`) на языке каждого. Владелец, который
//! никогда не писал боту, сообщение не получит — ограничение Bot API;
//! ошибка логируется и не прерывает остальных.

use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardMarkup, ParseMode};

use crate::backup::schedule::TickEvent;
use crate::backup::service;
use crate::bot::menu;
use crate::config::Config;
use crate::i18n::{self, Lang};
use crate::store::Store;
use crate::vpn::model::human_bytes;
use crate::vpn::Vpn;

pub async fn owners(
    bot: &Bot,
    cfg: &Config,
    store: &Store,
    build: impl Fn(Lang) -> (String, InlineKeyboardMarkup),
) {
    for uid in &cfg.admin_ids {
        let (text, kb) = build(store.lang(*uid));
        if let Err(e) = bot
            .send_message(ChatId(*uid), text)
            .parse_mode(ParseMode::Html)
            .reply_markup(kb)
            .await
        {
            tracing::warn!(error = %e, owner = uid, "уведомление владельцу не доставлено");
        }
    }
}

pub async fn on_tick(bot: &Bot, cfg: &Config, store: &Store, vpn: &Vpn, ev: TickEvent) {
    let free = service::free_bytes(&vpn.bot_backups_dir()).map(human_bytes);
    match ev {
        TickEvent::Ok {
            created,
            recovered_after,
            notify_ok,
        } => {
            if let Some(n) = recovered_after {
                let name = created.row.name.clone();
                owners(bot, cfg, store, |l| {
                    (
                        i18n::backup_recovered(l, n, &name),
                        menu::backup_notice_menu(l),
                    )
                })
                .await;
            }
            if notify_ok {
                let rows = service::reconcile(vpn, store);
                let kept = rows.len();
                let pinned = rows.iter().filter(|r| r.pinned).count();
                let keep = store.backup_schedule().keep;
                let secs = created.elapsed_ms.div_ceil(1000);
                let size = human_bytes(created.row.size);
                let name = created.row.name.clone();
                owners(bot, cfg, store, |l| {
                    (
                        i18n::backup_auto_ok(
                            l,
                            &name,
                            &size,
                            secs,
                            kept,
                            keep,
                            pinned,
                            free.as_deref(),
                        ),
                        menu::backup_notice_menu(l),
                    )
                })
                .await;
            }
        }
        TickEvent::Failed {
            error,
            failure,
            notify,
        } => {
            if notify {
                owners(bot, cfg, store, |l| {
                    (
                        i18n::backup_auto_failed(
                            l,
                            &i18n::error_text(l, &error),
                            failure.attempts,
                            &i18n::fmt_ts(l, failure.since),
                            free.as_deref(),
                        ),
                        menu::backup_notice_menu(l),
                    )
                })
                .await;
            }
        }
    }
}
