use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Client {
    pub name: String,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub rx_bytes: u64,
    #[serde(default)]
    pub tx_bytes: u64,
    #[serde(default)]
    pub last_handshake: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AddResult {
    pub name: String,
    pub conf_path: String,
    pub qr_path: String,
    pub uri: String,
}

pub fn parse_client_list(json: &str) -> Result<Vec<Client>, serde_json::Error> {
    serde_json::from_str(json)
}

pub fn parse_add_result(json: &str) -> Result<AddResult, serde_json::Error> {
    serde_json::from_str(json)
}

pub fn human_bytes(n: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    if n < 1024 {
        return format!("{n} B");
    }
    let mut value = n as f64;
    let mut unit = 0;
    // Advance while the value ROUNDED to 1 decimal is still >= 1024 in this unit.
    while ((value * 10.0).round() / 10.0) >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST_JSON: &str = r#"[
      {"name":"alice","active":true,"expires_at":"2026-08-01","rx_bytes":1288490188,"tx_bytes":356515840,"last_handshake":"2026-07-10T10:00:00Z"},
      {"name":"bob","active":false}
    ]"#;

    const ADD_JSON: &str = r#"{"name":"carol","conf_path":"/root/awg/carol.conf","qr_path":"/root/awg/carol.png","uri":"vpn://example"}"#;

    #[test]
    fn parses_client_list() {
        let clients = parse_client_list(LIST_JSON).unwrap();
        assert_eq!(clients.len(), 2);
        assert_eq!(clients[0].name, "alice");
        assert!(clients[0].active);
        assert_eq!(clients[0].rx_bytes, 1288490188);
        assert_eq!(clients[1].name, "bob");
        assert!(!clients[1].active);
        assert_eq!(clients[1].rx_bytes, 0);
    }

    #[test]
    fn parses_add_result() {
        let r = parse_add_result(ADD_JSON).unwrap();
        assert_eq!(r.name, "carol");
        assert_eq!(r.conf_path, "/root/awg/carol.conf");
        assert_eq!(r.qr_path, "/root/awg/carol.png");
    }

    #[test]
    fn human_bytes_formats() {
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(512), "512 B");
        assert_eq!(human_bytes(1536), "1.5 KB");
        assert_eq!(human_bytes(1288490188), "1.2 GB");
        assert_eq!(human_bytes(1048526), "1.0 MB");
        assert_eq!(human_bytes(1073741823), "1.0 GB");
        assert_eq!(human_bytes(1048576), "1.0 MB");
    }
}
