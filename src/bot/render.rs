use teloxide::prelude::*;
use teloxide::types::{ChatId, InputFile, InputMedia, InputMediaDocument, ParseMode};

use crate::error::{Error, Result};
use crate::i18n::{self, html_escape, Lang};
use crate::store::{EventRow, TrafficSummary};
use crate::vpn::model::{format_expiry, format_handshake, human_bytes, AddResult, Client};

pub fn format_client_card(
    lang: Lang,
    c: &Client,
    now: i64,
    expiry: Option<i64>,
    traffic: &TrafficSummary,
) -> String {
    let mark = c.mark(now);
    let status = i18n::status_label_mark(lang, mark);
    let base = i18n::client_card(
        lang,
        &c.name,
        mark,
        &status,
        &c.ip,
        &human_bytes(c.rx),
        &human_bytes(c.tx),
        &format_handshake(lang, now, c.last_handshake.unwrap_or(0)),
        &format_expiry(lang, now, expiry),
    );
    let periods = i18n::client_card_traffic(
        lang,
        &human_bytes(traffic.today.rx + traffic.today.tx),
        &human_bytes(traffic.d7.rx + traffic.d7.tx),
        &human_bytes(traffic.d30.rx + traffic.d30.tx),
        &human_bytes(traffic.total.rx + traffic.total.tx),
        &format_minutes(lang, traffic.d7.online_minutes),
    );
    format!("{base}\n{periods}")
}

/// «4 ч» / «35 мин» / «2 дн» — короткая длительность (онлайн-время, сессии).
pub fn format_minutes(lang: Lang, minutes: u64) -> String {
    let (m, h, d) = (minutes, minutes / 60, minutes / 1440);
    match lang {
        Lang::Ru if d > 0 => format!("{d} дн"),
        Lang::Ru if h > 0 => format!("{h} ч"),
        Lang::Ru => format!("{m} мин"),
        Lang::En if d > 0 => format!("{d} d"),
        Lang::En if h > 0 => format!("{h} h"),
        Lang::En => format!("{m} min"),
    }
}

/// Экран «История»: события клиента newest→oldest, с человекочитаемым временем
/// и меткой события. Для `offline` дописывает длительность сессии, если среди
/// более старых событий в списке находится парный `online`.
pub fn format_client_history(lang: Lang, name: &str, events: &[EventRow], now: i64) -> String {
    if events.is_empty() {
        return i18n::history_empty(lang, name);
    }
    // События идут новые→старые. Для offline ищем ПАРНЫЙ online дальше по
    // списку (он старше) и дописываем длительность сессии.
    let lines: Vec<String> = events
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let when = format_handshake(lang, now, e.ts);
            let label = i18n::event_label(lang, &e.kind);
            let session = (e.kind == "offline")
                .then(|| {
                    events[i + 1..]
                        .iter()
                        .find(|p| p.kind == "online")
                        .map(|p| {
                            let dur = format_minutes(lang, ((e.ts - p.ts).max(0) / 60) as u64);
                            format!(" ({dur})")
                        })
                })
                .flatten()
                .unwrap_or_default();
            format!("• {when} — {label}{session}")
        })
        .collect();
    i18n::history_screen(lang, name, &lines.join("\n"))
}

/// Стрелка тренда за 7 дней относительно предыдущей недели. Целочисленная
/// арифметика (без f64): рост > +15% → "↑", падение < -15% → "↓", иначе "→".
/// Из тишины (prev==0): любой трафик (cur>0) — рост; 0 vs 0 — без изменений.
pub fn trend_arrow(cur: u64, prev: u64) -> &'static str {
    if prev == 0 {
        return if cur == 0 { "→" } else { "↑" };
    }
    if cur * 100 > prev * 115 {
        "↑"
    } else if cur * 100 < prev * 85 {
        "↓"
    } else {
        "→"
    }
}

