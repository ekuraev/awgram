use std::path::PathBuf;
use std::sync::Arc;

use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::prelude::*;

use awgram::bot::{handlers, State};
use awgram::config::Config;
use awgram::settings::SettingsStore;
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

    let bot = Bot::new(&cfg.bot_token);
    let vpn = Arc::new(Vpn::from_config(&cfg));
    let settings = Arc::new(SettingsStore::load(cfg.state_file.clone()));

    tracing::info!("запуск long polling");
    Dispatcher::builder(bot, handlers::schema())
        .dependencies(dptree::deps![InMemStorage::<State>::new(), cfg, vpn, settings])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}
