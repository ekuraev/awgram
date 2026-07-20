//! Типы и парсеры JSON-конвертов manage_amneziawg.sh v5.21.0.
//! Контракт аддитивный: все опциональные поля имеют #[serde(default)],
//! чтобы новые поля в будущих версиях не ломали десериализацию.

use serde::Deserialize;

// --- Общий аварийный конверт (ok=false — любая команда при провале) ---
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ErrorEnvelope {
    pub command: String,
    pub ok: bool,
    pub error: String,
    #[serde(default)]
    pub rc: i32,
}

/// Если stdout — аварийный конверт (ok=false), возвращает его. Иначе None.
/// Нужен методам Vpn для единообразного маппинга провалов любой команды.
pub fn try_error_envelope(s: &str) -> Option<ErrorEnvelope> {
    // Десериализуем в Value сначала, чтобы не падать на командо-специфичных схемах.
    let v: serde_json::Value = serde_json::from_str(s).ok()?;
    if v.get("ok").and_then(|o| o.as_bool()) == Some(false) {
        serde_json::from_value(v).ok()
    } else {
        None
    }
}

// --- add / remove / regen: общая форма «команда над списком имён» ---
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AddOut {
    #[serde(default)]
    pub ok: Option<bool>,
    #[serde(default)]
    pub added: u32,
    #[serde(default)]
    pub failed: u32,
    #[serde(default)]
    pub applied: bool,
    #[serde(default)]
    pub results: Vec<AddEntry>,
}
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AddEntry {
    pub name: String,
    pub status: AddStatus,
    #[serde(default)]
    pub conf: Option<String>,
    #[serde(default)]
    pub qr: Option<String>,
    #[serde(default)]
    pub vpnuri: Option<String>,
    #[serde(default)]
    pub expires_at: Option<i64>,
}
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AddStatus {
    Created,
    Exists,
    InvalidName,
    Error,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct RemoveOut {
    #[serde(default)]
    pub ok: Option<bool>,
    #[serde(default)]
    pub removed: u32,
    #[serde(default)]
    pub failed: u32,
    #[serde(default)]
    pub applied: bool,
    #[serde(default)]
    pub results: Vec<RemoveEntry>,
}
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct RemoveEntry {
    pub name: String,
    pub status: RemoveStatus,
}
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RemoveStatus {
    Removed,
    NotFound,
    InvalidName,
    Error,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct RegenOut {
    #[serde(default)]
    pub ok: Option<bool>,
    #[serde(default)]
    pub regenerated: u32,
    #[serde(default)]
    pub failed: u32,
    #[serde(default)]
    pub reset_routes: bool,
    #[serde(default)]
    pub results: Vec<RegenEntry>,
}
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct RegenEntry {
    pub name: String,
    pub status: RegenStatus,
    #[serde(default)]
    pub conf: Option<String>,
    #[serde(default)]
    pub qr: Option<String>,
    #[serde(default)]
    pub vpnuri: Option<String>,
}
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RegenStatus {
    Regenerated,
    NotFound,
    InvalidName,
    Error,
    #[serde(other)]
    Unknown,
}

// --- modify ---
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ModifyOut {
    #[serde(default)]
    pub ok: Option<bool>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub param: String,
    #[serde(default)]
    pub value: String,
}

// --- backup ---
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct BackupOut {
    #[serde(default)]
    pub ok: Option<bool>,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub size_bytes: Option<u64>,
}

// --- restore (конверт есть и на провале) ---
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct RestoreCounts {
    #[serde(default)]
    pub server_conf: bool,
    #[serde(default)]
    pub clients: u32,
    #[serde(default)]
    pub keys: u32,
}
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct RestoreOut {
    #[serde(default)]
    pub ok: Option<bool>,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub applied: bool,
    #[serde(default)]
    pub rolled_back: bool,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub restored: RestoreCounts,
}

// --- check: детальный отчёт ---
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct ServiceBlock {
    #[serde(default)]
    pub unit: String,
    #[serde(default)]
    pub active: bool,
}
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct InterfaceBlock {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub present: bool,
    #[serde(default)]
    pub mtu: Option<u32>,
    #[serde(default)]
    pub addresses: Vec<String>,
}
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct PortBlock {
    #[serde(default)]
    pub number: u32,
    #[serde(default)]
    pub proto: String,
    #[serde(default)]
    pub listening: bool,
}
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct ModuleBlock {
    #[serde(default)]
    pub loaded: bool,
}
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct ClientsBlock {
    #[serde(default)]
    pub total: u32,
}
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct FirewallBlock {
    #[serde(default)]
    pub ufw_active: bool,
    #[serde(default)]
    pub port_allowed: bool,
}
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct CheckReport {
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub service: ServiceBlock,
    #[serde(default)]
    pub interface: InterfaceBlock,
    #[serde(default)]
    pub port: PortBlock,
    #[serde(default)]
    pub module: ModuleBlock,
    #[serde(default)]
    pub clients: ClientsBlock,
    #[serde(default)]
    pub firewall: FirewallBlock,
}

// --- restart / repair-module ---
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct RestartOut {
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub unit: String,
    #[serde(default)]
    pub active: bool,
}
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct RepairOut {
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub module_loaded: bool,
    #[serde(default)]
    pub service_active: bool,
    #[serde(default)]
    pub rc: i32,
}

pub fn parse_add(s: &str) -> Result<AddOut, serde_json::Error> {
    serde_json::from_str(s)
}
pub fn parse_remove(s: &str) -> Result<RemoveOut, serde_json::Error> {
    serde_json::from_str(s)
}
pub fn parse_regen(s: &str) -> Result<RegenOut, serde_json::Error> {
    serde_json::from_str(s)
}
pub fn parse_modify(s: &str) -> Result<ModifyOut, serde_json::Error> {
    serde_json::from_str(s)
}
pub fn parse_backup(s: &str) -> Result<BackupOut, serde_json::Error> {
    serde_json::from_str(s)
}
pub fn parse_restore(s: &str) -> Result<RestoreOut, serde_json::Error> {
    serde_json::from_str(s)
}
pub fn parse_check(s: &str) -> Result<CheckReport, serde_json::Error> {
    serde_json::from_str(s)
}
pub fn parse_restart(s: &str) -> Result<RestartOut, serde_json::Error> {
    serde_json::from_str(s)
}
pub fn parse_repair(s: &str) -> Result<RepairOut, serde_json::Error> {
    serde_json::from_str(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ADD_OK: &str = r#"{"command":"add","ok":true,"added":1,"failed":0,"applied":true,"results":[{"name":"phone","status":"created","conf":"/root/awg/phone.conf","qr":"/root/awg/phone.png","vpnuri":"/root/awg/phone.vpnuri","expires_at":null}]}"#;
    const ADD_EXISTS: &str = r#"{"command":"add","ok":true,"added":0,"failed":1,"applied":false,"results":[{"name":"phone","status":"exists"}]}"#;
    const ERR: &str = r#"{"command":"remove","ok":false,"error":"confirmation denied","rc":1}"#;

    #[test]
    fn parse_add_success() {
        let o = parse_add(ADD_OK).unwrap();
        assert_eq!(o.ok, Some(true));
        assert_eq!(o.added, 1);
        assert!(o.applied);
        assert_eq!(o.results.len(), 1);
        let e = &o.results[0];
        assert_eq!(e.name, "phone");
        assert_eq!(e.status, AddStatus::Created);
        assert_eq!(e.conf.as_deref(), Some("/root/awg/phone.conf"));
        assert_eq!(e.qr.as_deref(), Some("/root/awg/phone.png"));
        assert_eq!(e.vpnuri.as_deref(), Some("/root/awg/phone.vpnuri"));
        assert_eq!(e.expires_at, None);
    }

    #[test]
    fn parse_add_status_exists() {
        let o = parse_add(ADD_EXISTS).unwrap();
        assert_eq!(o.results[0].status, AddStatus::Exists);
        assert_eq!(o.results[0].conf, None); // qr/conf/vpnuri опциональны
    }

    #[test]
    fn parse_add_unknown_status_not_panic() {
        let s = r#"{"ok":true,"results":[{"name":"x","status":"totally_new_status"}]}"#;
        let o = parse_add(s).unwrap();
        assert_eq!(o.results[0].status, AddStatus::Unknown);
    }

    #[test]
    fn parse_add_missing_optional_fields() {
        // Контракт аддитивный — новые поля могут отсутствовать.
        let s = r#"{"results":[{"name":"x","status":"created"}]}"#;
        let o = parse_add(s).unwrap();
        assert_eq!(o.ok, None);
        assert_eq!(o.added, 0);
        assert!(!o.applied);
        assert_eq!(o.results[0].conf, None);
    }

    #[test]
    fn try_error_envelope_detects_failure() {
        let e = try_error_envelope(ERR).unwrap();
        assert!(!e.ok);
        assert_eq!(e.command, "remove");
        assert_eq!(e.error, "confirmation denied");
        assert_eq!(e.rc, 1);
    }

    #[test]
    fn try_error_envelope_none_for_success() {
        assert!(try_error_envelope(ADD_OK).is_none());
    }

    #[test]
    fn parse_remove_not_found() {
        let s = r#"{"ok":true,"removed":0,"failed":1,"results":[{"name":"x","status":"not_found"}]}"#;
        let o = parse_remove(s).unwrap();
        assert_eq!(o.results[0].status, RemoveStatus::NotFound);
    }

    #[test]
    fn parse_regen_regenerated_with_paths() {
        let s = r#"{"ok":true,"regenerated":1,"failed":0,"reset_routes":false,"results":[{"name":"x","status":"regenerated","conf":"/a.conf","qr":null,"vpnuri":null}]}"#;
        let o = parse_regen(s).unwrap();
        assert_eq!(o.regenerated, 1);
        assert!(o.results[0].qr.is_none()); // JSON null → None
        assert_eq!(o.results[0].status, RegenStatus::Regenerated);
    }

    #[test]
    fn parse_backup_with_size() {
        let s = r#"{"ok":true,"path":"/root/awg/backups/x.tar.gz","size_bytes":123}"#;
        let o = parse_backup(s).unwrap();
        assert_eq!(o.path, "/root/awg/backups/x.tar.gz");
        assert_eq!(o.size_bytes, Some(123));
    }

    #[test]
    fn parse_backup_size_null_when_missing() {
        let s = r#"{"ok":true,"path":"/x.tar.gz","size_bytes":null}"#;
        let o = parse_backup(s).unwrap();
        assert_eq!(o.size_bytes, None);
    }

    #[test]
    fn parse_restore_success() {
        let s = r#"{"ok":true,"source":"/x.tar.gz","applied":true,"rolled_back":false,"restored":{"server_conf":true,"clients":3,"keys":5}}"#;
        let o = parse_restore(s).unwrap();
        assert!(o.applied);
        assert!(!o.rolled_back);
        assert_eq!(o.restored.clients, 3);
        assert_eq!(o.restored.keys, 5);
    }

    #[test]
    fn parse_restore_failure_with_rollback() {
        let s = r#"{"ok":false,"error":"boom","source":"/x.tar.gz","applied":false,"rolled_back":true}"#;
        let o = parse_restore(s).unwrap();
        assert!(o.rolled_back);
        assert_eq!(o.error.as_deref(), Some("boom"));
    }

    #[test]
    fn parse_check_full_report() {
        let s = r#"{"command":"check","ok":true,"service":{"unit":"awg-quick@awg0","active":true},"interface":{"name":"awg0","present":true,"mtu":1280,"addresses":["10.9.9.1/24"]},"port":{"number":39743,"proto":"udp","listening":true},"module":{"loaded":true},"clients":{"total":5},"firewall":{"ufw_active":true,"port_allowed":true}}"#;
        let r = parse_check(s).unwrap();
        assert!(r.ok);
        assert!(r.service.active);
        assert_eq!(r.service.unit, "awg-quick@awg0");
        assert!(r.interface.present);
        assert_eq!(r.interface.mtu, Some(1280));
        assert_eq!(r.interface.addresses, vec!["10.9.9.1/24"]);
        assert_eq!(r.port.number, 39743);
        assert!(r.port.listening);
        assert!(r.module.loaded);
        assert_eq!(r.clients.total, 5);
        assert!(r.firewall.ufw_active);
        assert!(r.firewall.port_allowed);
    }

    #[test]
    fn parse_check_missing_optional_fields_defaults() {
        let s = r#"{"ok":false}"#;
        let r = parse_check(s).unwrap();
        assert!(!r.ok);
        assert!(!r.service.active); // default
        assert_eq!(r.interface.mtu, None);
        assert_eq!(r.port.number, 0);
    }

    #[test]
    fn parse_modify_ok() {
        let s = r#"{"ok":true,"name":"phone","param":"PersistentKeepalive","value":"25"}"#;
        let o = parse_modify(s).unwrap();
        assert_eq!(o.name, "phone");
        assert_eq!(o.param, "PersistentKeepalive");
        assert_eq!(o.value, "25");
    }

    #[test]
    fn parse_restart_active() {
        let s = r#"{"ok":true,"unit":"awg-quick@awg0","active":true}"#;
        let o = parse_restart(s).unwrap();
        assert!(o.active);
    }

    #[test]
    fn parse_repair_codes() {
        let ok = parse_repair(r#"{"ok":true,"module_loaded":true,"service_active":true,"rc":0}"#).unwrap();
        assert_eq!(ok.rc, 0);
        let svc_down = parse_repair(r#"{"ok":false,"module_loaded":true,"service_active":false,"rc":2}"#).unwrap();
        assert_eq!(svc_down.rc, 2);
        assert!(!svc_down.service_active);
    }
}
