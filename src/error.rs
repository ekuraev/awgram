use crate::config::ConfigError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("ошибка конфигурации: {0}")]
    Config(#[from] ConfigError),
    #[error("скрипт завершился с ошибкой (код {code:?})")]
    ScriptFailed { code: Option<i32>, stderr: String },
    #[error("превышено время ожидания операции")]
    Timeout,
    #[error("не удалось разобрать ответ скрипта: {0}")]
    Parse(String),
    #[error("клиент '{0}' уже существует — скрипт пропустил создание")]
    ClientExists(String),
    #[error("клиент '{0}' не найден")]
    ClientNotFound(String),
    #[error("восстановление провалилось, конфиг откачен к предыдущему состоянию")]
    RestoreRolledBack,
    #[error("ошибка ввода-вывода: {0}")]
    Io(#[from] std::io::Error),
    #[error("ошибка Telegram: {0}")]
    Telegram(String),
    #[error("бэкап не прошёл проверку: {0}")]
    BackupInvalid(#[from] crate::backup::format::FormatError),
    #[error("архив бэкапа недоступен для чтения: {0}")]
    BackupUnreadable(String),
    #[error("бэкап не найден")]
    BackupNotFound,
    #[error("недостаточно места: нужно {need} байт, свободно {free}")]
    BackupNoSpace { need: u64, free: u64 },
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn user_message(&self) -> &'static str {
        match self {
            Error::Config(_) => "Внутренняя ошибка конфигурации.",
            Error::ScriptFailed { .. } => "❌ Операция не удалась. Попробуйте ещё раз.",
            Error::Timeout => "⏳ Превышено время ожидания. Попробуйте позже.",
            Error::Parse(_) => "Не удалось разобрать ответ сервера.",
            Error::ClientExists(_) => "⚠️ Клиент с таким именем уже существует.",
            Error::ClientNotFound(_) => "⚠️ Клиент не найден.",
            Error::RestoreRolledBack => {
                "⚠️ Восстановление провалилось. Конфиг откачен к предыдущему состоянию."
            }
            Error::Io(_) => "❌ Ошибка выполнения операции.",
            Error::Telegram(_) => "❌ Ошибка отправки сообщения.",
            Error::BackupInvalid(_) => "⚠️ Файл бэкапа не прошёл проверку.",
            Error::BackupUnreadable(_) => "❌ Архив бэкапа недоступен для чтения.",
            Error::BackupNotFound => "⚠️ Бэкап не найден.",
            Error::BackupNoSpace { .. } => "❌ Недостаточно места на диске для бэкапа.",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_message_hides_stderr() {
        let e = Error::ScriptFailed {
            code: Some(1),
            stderr: "secret-key-leak".into(),
        };
        assert!(!e.user_message().contains("secret"));
        assert_eq!(
            e.user_message(),
            "❌ Операция не удалась. Попробуйте ещё раз."
        );
    }

    #[test]
    fn client_not_found_user_message() {
        let e = Error::ClientNotFound("ghost".into());
        let m = e.user_message();
        assert!(m.contains("не найден") || m.contains("not found"));
    }

    #[test]
    fn restore_rolled_back_user_message() {
        let e = Error::RestoreRolledBack;
        let m = e.user_message();
        // локализованный текст, без утечки stderr
        assert!(!m.is_empty());
    }
}
