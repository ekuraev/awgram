use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::i18n::Lang;
use crate::vpn::model::ClientFilter;

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotState {
    #[serde(default)]
    pub psk_default: bool,
    #[serde(default)]
    pub name_slug: bool,
    #[serde(default)]
    pub langs: HashMap<i64, Lang>,
    #[serde(default = "default_true")]
    pub deliver_conf: bool,
    #[serde(default = "default_true")]
    pub deliver_qr: bool,
    #[serde(default = "default_true")]
    pub deliver_link: bool,
    /// Фильтр списка клиентов по статусу (персистентный, как name_slug/deliver_*).
    #[serde(default)]
    pub client_filter: ClientFilter,
}

impl Default for BotState {
    fn default() -> Self {
        BotState {
            psk_default: false,
            name_slug: false,
            langs: HashMap::new(),
            deliver_conf: true,
            deliver_qr: true,
            deliver_link: true,
            client_filter: ClientFilter::default(),
        }
    }
}

pub struct SettingsStore {
    path: PathBuf,
    state: Mutex<BotState>,
}

impl SettingsStore {
    pub fn load(path: PathBuf) -> Self {
        let state = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<BotState>(&s).ok())
            .unwrap_or_default();
        SettingsStore {
            path,
            state: Mutex::new(state),
        }
    }

    fn persist(&self, state: &BotState) {
        let tmp = self.path.with_extension("json.tmp");
        match serde_json::to_string_pretty(state) {
            Ok(json) => {
                if std::fs::write(&tmp, json)
                    .and_then(|_| std::fs::rename(&tmp, &self.path))
                    .is_err()
                {
                    tracing::error!(path = %self.path.display(), "не удалось сохранить state.json");
                }
            }
            Err(e) => tracing::error!(error = %e, "сериализация state.json"),
        }
    }

    pub fn lang(&self, uid: i64) -> Lang {
        self.state
            .lock()
            .unwrap()
            .langs
            .get(&uid)
            .copied()
            .unwrap_or_default()
    }

    pub fn has_lang(&self, uid: i64) -> bool {
        self.state.lock().unwrap().langs.contains_key(&uid)
    }

    pub fn set_lang(&self, uid: i64, lang: Lang) {
        let mut s = self.state.lock().unwrap();
        s.langs.insert(uid, lang);
        let snapshot = s.clone();
        drop(s);
        self.persist(&snapshot);
    }

    pub fn psk_default(&self) -> bool {
        self.state.lock().unwrap().psk_default
    }

    pub fn set_psk_default(&self, v: bool) {
        let mut s = self.state.lock().unwrap();
        s.psk_default = v;
        let snapshot = s.clone();
        drop(s);
        self.persist(&snapshot);
    }

    pub fn name_slug(&self) -> bool {
        self.state.lock().unwrap().name_slug
    }

    pub fn set_name_slug(&self, v: bool) {
        let mut s = self.state.lock().unwrap();
        s.name_slug = v;
        let snapshot = s.clone();
        drop(s);
        self.persist(&snapshot);
    }

    pub fn deliver_conf(&self) -> bool {
        self.state.lock().unwrap().deliver_conf
    }

    pub fn set_deliver_conf(&self, v: bool) {
        let mut s = self.state.lock().unwrap();
        s.deliver_conf = v;
        let snapshot = s.clone();
        drop(s);
        self.persist(&snapshot);
    }

    pub fn deliver_qr(&self) -> bool {
        self.state.lock().unwrap().deliver_qr
    }

    pub fn set_deliver_qr(&self, v: bool) {
        let mut s = self.state.lock().unwrap();
        s.deliver_qr = v;
        let snapshot = s.clone();
        drop(s);
        self.persist(&snapshot);
    }

    pub fn deliver_link(&self) -> bool {
        self.state.lock().unwrap().deliver_link
    }

    pub fn set_deliver_link(&self, v: bool) {
        let mut s = self.state.lock().unwrap();
        s.deliver_link = v;
        let snapshot = s.clone();
        drop(s);
        self.persist(&snapshot);
    }

    pub fn client_filter(&self) -> ClientFilter {
        self.state.lock().unwrap().client_filter
    }

    pub fn set_client_filter(&self, f: ClientFilter) {
        let mut s = self.state.lock().unwrap();
        s.client_filter = f;
        let snapshot = s.clone();
        drop(s);
        self.persist(&snapshot);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, SettingsStore) {
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::load(dir.path().join("state.json"));
        (dir, store)
    }

    #[test]
    fn defaults_when_empty() {
        let (_d, s) = store();
        assert_eq!(s.lang(1), Lang::Ru);
        assert!(!s.has_lang(1));
        assert!(!s.psk_default());
    }

    #[test]
    fn per_user_lang_and_global_psk() {
        let (_d, s) = store();
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
        let path = dir.path().join("state.json");
        {
            let s = SettingsStore::load(path.clone());
            s.set_lang(42, Lang::En);
            s.set_psk_default(true);
        }
        let s2 = SettingsStore::load(path);
        assert_eq!(s2.lang(42), Lang::En);
        assert!(s2.psk_default());
    }

    #[test]
    fn name_slug_default_off_toggle_and_persist() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        {
            let s = SettingsStore::load(path.clone());
            assert!(!s.name_slug()); // дефолт — выключено
            s.set_name_slug(true);
            assert!(s.name_slug());
        }
        let s2 = SettingsStore::load(path);
        assert!(s2.name_slug()); // пережил перезагрузку
    }

    #[test]
    fn deliver_toggles_default_true() {
        let (_d, s) = store();
        assert!(s.deliver_conf());
        assert!(s.deliver_qr());
        assert!(s.deliver_link());
    }

    #[test]
    fn deliver_toggles_set_and_get() {
        let (_d, s) = store();
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
        let path = dir.path().join("state.json");
        {
            let s = SettingsStore::load(path.clone());
            s.set_deliver_conf(false);
            s.set_deliver_qr(true);
            s.set_deliver_link(false);
        }
        let s2 = SettingsStore::load(path);
        assert!(!s2.deliver_conf());
        assert!(s2.deliver_qr());
        assert!(!s2.deliver_link());
        // старые настройки тоже пережили перезагрузку
        assert!(!s2.psk_default());
    }

    #[test]
    fn deliver_toggles_default_true_when_missing_in_old_state() {
        // Старый state.json без полей deliver_* должен десериализоваться с true.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        std::fs::write(
            &path,
            r#"{"psk_default":true,"name_slug":false,"langs":{}}"#,
        )
        .unwrap();
        let s = SettingsStore::load(path);
        assert!(s.deliver_conf());
        assert!(s.deliver_qr());
        assert!(s.deliver_link());
    }

    #[test]
    fn client_filter_default_is_all() {
        let (_d, s) = store();
        assert_eq!(s.client_filter(), ClientFilter::All);
    }

    #[test]
    fn client_filter_set_and_get() {
        let (_d, s) = store();
        s.set_client_filter(ClientFilter::Online);
        assert_eq!(s.client_filter(), ClientFilter::Online);
        s.set_client_filter(ClientFilter::Never);
        assert_eq!(s.client_filter(), ClientFilter::Never);
    }

    #[test]
    fn client_filter_persists_across_reload() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        {
            let s = SettingsStore::load(path.clone());
            s.set_client_filter(ClientFilter::Offline);
        }
        let s2 = SettingsStore::load(path);
        assert_eq!(s2.client_filter(), ClientFilter::Offline);
    }

    #[test]
    fn client_filter_default_all_when_missing_in_old_state() {
        // Старый state.json без client_filter должен десериализоваться как All.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        std::fs::write(
            &path,
            r#"{"psk_default":true,"name_slug":false,"langs":{}}"#,
        )
        .unwrap();
        let s = SettingsStore::load(path);
        assert_eq!(s.client_filter(), ClientFilter::All);
    }
}
