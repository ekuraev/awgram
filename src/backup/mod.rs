//! Бэкапы бота: бандл поверх архива инсталлера, сценарии, расписание,
//! уведомления. Инсталлер про бандл не знает и его не трогает.
pub mod format;
pub mod notify;
pub mod schedule;
pub mod service;

pub use service::Key;

/// Лимит длины комментария к бэкапу (в символах).
pub const MAX_COMMENT_CHARS: usize = 200;
