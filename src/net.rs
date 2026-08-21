//! Подключение к Telegram Bot API: выбор прокси и контроль его живости.

use std::time::Duration;

use teloxide::prelude::Requester;
use teloxide::Bot;

pub const PROBE_TIMEOUT: Duration = Duration::from_secs(10);
pub const WATCHDOG_INTERVAL: Duration = Duration::from_secs(60);
pub const WATCHDOG_MAX_FAILURES: u32 = 3;

/// Bot, ходящий в api.telegram.org через указанный прокси.
pub fn proxied_bot(token: &str, proxy_url: &str) -> Result<Bot, reqwest::Error> {
    let client = teloxide::net::default_reqwest_settings()
        .proxy(reqwest::Proxy::all(proxy_url)?)
        .build()?;
    Ok(Bot::with_client(token, client))
}

/// Ошибка уровня транспорта, а не API: до Telegram не достучались.
fn is_network_error(e: &teloxide::RequestError) -> bool {
    matches!(
        e,
        teloxide::RequestError::Network(_) | teloxide::RequestError::Io(_)
    )
}

/// true, если сетевой путь до Telegram живой. Ошибка уровня API
/// (например, неверный токен) — тоже «живой»: ответ ведь пришёл,
/// и смена прокси тут ничего не исправит.
pub async fn bot_alive(bot: &Bot, timeout: Duration) -> bool {
    match tokio::time::timeout(timeout, bot.get_me()).await {
        Ok(Ok(_)) => true,
        // teloxide прячет токен в Display ошибок, логировать безопасно.
        Ok(Err(e)) if is_network_error(&e) => {
            tracing::warn!(error = %e, "getMe не прошёл по сети");
            false
        }
        Ok(Err(e)) => {
            tracing::warn!(error = %e, "getMe вернул ошибку API — сеть работает");
            true
        }
        Err(_) => {
            tracing::warn!(timeout_secs = timeout.as_secs(), "getMe: таймаут");
            false
        }
    }
}

/// Готовый к работе Bot: без прокси — прямое соединение, со списком —
/// первый живой из telegram_proxies. None — все прокси мертвы.
pub async fn connect(token: &str, proxies: &[String]) -> Option<Bot> {
    if proxies.is_empty() {
        return Some(Bot::new(token));
    }
    let idx = select_first_alive(proxies, |p| async move {
        match proxied_bot(token, &p) {
            Ok(bot) => bot_alive(&bot, PROBE_TIMEOUT).await,
            Err(e) => {
                tracing::warn!(proxy = %redact_proxy_url(&p), error = %e, "прокси не сконфигурировать");
                false
            }
        }
    })
    .await?;
    match proxied_bot(token, &proxies[idx]) {
        Ok(bot) => Some(bot),
        Err(e) => {
            tracing::error!(
                proxy = %redact_proxy_url(&proxies[idx]),
                error = %e,
                "не удалось собрать клиент для выбранного прокси"
            );
            None
        }
    }
}

/// URL прокси без кредов — только такой вид допустим в логах и Debug-выводе.
pub fn redact_proxy_url(raw: &str) -> String {
    let Ok(u) = url::Url::parse(raw) else {
        return "<invalid>".to_string();
    };
    if u.username().is_empty() && u.password().is_none() {
        return raw.to_string();
    }
    // Собираем вручную: у прокси-URL значимы только схема, хост и порт,
    // а Url::to_string() ещё и нормализует путь (у http появляется хвостовой «/»).
    let host = u.host_str().unwrap_or("<invalid>");
    match u.port() {
        Some(port) => format!("{}://***@{}:{}", u.scheme(), host, port),
        None => format!("{}://***@{}", u.scheme(), host),
    }
}

