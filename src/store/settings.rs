//! Настройки бота в таблице `settings` (key → JSON-значение).
//! API повторяет прежнее JSON-хранилище настроек 1:1 — хендлеры не заметили замены.
//! Точечные чтения SQLite на WAL — микросекунды, поэтому методы синхронные
//! и зовутся прямо из async-хендлеров (как раньше Mutex<BotState>).

use std::path::Path;

use serde::Deserialize;

use crate::i18n::Lang;
use crate::store::Store;
use crate::vpn::model::ClientFilter;

/// Формат старого state.json — только для одноразовой миграции.
#[derive(Deserialize, Default)]
#[serde(default)]
struct LegacyState {
    psk_default: bool,
    name_slug: bool,
    langs: std::collections::HashMap<i64, Lang>,
    deliver_conf: Option<bool>,
    deliver_qr: Option<bool>,
    deliver_link: Option<bool>,
    client_filter: Option<ClientFilter>,
}

impl Store {
    fn get_json<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.with_conn(|c| {
            c.query_row("SELECT value FROM settings WHERE key=?1", [key], |r| {
                r.get::<_, String>(0)
            })
        })
        .ok()
        .and_then(|v| serde_json::from_str(&v).ok())
    }

    fn set_json<T: serde::Serialize>(&self, key: &str, value: &T) {
        let json = serde_json::to_string(value).expect("настройки сериализуемы");
        if let Err(e) = self.with_conn(|c| {
            c.execute(
                "INSERT INTO settings(key,value) VALUES(?1,?2)
                 ON CONFLICT(key) DO UPDATE SET value=?2",
                rusqlite::params![key, json],
            )
        }) {
            tracing::error!(error = %e, key, "не удалось сохранить настройку");
        }
    }

    pub fn lang(&self, uid: i64) -> Lang {
        self.get_json(&format!("lang:{uid}")).unwrap_or_default()
    }
    pub fn has_lang(&self, uid: i64) -> bool {
        self.get_json::<Lang>(&format!("lang:{uid}")).is_some()
    }
    pub fn set_lang(&self, uid: i64, lang: Lang) {
        self.set_json(&format!("lang:{uid}"), &lang);
    }
    /// Выбранная группа для групповых админов с несколькими группами.
    /// Валидность (не удалили ли группу, не отозвали ли права) проверяет
    /// вызывающий по актуальной Role.
    pub fn current_group(&self, uid: i64) -> Option<i64> {
        self.get_json(&format!("cur_group:{uid}"))
    }
    pub fn set_current_group(&self, uid: i64, group_id: i64) {
        self.set_json(&format!("cur_group:{uid}"), &group_id);
    }
    pub fn psk_default(&self) -> bool {
        self.get_json("psk_default").unwrap_or(false)
    }
    pub fn set_psk_default(&self, v: bool) {
        self.set_json("psk_default", &v);
    }
    pub fn name_slug(&self) -> bool {
        self.get_json("name_slug").unwrap_or(false)
    }
    pub fn set_name_slug(&self, v: bool) {
        self.set_json("name_slug", &v);
    }
    pub fn deliver_conf(&self) -> bool {
        self.get_json("deliver_conf").unwrap_or(true)
    }
    pub fn set_deliver_conf(&self, v: bool) {
        self.set_json("deliver_conf", &v);
    }
    pub fn deliver_qr(&self) -> bool {
        self.get_json("deliver_qr").unwrap_or(true)
    }
    pub fn set_deliver_qr(&self, v: bool) {
        self.set_json("deliver_qr", &v);
    }
    pub fn deliver_link(&self) -> bool {
        self.get_json("deliver_link").unwrap_or(true)
    }
    pub fn set_deliver_link(&self, v: bool) {
        self.set_json("deliver_link", &v);
    }
    pub fn client_filter(&self) -> ClientFilter {
        self.get_json("client_filter").unwrap_or_default()
    }
    pub fn set_client_filter(&self, f: ClientFilter) {
        self.set_json("client_filter", &f);
    }

    /// Одноразовая миграция старого state.json. Вызывается при старте.
    /// No-op, если файла нет или в БД уже есть настройки (повторный старт).
    pub fn migrate_state_json(&self, path: &Path) {
        let has_settings: bool = self
            .with_conn(|c| c.query_row("SELECT COUNT(*) FROM settings", [], |r| r.get::<_, i64>(0)))
            .map(|n| n > 0)
            .unwrap_or(false);
        if has_settings {
            return;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            return;
        };
        let Ok(legacy) = serde_json::from_str::<LegacyState>(&text) else {
            tracing::warn!(path = %path.display(), "state.json не разобран — пропускаю миграцию");
            return;
        };
        self.set_psk_default(legacy.psk_default);
        self.set_name_slug(legacy.name_slug);
        self.set_deliver_conf(legacy.deliver_conf.unwrap_or(true));
        self.set_deliver_qr(legacy.deliver_qr.unwrap_or(true));
        self.set_deliver_link(legacy.deliver_link.unwrap_or(true));
        self.set_client_filter(legacy.client_filter.unwrap_or_default());
        for (uid, lang) in legacy.langs {
            self.set_lang(uid, lang);
        }
        if let Err(e) = std::fs::rename(path, path.with_extension("json.migrated")) {
            tracing::warn!(error = %e, "не удалось переименовать state.json после миграции");
        }
        tracing::info!("настройки мигрированы из state.json в SQLite");
    }
}

