use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::i18n::Lang;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct BotState {
    #[serde(default)]
    pub psk_default: bool,
    #[serde(default)]
    pub langs: HashMap<i64, Lang>,
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
        SettingsStore { path, state: Mutex::new(state) }
    }

    fn persist(&self, state: &BotState) {
        let tmp = self.path.with_extension("json.tmp");
        match serde_json::to_string_pretty(state) {
            Ok(json) => {
                if std::fs::write(&tmp, json).and_then(|_| std::fs::rename(&tmp, &self.path)).is_err() {
                    tracing::error!(path = %self.path.display(), "не удалось сохранить state.json");
                }
            }
            Err(e) => tracing::error!(error = %e, "сериализация state.json"),
        }
    }

    pub fn lang(&self, uid: i64) -> Lang {
        self.state.lock().unwrap().langs.get(&uid).copied().unwrap_or_default()
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
}