/// Индекс первого прокси, чей probe вернул true; список приоритетный,
/// после первого живого остальные не проверяются.
pub async fn select_first_alive<F, Fut>(proxies: &[String], probe: F) -> Option<usize>
where
    F: Fn(String) -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    for (i, p) in proxies.iter().enumerate() {
        let shown = redact_proxy_url(p);
        tracing::info!(proxy = %shown, "проверка прокси");
        if probe(p.clone()).await {
            tracing::info!(proxy = %shown, "выбран прокси");
            return Some(i);
        }
        tracing::warn!(proxy = %shown, "прокси не отвечает");
    }
    None
}

/// Периодически выполняет check и возвращается, когда max_failures
/// проверок подряд провалились; успех сбрасывает счётчик.
pub async fn watch_until_dead<F, Fut>(check: F, interval: std::time::Duration, max_failures: u32)
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let mut failures = 0u32;
    loop {
        tokio::time::sleep(interval).await;
        if check().await {
            failures = 0;
        } else {
            failures += 1;
            tracing::warn!(failures, max_failures, "проверка соединения провалилась");
            if failures >= max_failures {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_masks_credentials() {
        assert_eq!(
            redact_proxy_url("socks5h://user:secret@10.0.0.1:1080"),
            "socks5h://***@10.0.0.1:1080"
        );
    }

    #[test]
    fn redact_masks_username_only() {
        assert_eq!(
            redact_proxy_url("http://user@proxy.example:3128"),
            "http://***@proxy.example:3128"
        );
    }

    #[test]
    fn redact_keeps_url_without_credentials() {
        assert_eq!(
            redact_proxy_url("https://proxy.example:3129"),
            "https://proxy.example:3129"
        );
    }

    #[test]
    fn redact_never_echoes_unparsable_input() {
        assert_eq!(redact_proxy_url("not a url"), "<invalid>");
    }

    #[test]
    fn redact_keeps_ipv6_host_brackets() {
        assert_eq!(
            redact_proxy_url("socks5h://u:p@[2001:db8::1]:1080"),
            "socks5h://***@[2001:db8::1]:1080"
        );
    }

    #[test]
    fn api_error_is_not_network_error() {
        let e = teloxide::RequestError::Api(teloxide::ApiError::Unknown("401".into()));
        assert!(!is_network_error(&e));
    }

    #[test]
    fn io_error_is_network_error() {
        let e = teloxide::RequestError::Io(std::sync::Arc::new(std::io::Error::other(
            "connection refused",
        )));
        assert!(is_network_error(&e));
    }

    fn proxies(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[tokio::test]
    async fn select_returns_first_alive() {
        let idx = select_first_alive(&proxies(&["a", "b", "c"]), |p| async move { p == "b" }).await;
        assert_eq!(idx, Some(1));
    }

    #[tokio::test]
    async fn select_returns_none_when_all_dead() {
        let idx = select_first_alive(&proxies(&["a", "b"]), |_| async { false }).await;
        assert_eq!(idx, None);
    }

    #[tokio::test(start_paused = true)]
    async fn watchdog_returns_after_consecutive_failures() {
        let checks = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let seen = checks.clone();
        watch_until_dead(
            move || {
                let seen = seen.clone();
                async move {
                    seen.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    false
                }
            },
            std::time::Duration::from_secs(60),
            3,
        )
        .await;
        assert_eq!(checks.load(std::sync::atomic::Ordering::SeqCst), 3);
    }

    #[tokio::test(start_paused = true)]
    async fn watchdog_success_resets_failure_counter() {
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let seen = calls.clone();
        watch_until_dead(
            move || {
                let seen = seen.clone();
                async move {
                    let n = seen.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    n == 2 // два провала, успех, дальше только провалы
                }
            },
            std::time::Duration::from_secs(60),
            3,
        )
        .await;
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 6);
    }

    #[tokio::test]
    async fn select_stops_probing_after_first_alive() {
        let probed = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen = probed.clone();
        let idx = select_first_alive(&proxies(&["a", "b", "c"]), move |p| {
            let seen = seen.clone();
            async move {
                seen.lock().unwrap().push(p);
                true
            }
        })
        .await;
        assert_eq!(idx, Some(0));
        assert_eq!(*probed.lock().unwrap(), vec!["a".to_string()]);
    }
}
