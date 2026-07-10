use teloxide::prelude::*;
use teloxide::types::{ChatId, InputFile};

use crate::error::{Error, Result};
use crate::vpn::model::{human_bytes, AddResult, Client};

pub fn format_client_card(c: &Client) -> String {
    let status = if c.active { "активен" } else { "отключён" };
    let expires = c.expires_at.as_deref().unwrap_or("бессрочно");
    format!(
        "client: {name}\nстатус: {status} · истекает: {expires}\nтрафик: ↓ {rx}  ↑ {tx}",
        name = c.name,
        rx = human_bytes(c.rx_bytes),
        tx = human_bytes(c.tx_bytes),
    )
}

pub fn format_stats(clients: &[Client]) -> String {
    let total = clients.len();
    let active = clients.iter().filter(|c| c.active).count();
    let rx: u64 = clients.iter().map(|c| c.rx_bytes).sum();
    let tx: u64 = clients.iter().map(|c| c.tx_bytes).sum();
    format!(
        "📊 Статистика\nВсего клиентов: {total}\nАктивных: {active}\nТрафик суммарно: ↓ {rx}  ↑ {tx}",
        rx = human_bytes(rx),
        tx = human_bytes(tx),
    )
}

pub async fn send_client_files(bot: &Bot, chat: ChatId, res: &AddResult) -> Result<()> {
    bot.send_document(chat, InputFile::file(&res.conf_path))
        .await
        .map_err(|e| Error::Telegram(e.to_string()))?;
    bot.send_photo(chat, InputFile::file(&res.qr_path))
        .await
        .map_err(|e| Error::Telegram(e.to_string()))?;
    bot.send_message(chat, format!("🔗 Ссылка для импорта:\n`{}`", res.uri))
        .parse_mode(teloxide::types::ParseMode::MarkdownV2)
        .await
        .map_err(|e| Error::Telegram(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Client {
        Client {
            name: "alice".into(),
            active: true,
            expires_at: Some("2026-08-01".into()),
            rx_bytes: 1288490188,
            tx_bytes: 356515840,
            last_handshake: Some("2026-07-10T10:00:00Z".into()),
        }
    }

    #[test]
    fn card_contains_name_and_traffic() {
        let text = format_client_card(&sample());
        assert!(text.contains("alice"));
        assert!(text.contains("активен"));
        assert!(text.contains("1.2 GB"));
        assert!(text.contains("2026-08-01"));
    }

    #[test]
    fn stats_counts_clients() {
        let clients = vec![sample(), Client { active: false, name: "bob".into(), expires_at: None, rx_bytes: 0, tx_bytes: 0, last_handshake: None }];
        let text = format_stats(&clients);
        assert!(text.contains("2")); // всего клиентов
        assert!(text.contains("1")); // активных
    }
}
