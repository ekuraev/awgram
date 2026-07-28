use serde::Deserialize;

use crate::i18n::Lang;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Client {
    pub name: String,
    #[serde(default)]
    pub ip: String,
    #[serde(default)]
    pub client_ipv6: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub status_code: String,
    #[serde(default)]
    pub rx: u64,
    #[serde(default)]
    pub tx: u64,
    #[serde(default)]
    pub last_handshake: Option<i64>,
}

impl Client {
    pub fn active(&self) -> bool {
        self.status_code == "active"
    }

    /// Трёхцветная метка статуса для индикации в списке и карточке.
    /// Опирается на стабильный `status_code` инсталлера (а не на handshake):
    /// инсталлер уже считает эти градации корректно с учётом keepalive/недавности.
    ///   🟢 `active` / `recent`      — подключён / был недавно
    ///   🟡 `no_handshake` / `no_data` — никогда не подключался / нет данных
    ///   🔴 прочее (`inactive`/`key_error`/...) — был, но давно / ошибка ключа
    pub fn status_mark(&self) -> &'static str {
        status_mark_code(&self.status_code)
    }
}

/// Цвет статуса по строковому `status_code`. Вынесен из `Client::status_mark`,
/// чтобы слой `i18n` мог выбрать эмодзи для карточки без владения `Client`.
pub fn status_mark_code(status_code: &str) -> &'static str {
    match status_code {
        "active" | "recent" => "🟢",
        "no_handshake" | "no_data" => "🟡",
        _ => "🔴",
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AddResult {
    pub name: String,
    pub conf_path: String,
    pub qr_path: String,
    pub uri: String,
}

/// Результат массового создания: успешно созданные клиенты (с путями для
/// выдачи) и пропущенные (с причиной). `created.is_empty()` → ничего не
/// создано, альбома не будет.
#[derive(Debug, Clone, PartialEq)]
pub struct BulkResult {
    pub created: Vec<AddResult>,
    pub skipped: Vec<Skip>,
}

/// Пропущенный при массовом создании клиент (коллизия имени / невалидное
/// имя / ошибка генерации). `reason` маппится из `AddStatus` инсталлера.
#[derive(Debug, Clone, PartialEq)]
pub struct Skip {
    pub name: String,
    pub reason: SkipReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    Exists,
    InvalidName,
    Error,
}

/// Свободные адреса в подсети сервера: `total` — usable-хостов (минус
/// network+broadcast), `free` — минус сервер и существующие клиенты.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapacityInfo {
    pub free: u32,
    pub total: u32,
}

pub fn parse_client_list(json: &str) -> Result<Vec<Client>, serde_json::Error> {
    serde_json::from_str(json)
}