/// Расширенный экран статистики: онлайн-счётчик — из живого списка клиентов
/// (`Client::online`), объёмы трафика — из `TrafficSummary` (SQLite-агрегаты),
/// топ клиентов за 7 дней — из `Store::top_clients`.
pub fn format_stats(
    lang: Lang,
    clients: &[Client],
    now: i64,
    summary: &TrafficSummary,
    top: &[(String, u64)],
) -> String {
    let total = clients.len();
    let online = clients.iter().filter(|c| c.online(now)).count();
    let today = human_bytes(summary.today.rx + summary.today.tx);
    let d7 = human_bytes(summary.d7.rx + summary.d7.tx);
    let d30 = human_bytes(summary.d30.rx + summary.d30.tx);
    let all_time = human_bytes(summary.total.rx + summary.total.tx);
    let avg_day = human_bytes((summary.d7.rx + summary.d7.tx) / 7);
    let trend = trend_arrow(
        summary.d7.rx + summary.d7.tx,
        summary.prev7.rx + summary.prev7.tx,
    );
    let top_lines = if top.is_empty() {
        i18n::top_empty(lang)
    } else {
        top.iter()
            .enumerate()
            .map(|(i, (name, bytes))| {
                format!("{}. {} — {}", i + 1, html_escape(name), human_bytes(*bytes))
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    i18n::stats_screen(
        lang, total, online, &today, &d7, &d30, &all_time, &avg_day, trend, &top_lines,
    )
}

pub async fn send_client_files(bot: &Bot, chat: ChatId, lang: Lang, res: &AddResult) -> Result<()> {
    bot.send_document(chat, InputFile::file(&res.conf_path))
        .await
        .map_err(|e| Error::Telegram(e.to_string()))?;
    // QR and URI are generated by the script conditionally (e.g. only if `qrencode`
    // is installed) — only send them when they actually exist.
    if std::path::Path::new(&res.qr_path).exists() {
        bot.send_photo(chat, InputFile::file(&res.qr_path))
            .await
            .map_err(|e| Error::Telegram(e.to_string()))?;
    }
    if !res.uri.is_empty() {
        bot.send_message(chat, i18n::import_link(lang, &res.uri))
            .parse_mode(ParseMode::Html)
            .await
            .map_err(|e| Error::Telegram(e.to_string()))?;
    }
    Ok(())
}

/// Выдаёт `.conf`-файлы одним альбомом через `sendMediaGroup` (2–10 документов).
/// Используется при массовой генерации. Все элементы — одного типа (документы),
/// иначе Telegram отклонит альбом. `conf_paths` пуст → no-op (не ошибка).
pub async fn send_album(bot: &Bot, chat: ChatId, conf_paths: &[String]) -> Result<()> {
    match conf_paths {
        [] => Ok(()),
        [single] => {
            // sendMediaGroup требует 2–10 элементов; один конфиг шлём документом.
            bot.send_document(chat, InputFile::file(single))
                .await
                .map_err(|e| Error::Telegram(e.to_string()))?;
            Ok(())
        }
        paths => {
            let media: Vec<InputMedia> = paths
                .iter()
                .map(|p| InputMedia::Document(InputMediaDocument::new(InputFile::file(p))))
                .collect();
            bot.send_media_group(chat, media)
                .await
                .map_err(|e| Error::Telegram(e.to_string()))?;
            Ok(())
        }
    }
}

/// Авто-выдача после создания с учётом фильтра настроек. `conf/qr/link` —
/// тумблеры из `Store`; каждый артефакт шлётся только если включён и
/// существует (QR/ссылка условны — qrencode может отсутствовать).
pub async fn send_client_files_filtered(
    bot: &Bot,
    chat: ChatId,
    lang: Lang,
    res: &AddResult,
    conf: bool,
    qr: bool,
    link: bool,
) -> Result<()> {
    if conf {
        bot.send_document(chat, InputFile::file(&res.conf_path))
            .await
            .map_err(|e| Error::Telegram(e.to_string()))?;
    }
    if qr && std::path::Path::new(&res.qr_path).exists() {
        bot.send_photo(chat, InputFile::file(&res.qr_path))
            .await
            .map_err(|e| Error::Telegram(e.to_string()))?;
    }
    if link && !res.uri.is_empty() {
        bot.send_message(chat, i18n::import_link(lang, &res.uri))
            .parse_mode(ParseMode::Html)
            .await
            .map_err(|e| Error::Telegram(e.to_string()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::PeriodTotals;

    fn sample() -> Client {
        Client {
            name: "alice".into(),
            ip: "10.0.0.2".into(),
            client_ipv6: String::new(),
            status: "Активен".into(),
            status_code: "active".into(),
            rx: 1288490188,
            tx: 356515840,
            last_handshake: Some(1_700_000_000 - 30), // близко к now — статус 🟢
        }
    }

    #[test]
    fn card_contains_name_and_traffic() {
        let now = 1_700_000_000;
        let expiry = Some(now + 5 * 86400);
        let text = format_client_card(Lang::Ru, &sample(), now, expiry, &TrafficSummary::default());
        assert!(text.contains("alice"));
        assert!(text.contains("Онлайн"));
        assert!(text.contains("1.2 GB"));
        assert!(text.contains("Рукопожатие:")); // строка рукопожатия отрендерена
        assert!(text.contains("ещё")); // истечение через 5 дней
    }

    #[test]
    fn card_escapes_name_html() {
        let now = 1_700_000_000;
        let mut c = sample();
        c.name = "a<b>&c".to_string();
        let text = format_client_card(Lang::Ru, &c, now, None, &TrafficSummary::default());
        assert!(text.contains("a&lt;b&gt;&amp;c"));
        assert!(!text.contains("a<b>&c"));
    }

    #[test]
    fn card_localized_en() {
        let now = 1_700_000_000;
        let text = format_client_card(
            Lang::En,
            &sample(),
            now,
            Some(now + 86400),
            &TrafficSummary::default(),
        );
        assert!(text.contains("Status:"));
        assert!(text.contains("Handshake:"));
        assert!(text.contains("Expires:"));
        assert!(text.contains("1.2 GB"));
        // translated values, not raw backend Russian text
        assert!(text.contains("Online"));
        assert!(text.contains("just now")); // last_handshake is close to `now`
        assert!(text.contains("left")); // expiry is in the future
                                        // no Russian leaked into the EN card
        assert!(!text.contains("Активен"));
        assert!(!text.contains("только что"));
        assert!(!text.contains("ещё"));
    }

    #[test]
    fn stats_counts_clients() {
        let now = 1_700_000_000;
        let clients = vec![
            sample(), // last_handshake близко к now → онлайн
            Client {
                name: "bob".into(),
                ip: "10.0.0.3".into(),
                client_ipv6: String::new(),
                status: "Неактивен".into(),
                status_code: "inactive".into(),
                rx: 0,
                tx: 0,
                last_handshake: None,
            },
        ];
        let summary = TrafficSummary::default();
        let text = format_stats(Lang::Ru, &clients, now, &summary, &[]);
        assert!(text.contains("Клиентов: 2"));
        assert!(text.contains("Онлайн: 1"));
    }

    // Регресс бага: хэндшейк 6 часов назад больше НЕ «Активен»/🟢.
    #[test]
    fn card_stale_handshake_is_offline() {
        let now = 1_700_000_000;
        let mut c = sample();
        c.status_code = "recent".into(); // инсталлер считает это «недавно»
        c.last_handshake = Some(now - 6 * 3600); // а на деле — 6 часов назад
        let text = format_client_card(Lang::Ru, &c, now, None, &TrafficSummary::default());
        assert!(text.contains("🔴"));
        assert!(text.contains("Оффлайн"));
        assert!(!text.contains("🟢"));
    }

    #[test]
    fn stats_counts_online_by_handshake() {
        let now = 1_700_000_000;
        let mut fresh = sample();
        fresh.last_handshake = Some(now - 30);
        let mut stale = sample();
        stale.name = "bob".into();
        stale.status_code = "recent".into();
        stale.last_handshake = Some(now - 7200);
        let summary = TrafficSummary::default();
        let text = format_stats(Lang::Ru, &[fresh, stale], now, &summary, &[]);
        assert!(text.contains("Онлайн: 1"));
    }

    #[test]
    fn trend_arrow_thresholds() {
        assert_eq!(trend_arrow(110, 100), "→"); // +10% — в пределах шума
        assert_eq!(trend_arrow(116, 100), "↑"); // > +15%
        assert_eq!(trend_arrow(84, 100), "↓"); // < -15%
        assert_eq!(trend_arrow(0, 0), "→");
        assert_eq!(trend_arrow(5, 0), "↑"); // из тишины — рост
    }

    #[test]
    fn stats_screen_contains_periods_and_top() {
        let now = 1_700_000_000;
        let mut c = sample();
        c.last_handshake = Some(now - 30);
        let summary = TrafficSummary {
            today: PeriodTotals {
                rx: 1024,
                tx: 512,
                online_minutes: 60,
            },
            d7: PeriodTotals {
                rx: 7 * 1024 * 1024,
                tx: 1024,
                online_minutes: 600,
            },
            d30: PeriodTotals {
                rx: 30 * 1024 * 1024,
                tx: 2048,
                online_minutes: 1200,
            },
            total: PeriodTotals {
                rx: 100 * 1024 * 1024,
                tx: 4096,
                online_minutes: 9000,
            },
            prev7: PeriodTotals {
                rx: 3 * 1024 * 1024,
                tx: 512,
                online_minutes: 300,
            },
        };
        let top = vec![("alice".to_string(), 7 * 1024 * 1024 + 1024_u64)];
        let text = format_stats(Lang::Ru, &[c], now, &summary, &top);
        assert!(text.contains("Сегодня"));
        assert!(text.contains("7 дн"));
        assert!(text.contains("30 дн"));
        assert!(text.contains("alice"));
        assert!(text.contains("↑")); // 7 MB против 3 MB — рост
        assert!(text.contains("Онлайн: 1"));
    }

    #[test]
    fn stats_screen_empty_top_shows_placeholder() {
        // Пустой топ не должен ломать рендер и должен показывать понятную
        // плашку, а не пустую секцию.
        let now = 1_700_000_000;
        let summary = TrafficSummary::default();
        let ru = format_stats(Lang::Ru, &[], now, &summary, &[]);
        assert!(ru.contains("пока нет данных"));
        let en = format_stats(Lang::En, &[], now, &summary, &[]);
        assert!(en.contains("no data yet"));
    }

    #[test]
    fn card_omits_ip_line_when_empty() {
        let now = 1_700_000_000;
        let client = Client {
            name: "charlie".into(),
            ip: String::new(),
            client_ipv6: String::new(),
            status: "Активен".into(),
            status_code: "active".into(),
            rx: 1048576,
            tx: 524288,
            last_handshake: Some(1700000000 - 600),
        };
        let text = format_client_card(Lang::Ru, &client, now, None, &TrafficSummary::default());
        assert!(!text.contains("IP:"));
        assert!(text.contains("charlie"));
        assert!(text.contains("Трафик"));
    }

    #[test]
    fn card_includes_traffic_periods() {
        let now = 1_700_000_000;
        let summary = TrafficSummary {
            today: PeriodTotals {
                rx: 1024,
                tx: 0,
                online_minutes: 30,
            },
            d7: PeriodTotals {
                rx: 2048,
                tx: 1024,
                online_minutes: 240,
            },
            ..Default::default()
        };
        let text = format_client_card(Lang::Ru, &sample(), now, None, &summary);
        assert!(text.contains("Сегодня"));
        assert!(text.contains("7 дн"));
        assert!(text.contains("4 ч")); // 240 минут онлайна за 7 дн
    }

    #[test]
    fn history_lists_events_newest_first() {
        let now = 1_700_000_000;
        let events = vec![
            EventRow {
                ts: now - 60,
                kind: "offline".into(),
                client: Some("alice".into()),
                actor: None,
                details: None,
            },
            EventRow {
                ts: now - 3600,
                kind: "online".into(),
                client: Some("alice".into()),
                actor: None,
                details: None,
            },
            EventRow {
                ts: now - 86400,
                kind: "client_add".into(),
                client: Some("alice".into()),
                actor: Some(42),
                details: None,
            },
        ];
        let text = format_client_history(Lang::Ru, "alice", &events, now);
        assert!(text.contains("отключился"));
        assert!(text.contains("подключился"));
        assert!(text.contains("создан"));
        let off = text.find("отключился").unwrap();
        let on = text.find("подключился").unwrap();
        assert!(off < on); // новые сверху
    }

    #[test]
    fn history_empty_friendly() {
        let text = format_client_history(Lang::Ru, "alice", &[], 0);
        assert!(text.contains("пока нет"));
    }

    #[test]
    fn history_offline_shows_session_duration() {
        let now = 1_700_000_000;
        let events = vec![
            EventRow {
                ts: now - 600,
                kind: "offline".into(),
                client: None,
                actor: None,
                details: None,
            },
            EventRow {
                ts: now - 4200,
                kind: "online".into(),
                client: None,
                actor: None,
                details: None,
            },
        ];
        let text = format_client_history(Lang::Ru, "alice", &events, now);
        assert!(text.contains("(1 ч)")); // сессия 3600 с = 60 мин
    }

    #[test]
    fn send_album_and_filtered_signatures_compile() {
        // Compile-time check: функции существуют с правильными сигнатурами.
        // Реальная отправка тестируется вручную (нужен живой Bot).
        // Оба заимствования привязаны к одному `'a`, чтобы boxed future жил не дольше.
        fn _assert_send_album<'a>(
            bot: &'a teloxide::Bot,
            chat: teloxide::types::ChatId,
            paths: &'a [String],
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::error::Result<()>> + 'a>>
        {
            Box::pin(async move { send_album(bot, chat, paths).await })
        }
        fn _assert_filtered<'a>(
            bot: &'a teloxide::Bot,
            chat: teloxide::types::ChatId,
            lang: crate::i18n::Lang,
            res: &'a crate::vpn::model::AddResult,
            conf: bool,
            qr: bool,
            link: bool,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::error::Result<()>> + 'a>>
        {
            Box::pin(async move {
                send_client_files_filtered(bot, chat, lang, res, conf, qr, link).await
            })
        }
        // если компилируется — контракт сигнатур соблюдён
    }
}