#[cfg(test)]
mod tests {
    use crate::i18n::Lang;
    use crate::store::Store;
    use crate::vpn::model::ClientFilter;

    fn store() -> Store {
        Store::open_in_memory()
    }

    #[test]
    fn defaults_when_empty() {
        let s = store();
        assert_eq!(s.lang(1), Lang::Ru);
        assert!(!s.has_lang(1));
        assert!(!s.psk_default());
    }

    #[test]
    fn per_user_lang_and_global_psk() {
        let s = store();
        s.set_lang(1, Lang::En);
        s.set_lang(2, Lang::Ru);
        s.set_psk_default(true);
        assert_eq!(s.lang(1), Lang::En);
        assert!(s.has_lang(1));
        assert_eq!(s.lang(2), Lang::Ru);
        assert_eq!(s.lang(3), Lang::Ru); // не задан → дефолт
        assert!(s.psk_default());
    }

    #[test]
    fn persists_across_reload() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("awgram.db");
        {
            let s = Store::open(&path).unwrap();
            s.set_lang(42, Lang::En);
            s.set_psk_default(true);
        }
        let s2 = Store::open(&path).unwrap();
        assert_eq!(s2.lang(42), Lang::En);
        assert!(s2.psk_default());
    }

    #[test]
    fn name_slug_default_off_toggle_and_persist() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("awgram.db");
        {
            let s = Store::open(&path).unwrap();
            assert!(!s.name_slug()); // дефолт — выключено
            s.set_name_slug(true);
            assert!(s.name_slug());
        }
        let s2 = Store::open(&path).unwrap();
        assert!(s2.name_slug()); // пережил перезагрузку
    }

    #[test]
    fn deliver_toggles_default_true() {
        let s = store();
        assert!(s.deliver_conf());
        assert!(s.deliver_qr());
        assert!(s.deliver_link());
    }

    #[test]
    fn deliver_toggles_set_and_get() {
        let s = store();
        s.set_deliver_conf(false);
        s.set_deliver_qr(false);
        s.set_deliver_link(false);
        assert!(!s.deliver_conf());
        assert!(!s.deliver_qr());
        assert!(!s.deliver_link());
    }

    #[test]
    fn deliver_toggles_persist_across_reload() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("awgram.db");
        {
            let s = Store::open(&path).unwrap();
            s.set_deliver_conf(false);
            s.set_deliver_qr(true);
            s.set_deliver_link(false);
        }
        let s2 = Store::open(&path).unwrap();
        assert!(!s2.deliver_conf());
        assert!(s2.deliver_qr());
        assert!(!s2.deliver_link());
        // старые настройки тоже пережили перезагрузку
        assert!(!s2.psk_default());
    }

    #[test]
    fn client_filter_default_is_all() {
        let s = store();
        assert_eq!(s.client_filter(), ClientFilter::All);
    }

    #[test]
    fn client_filter_set_and_get() {
        let s = store();
        s.set_client_filter(ClientFilter::Online);
        assert_eq!(s.client_filter(), ClientFilter::Online);
        s.set_client_filter(ClientFilter::Never);
        assert_eq!(s.client_filter(), ClientFilter::Never);
    }

    #[test]
    fn client_filter_persists_across_reload() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("awgram.db");
        {
            let s = Store::open(&path).unwrap();
            s.set_client_filter(ClientFilter::Offline);
        }
        let s2 = Store::open(&path).unwrap();
        assert_eq!(s2.client_filter(), ClientFilter::Offline);
    }

    // Тесты "default_..._when_missing_in_old_state" из старого хранилища
    // проверяли десериализацию произвольного state.json напрямую через
    // старую загрузку из файла. Store так JSON не читает — единственная точка
    // разбора legacy-формата это migrate_state_json, поэтому проверяем
    // те же дефолты там, на неполном файле без deliver_*/client_filter.
    #[test]
    fn migrate_state_json_deliver_defaults_true_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        std::fs::write(
            &path,
            r#"{"psk_default":true,"name_slug":false,"langs":{}}"#,
        )
        .unwrap();
        let s = store();
        s.migrate_state_json(&path);
        assert!(s.deliver_conf());
        assert!(s.deliver_qr());
        assert!(s.deliver_link());
    }

    #[test]
    fn migrate_state_json_client_filter_defaults_all_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        std::fs::write(
            &path,
            r#"{"psk_default":true,"name_slug":false,"langs":{}}"#,
        )
        .unwrap();
        let s = store();
        s.migrate_state_json(&path);
        assert_eq!(s.client_filter(), ClientFilter::All);
    }

    #[test]
    fn migrate_state_json_imports_and_renames() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state.json");
        std::fs::write(
            &state,
            // ВАЖНО: Lang сериализуется как "Ru"/"En" (derive без rename_all) —
            // ровно так писало старое JSON-хранилище настроек.
            r#"{"psk_default":true,"name_slug":true,"langs":{"42":"En"},
                "deliver_conf":false,"deliver_qr":true,"deliver_link":false,
                "client_filter":"online"}"#,
        )
        .unwrap();
        let store = Store::open_in_memory();
        store.migrate_state_json(&state);
        assert!(store.psk_default());
        assert!(store.name_slug());
        assert_eq!(store.lang(42), Lang::En);
        assert!(!store.deliver_conf());
        assert!(!store.deliver_link());
        assert_eq!(store.client_filter(), ClientFilter::Online);
        assert!(!state.exists());
        assert!(state.with_extension("json.migrated").exists());
    }

    #[test]
    fn migrate_state_json_noop_when_missing_or_settings_present() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open_in_memory();
        store.migrate_state_json(&dir.path().join("nope.json")); // нет файла — тихий no-op
        store.set_psk_default(true);
        let state = dir.path().join("state.json");
        std::fs::write(&state, r#"{"psk_default":false}"#).unwrap();
        store.migrate_state_json(&state); // настройки уже есть — не перетирать
        assert!(store.psk_default());
        assert!(state.exists()); // файл не тронут
    }

    #[test]
    fn current_group_roundtrip() {
        let s = Store::open_in_memory();
        assert_eq!(s.current_group(42), None);
        s.set_current_group(42, 7);
        assert_eq!(s.current_group(42), Some(7));
        s.set_current_group(42, 9);
        assert_eq!(s.current_group(42), Some(9));
    }
}