pub fn human_bytes(n: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    if n < 1024 {
        return format!("{n} B");
    }
    let mut value = n as f64;
    let mut unit = 0;
    // Advance while the value ROUNDED to 1 decimal is still >= 1024 in this unit.
    while ((value * 10.0).round() / 10.0) >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

/// Человекочитаемое «сколько назад» для last_handshake (epoch, сек).
/// `now` — текущее время (epoch, сек), передаётся явно ради тестируемости.
pub fn format_handshake(lang: Lang, now: i64, hs: i64) -> String {
    if hs <= 0 {
        return match lang {
            Lang::Ru => "никогда",
            Lang::En => "never",
        }
        .to_string();
    }
    let d = now - hs;
    if d < 0 {
        return match lang {
            Lang::Ru => "только что",
            Lang::En => "just now",
        }
        .to_string();
    }
    if d < 60 {
        match lang {
            Lang::Ru => "только что",
            Lang::En => "just now",
        }
        .to_string()
    } else if d < 3600 {
        match lang {
            Lang::Ru => format!("{} мин назад", d / 60),
            Lang::En => format!("{} min ago", d / 60),
        }
    } else if d < 86400 {
        match lang {
            Lang::Ru => format!("{} ч назад", d / 3600),
            Lang::En => format!("{} h ago", d / 3600),
        }
    } else {
        match lang {
            Lang::Ru => format!("{} дн назад", d / 86400),
            Lang::En => format!("{} d ago", d / 86400),
        }
    }
}

/// Компактная метка handshake для кнопки списка («5 мин», а не «5 мин назад»).
/// Те же пороги, что у `format_handshake`, но без хвоста «назад»/«ago» — кнопки
/// Telegram узкие, и каждая морфема на счету. `hs <= 0` → «никогда»/«never»
/// (клиент с `no_handshake` по `status_code` плюс никогда не имевший handshake).
pub fn format_handshake_compact(lang: Lang, now: i64, hs: i64) -> String {
    if hs <= 0 {
        return match lang {
            Lang::Ru => "никогда",
            Lang::En => "never",
        }
        .to_string();
    }
    let d = now - hs;
    if d < 60 {
        match lang {
            Lang::Ru => "сейчас",
            Lang::En => "now",
        }
        .to_string()
    } else if d < 3600 {
        match lang {
            Lang::Ru => format!("{} мин", d / 60),
            Lang::En => format!("{} min", d / 60),
        }
    } else if d < 86400 {
        match lang {
            Lang::Ru => format!("{} ч", d / 3600),
            Lang::En => format!("{} h", d / 3600),
        }
    } else {
        match lang {
            Lang::Ru => format!("{} дн", d / 86400),
            Lang::En => format!("{} d", d / 86400),
        }
    }
}

/// Человекочитаемый срок действия. None → бессрочно.
pub fn format_expiry(lang: Lang, now: i64, exp: Option<i64>) -> String {
    match exp {
        None => match lang {
            Lang::Ru => "бессрочно",
            Lang::En => "no expiry",
        }
        .to_string(),
        Some(e) if e <= now => match lang {
            Lang::Ru => "истёк",
            Lang::En => "expired",
        }
        .to_string(),
        Some(e) => {
            let d = e - now;
            if d >= 86400 {
                match lang {
                    Lang::Ru => format!("ещё {} дн", d / 86400),
                    Lang::En => format!("{} d left", d / 86400),
                }
            } else if d >= 3600 {
                match lang {
                    Lang::Ru => format!("ещё {} ч", d / 3600),
                    Lang::En => format!("{} h left", d / 3600),
                }
            } else {
                match lang {
                    Lang::Ru => "< 1 ч",
                    Lang::En => "< 1 h",
                }
                .to_string()
            }
        }
    }
}

/// Компактная метка срока для кнопки списка клиентов. None → бессрочный
/// клиент (метка не показывается). Пороги — как у `format_expiry`.
pub fn format_expiry_badge(lang: Lang, now: i64, exp: Option<i64>) -> Option<String> {
    let e = exp?;
    let d = e - now;
    let text = if d <= 0 {
        match lang {
            Lang::Ru => "⏳ истёк".to_string(),
            Lang::En => "⏳ expired".to_string(),
        }
    } else if d >= 86400 {
        match lang {
            Lang::Ru => format!("⏳ {}д", d / 86400),
            Lang::En => format!("⏳ {}d", d / 86400),
        }
    } else if d >= 3600 {
        match lang {
            Lang::Ru => format!("⏳ {}ч", d / 3600),
            Lang::En => format!("⏳ {}h", d / 3600),
        }
    } else {
        match lang {
            Lang::Ru => "⏳ <1ч".to_string(),
            Lang::En => "⏳ <1h".to_string(),
        }
    };
    Some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real `list --json` shape: no traffic, no expiry.
    const LIST_JSON: &str = r#"[
      {"name":"alice","ip":"10.0.0.2","client_ipv6":"","status":"Активен","status_code":"active"},
      {"name":"bob","ip":"10.0.0.3","client_ipv6":"","status":"Нет данных","status_code":"no_data"}
    ]"#;

    // Real `stats --json` shape: traffic + last_handshake, no expiry.
    const STATS_JSON: &str = r#"[
      {"name":"alice","ip":"10.0.0.2","rx":1288490188,"tx":356515840,"last_handshake":1752000000,"status":"Активен","status_code":"active"},
      {"name":"bob","ip":"10.0.0.3","rx":0,"tx":0,"last_handshake":0,"status":"Неактивен","status_code":"inactive"}
    ]"#;

    #[test]
    fn parses_list_json() {
        let clients = parse_client_list(LIST_JSON).unwrap();
        assert_eq!(clients.len(), 2);
        assert_eq!(clients[0].name, "alice");
        assert_eq!(clients[0].status_code, "active");
        assert_eq!(clients[0].status, "Активен");
        // list has no traffic fields — must default to 0.
        assert_eq!(clients[0].rx, 0);
        assert_eq!(clients[0].tx, 0);
        assert_eq!(clients[1].name, "bob");
        assert_eq!(clients[1].status_code, "no_data");
    }

    #[test]
    fn parses_stats_json() {
        let clients = parse_client_list(STATS_JSON).unwrap();
        assert_eq!(clients.len(), 2);
        assert_eq!(clients[0].name, "alice");
        assert_eq!(clients[0].rx, 1288490188);
        assert_eq!(clients[0].tx, 356515840);
        assert_eq!(clients[0].last_handshake, Some(1752000000));
        assert_eq!(clients[1].last_handshake, Some(0));
    }

    #[test]
    fn active_true_only_for_active_status_code() {
        let clients = parse_client_list(LIST_JSON).unwrap();
        assert!(clients[0].active());
        assert!(!clients[1].active());

        let stats = parse_client_list(STATS_JSON).unwrap();
        assert!(stats[0].active());
        assert!(!stats[1].active());
    }

    #[test]
    fn human_bytes_formats() {
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(512), "512 B");
        assert_eq!(human_bytes(1536), "1.5 KB");
        assert_eq!(human_bytes(1288490188), "1.2 GB");
        assert_eq!(human_bytes(1048526), "1.0 MB");
        assert_eq!(human_bytes(1073741823), "1.0 GB");
        assert_eq!(human_bytes(1048576), "1.0 MB");
    }

    #[test]
    fn format_handshake_never() {
        assert_eq!(format_handshake(Lang::Ru, 1_700_000_000, 0), "никогда");
    }

    #[test]
    fn format_handshake_never_en() {
        assert_eq!(format_handshake(Lang::En, 2_000_000, 0), "never");
    }

    #[test]
    fn format_handshake_just_now() {
        let now = 1_700_000_000;
        assert_eq!(format_handshake(Lang::Ru, now, now - 30), "только что");
    }

    #[test]
    fn format_handshake_just_now_en() {
        assert_eq!(
            format_handshake(Lang::En, 1_700_000_000, 1_700_000_100),
            "just now"
        );
    }

    #[test]
    fn format_handshake_minutes_ago() {
        let now = 1_700_000_000;
        assert_eq!(format_handshake(Lang::Ru, now, now - 600), "10 мин назад");
    }

    #[test]
    fn format_handshake_minutes_ago_en() {
        let now = 1_700_000_000;
        assert_eq!(format_handshake(Lang::En, now, now - 600), "10 min ago");
    }

    #[test]
    fn format_handshake_hours_ago() {
        let now = 1_700_000_000;
        assert_eq!(format_handshake(Lang::Ru, now, now - 7200), "2 ч назад");
    }

    #[test]
    fn format_handshake_hours_ago_en() {
        let now = 1_700_000_000;
        assert_eq!(format_handshake(Lang::En, now, now - 7200), "2 h ago");
    }

    #[test]
    fn format_handshake_days_ago() {
        let now = 1_700_000_000;
        assert_eq!(format_handshake(Lang::Ru, now, now - 172800), "2 дн назад");
    }

    #[test]
    fn format_handshake_days_ago_en() {
        let now = 1_700_000_000;
        assert_eq!(format_handshake(Lang::En, now, now - 172800), "2 d ago");
    }

    #[test]
    fn format_expiry_none_is_unlimited() {
        assert_eq!(format_expiry(Lang::Ru, 1_700_000_000, None), "бессрочно");
    }

    #[test]
    fn format_expiry_none_is_unlimited_en() {
        assert_eq!(format_expiry(Lang::En, 1_700_000_000, None), "no expiry");
    }

    #[test]
    fn format_expiry_past_is_expired() {
        let now = 1_700_000_000;
        assert_eq!(format_expiry(Lang::Ru, now, Some(now - 1)), "истёк");
        assert_eq!(format_expiry(Lang::Ru, now, Some(now)), "истёк");
    }

    #[test]
    fn format_expiry_past_is_expired_en() {
        let now = 1_700_000_000;
        assert_eq!(format_expiry(Lang::En, now, Some(now - 1)), "expired");
        assert_eq!(format_expiry(Lang::En, now, Some(now)), "expired");
    }

    #[test]
    fn format_expiry_days_remaining() {
        let now = 1_700_000_000;
        assert_eq!(format_expiry(Lang::Ru, now, Some(now + 172800)), "ещё 2 дн");
    }

    #[test]
    fn format_expiry_days_remaining_en() {
        let now = 1_700_000_000;
        assert_eq!(format_expiry(Lang::En, now, Some(now + 86400)), "1 d left");
    }

    #[test]
    fn format_expiry_hours_remaining() {
        let now = 1_700_000_000;
        assert_eq!(format_expiry(Lang::Ru, now, Some(now + 7200)), "ещё 2 ч");
    }

    #[test]
    fn format_expiry_hours_remaining_en() {
        let now = 1_700_000_000;
        assert_eq!(format_expiry(Lang::En, now, Some(now + 7200)), "2 h left");
    }

    #[test]
    fn format_expiry_under_an_hour_remaining() {
        let now = 1_700_000_000;
        assert_eq!(format_expiry(Lang::Ru, now, Some(now + 600)), "< 1 ч");
    }

    #[test]
    fn format_expiry_under_an_hour_remaining_en() {
        let now = 1_700_000_000;
        assert_eq!(format_expiry(Lang::En, now, Some(now + 600)), "< 1 h");
    }

    #[test]
    fn format_handshake_future_reads_just_now() {
        assert_eq!(
            format_handshake(Lang::Ru, 1_700_000_000, 1_700_000_100),
            "только что"
        );
    }

    // --- format_handshake_compact: те же пороги, что у format_handshake,
    // но без хвоста «назад»/«ago» — для узких кнопок списка клиентов. ---

    #[test]
    fn format_handshake_compact_never() {
        assert_eq!(format_handshake_compact(Lang::Ru, 1_700_000_000, 0), "никогда");
        assert_eq!(format_handshake_compact(Lang::En, 1_700_000_000, -5), "never");
    }

    #[test]
    fn format_handshake_compact_now() {
        let now = 1_700_000_000;
        assert_eq!(format_handshake_compact(Lang::Ru, now, now - 30), "сейчас");
        assert_eq!(format_handshake_compact(Lang::En, now, now + 10), "now");
    }

    #[test]
    fn format_handshake_compact_minutes() {
        let now = 1_700_000_000;
        assert_eq!(format_handshake_compact(Lang::Ru, now, now - 600), "10 мин");
        assert_eq!(format_handshake_compact(Lang::En, now, now - 600), "10 min");
    }

    #[test]
    fn format_handshake_compact_hours() {
        let now = 1_700_000_000;
        assert_eq!(format_handshake_compact(Lang::Ru, now, now - 7200), "2 ч");
        assert_eq!(format_handshake_compact(Lang::En, now, now - 7200), "2 h");
    }

    #[test]
    fn format_handshake_compact_days() {
        let now = 1_700_000_000;
        assert_eq!(format_handshake_compact(Lang::Ru, now, now - 172800), "2 дн");
        assert_eq!(format_handshake_compact(Lang::En, now, now - 172800), "2 d");
    }

    #[test]
    fn format_handshake_compact_boundary_60_seconds() {
        let now = 2_000_000;
        assert_eq!(format_handshake_compact(Lang::Ru, now, now - 60), "1 мин");
    }

    // --- status_mark_code: трёхцветная карта по status_code инсталлера. ---

    #[test]
    fn status_mark_green_for_active_and_recent() {
        assert_eq!(status_mark_code("active"), "🟢");
        assert_eq!(status_mark_code("recent"), "🟢");
    }

    #[test]
    fn status_mark_yellow_for_no_handshake_and_no_data() {
        // Клиент, который НИКОГДА не подключался / нет данных — отличается от
        // «был, но давно»: жёлтый, а не красный.
        assert_eq!(status_mark_code("no_handshake"), "🟡");
        assert_eq!(status_mark_code("no_data"), "🟡");
    }

    #[test]
    fn status_mark_red_for_inactive_key_error_and_unknown() {
        assert_eq!(status_mark_code("inactive"), "🔴");
        assert_eq!(status_mark_code("key_error"), "🔴");
        // Неизвестный код в будущей версии инсталлера — безопасный дефолт красный.
        assert_eq!(status_mark_code("totally_new_code"), "🔴");
        assert_eq!(status_mark_code(""), "🔴");
    }

    #[test]
    fn client_status_mark_matches_helper() {
        let c = Client {
            name: "x".into(),
            ip: String::new(),
            client_ipv6: String::new(),
            status: String::new(),
            status_code: "no_handshake".into(),
            rx: 0,
            tx: 0,
            last_handshake: None,
        };
        assert_eq!(c.status_mark(), "🟡");
    }

    #[test]
    fn format_handshake_boundary_60_seconds() {
        let now = 2_000_000;
        assert_eq!(format_handshake(Lang::Ru, now, now - 60), "1 мин назад");
    }

    #[test]
    fn format_handshake_boundary_3600_seconds() {
        let now = 2_000_000;
        assert_eq!(format_handshake(Lang::Ru, now, now - 3600), "1 ч назад");
    }

    #[test]
    fn format_handshake_boundary_86400_seconds() {
        let now = 2_000_000;
        assert_eq!(format_handshake(Lang::Ru, now, now - 86400), "1 дн назад");
    }

    #[test]
    fn format_expiry_boundary_1_hour() {
        let now = 2_000_000;
        assert_eq!(format_expiry(Lang::Ru, now, Some(now + 3600)), "ещё 1 ч");
    }

    #[test]
    fn format_expiry_boundary_1_day() {
        let now = 2_000_000;
        assert_eq!(format_expiry(Lang::Ru, now, Some(now + 86400)), "ещё 1 дн");
    }

    #[test]
    fn format_expiry_boundary_exactly_now() {
        let now = 2_000_000;
        assert_eq!(format_expiry(Lang::Ru, now, Some(now)), "истёк");
    }

    #[test]
    fn expiry_badge_none_for_permanent() {
        assert_eq!(format_expiry_badge(Lang::Ru, 1_700_000_000, None), None);
    }

    #[test]
    fn expiry_badge_days() {
        let now = 1_700_000_000;
        assert_eq!(
            format_expiry_badge(Lang::Ru, now, Some(now + 6 * 86400)),
            Some("⏳ 6д".to_string())
        );
        assert_eq!(
            format_expiry_badge(Lang::En, now, Some(now + 6 * 86400)),
            Some("⏳ 6d".to_string())
        );
    }

    #[test]
    fn expiry_badge_hours() {
        let now = 1_700_000_000;
        assert_eq!(
            format_expiry_badge(Lang::Ru, now, Some(now + 5 * 3600)),
            Some("⏳ 5ч".to_string())
        );
        assert_eq!(
            format_expiry_badge(Lang::En, now, Some(now + 5 * 3600)),
            Some("⏳ 5h".to_string())
        );
    }

    #[test]
    fn expiry_badge_under_hour() {
        let now = 1_700_000_000;
        assert_eq!(
            format_expiry_badge(Lang::Ru, now, Some(now + 600)),
            Some("⏳ <1ч".to_string())
        );
        assert_eq!(
            format_expiry_badge(Lang::En, now, Some(now + 600)),
            Some("⏳ <1h".to_string())
        );
    }

    #[test]
    fn expiry_badge_expired() {
        let now = 1_700_000_000;
        assert_eq!(
            format_expiry_badge(Lang::Ru, now, Some(now)),
            Some("⏳ истёк".to_string())
        );
        assert_eq!(
            format_expiry_badge(Lang::En, now, Some(now - 1)),
            Some("⏳ expired".to_string())
        );
    }

    #[test]
    fn bulk_result_default_is_empty() {
        let b = BulkResult {
            created: vec![],
            skipped: vec![],
        };
        assert!(b.created.is_empty());
        assert!(b.skipped.is_empty());
    }

    #[test]
    fn capacity_info_holds_counts() {
        let c = CapacityInfo {
            free: 250,
            total: 254,
        };
        assert_eq!(c.free, 250);
        assert_eq!(c.total, 254);
    }

    #[test]
    fn skip_reason_variants_exist() {
        let s = Skip {
            name: "x".into(),
            reason: SkipReason::Exists,
        };
        assert!(matches!(s.reason, SkipReason::Exists));
    }
}
