use std::path::PathBuf;
use std::sync::Arc;

use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::prelude::*;

use awgram::bot::{handlers, State};
use awgram::config::Config;
use awgram::store::Store;
use awgram::vpn::Vpn;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let cfg_path = std::env::var("AWGRAM_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/etc/awgram/config.toml"));

    let cfg = match Config::load(&cfg_path) {
        Ok(c) => Arc::new(c),
        Err(e) => {
            tracing::error!(error = %e, path = %cfg_path.display(), "не удалось загрузить конфиг");
            std::process::exit(1);
        }
    };
    tracing::info!(admins = cfg.admin_ids.len(), "конфиг загружен");

    let bot = match awgram::net::connect(&cfg.bot_token, &cfg.telegram_proxies).await {
        Some(b) => b,
        None => {
            tracing::error!(
                proxies = cfg.telegram_proxies.len(),
                "ни один прокси из telegram_proxies не отвечает"
            );
            std::process::exit(1);
        }
    };
    if !cfg.telegram_proxies.is_empty() {
        // Умерший в рантайме прокси лечится рестартом: systemd поднимет
        // процесс, и connect() снова выберет первый живой из списка.
        let wd_bot = bot.clone();
        tokio::spawn(async move {
            awgram::net::watch_until_dead(
                || {
                    let b = wd_bot.clone();
                    async move { awgram::net::bot_alive(&b, awgram::net::PROBE_TIMEOUT).await }
                },
                awgram::net::WATCHDOG_INTERVAL,
                awgram::net::WATCHDOG_MAX_FAILURES,
            )
            .await;
            tracing::error!("прокси перестал отвечать — завершаемся для перезапуска");
            std::process::exit(1);
        });
    }
    let vpn = Arc::new(Vpn::from_config(&cfg));
    let store = match Store::open(&cfg.db_path) {
        Ok(s) => Arc::new(s),
        Err(e) => {
            tracing::error!(error = %e, path = %cfg.db_path.display(), "не удалось открыть БД");
            std::process::exit(1);
        }
    };
    store.migrate_state_json(&cfg.state_file);

    tokio::spawn(awgram::collector::run(vpn.clone(), store.clone()));

    tracing::info!("запуск long polling");
    Dispatcher::builder(bot, handlers::schema())
        .dependencies(dptree::deps![InMemStorage::<State>::new(), cfg, vpn, store])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}
