use regex::Regex;
use std::sync::OnceLock;

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum ValidateError {
    #[error("имя должно содержать 1–32 символа: латиница, цифры, дефис, подчёркивание")]
    BadName,
    #[error("срок должен быть в формате Nh/Nd/Nw, например 12h, 10d, 3w")]
    BadExpiry,
}

fn name_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Note: deviates from the brief's literal `^[A-Za-z0-9_-]{1,32}$` by forbidding a
    // leading hyphen. The literal pattern allows a hyphen anywhere, including first
    // position, so "--flag" would validate as a name yet be interpretable as a CLI
    // flag by the downstream script (argument injection). The brief's own test
    // `rejects_injection_and_bad_names` requires "--flag" to be rejected, so the
    // first character is restricted to alnum/underscore while the overall charset
    // and 1-32 length bound are unchanged.
    RE.get_or_init(|| Regex::new(r"^[A-Za-z0-9_][A-Za-z0-9_-]{0,31}$").unwrap())
}

fn expiry_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[0-9]{1,4}[hdw]$").unwrap())
}

pub fn validate_name(input: &str) -> Result<String, ValidateError> {
    let name = input.trim();
    if name_re().is_match(name) {
        Ok(name.to_string())
    } else {
        Err(ValidateError::BadName)
    }
}

/// Нормализация имени из диалога добавления: trim, каждая последовательность
/// пробельных символов → один дефис, опциональный слаг-префикс `{slug}-`,
/// затем та же валидация, что и в `validate_name`. Слишком длинный итог —
/// ошибка, а не молчаливая обрезка.
pub fn normalize_name(input: &str, slug: Option<&str>) -> Result<String, ValidateError> {
    let dashed = input.split_whitespace().collect::<Vec<_>>().join("-");
    if dashed.is_empty() {
        return Err(ValidateError::BadName);
    }
    let name = match slug {
        Some(s) => format!("{s}-{dashed}"),
        None => dashed,
    };
    if name_re().is_match(&name) {
        Ok(name)
    } else {
        Err(ValidateError::BadName)
    }
}

/// 5 случайных символов a-z0-9 (~60 млн комбинаций); коллизии дополнительно
/// отсекает проверка дубликатов `vpn.exists` в диалоге добавления.
pub fn gen_slug() -> String {
    use rand::Rng;
    const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::rng();
    (0..5)
        .map(|_| CHARS[rng.random_range(0..CHARS.len())] as char)
        .collect()
}

pub fn validate_expiry(input: &str) -> Result<String, ValidateError> {
    let v = input.trim();
    if expiry_re().is_match(v) {
        Ok(v.to_string())
    } else {
        Err(ValidateError::BadExpiry)
    }
}

/// Параметры клиента, которые бот умеет менять через `manage modify`.
/// CLI-имена совпадают с ключами в клиентском .conf (PersistentKeepalive/DNS/AllowedIPs/Endpoint).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModifyParam {
    Keepalive,
    Dns,
    AllowedIps,
    Endpoint,
}

pub fn modify_param_cli(p: ModifyParam) -> &'static str {
    match p {
        ModifyParam::Keepalive => "PersistentKeepalive",
        ModifyParam::Dns => "DNS",
        ModifyParam::AllowedIps => "AllowedIPs",
        ModifyParam::Endpoint => "Endpoint",
    }
}

fn keepalive_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[0-9]{1,3}$").unwrap())
}

/// 0..=600 секунд (0 = off). Буквы/знаки/вне диапазона → ошибка.
pub fn parse_keepalive(input: &str) -> Result<String, ValidateError> {
    let v = input.trim();
    if !keepalive_re().is_match(v) {
        return Err(ValidateError::BadExpiry);
    }
    match v.parse::<u32>() {
        Ok(n) if n <= 600 => Ok(n.to_string()),
        _ => Err(ValidateError::BadExpiry),
    }
}

/// 1..=4 IP-адресов (v4/v6) через запятую. Shell-метасимволы невозможны —
/// `IpAddr::from_str` их не примет.
pub fn parse_dns(input: &str) -> Result<String, ValidateError> {
    let parts: Vec<&str> = input.split(',').map(|s| s.trim()).collect();
    if parts.is_empty() || parts.len() > 4 || parts.iter().any(|s| s.is_empty()) {
        return Err(ValidateError::BadExpiry);
    }
    for p in &parts {
        if p.parse::<std::net::IpAddr>().is_err() {
            return Err(ValidateError::BadExpiry);
        }
    }
    Ok(parts.join(", "))
}

fn cidr_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // IPv4 CIDR или IPv6 CIDR. Не принимаем ничего с shell-метасимволами: в
    // шаблон не входят ; | & $ ` < > и т.д.
    RE.get_or_init(|| {
        Regex::new(r"^(?:[0-9]{1,3}(?:\.[0-9]{1,3}){3}/[0-9]{1,2}|[0-9a-fA-F:]+/[0-9]{1,3})$")
            .unwrap()
    })
}

/// CIDR-список через запятую. Синтаксическая проверка; валидность подсети
/// оставляем скрипту.
pub fn parse_allowed_ips(input: &str) -> Result<String, ValidateError> {
    let parts: Vec<&str> = input.split(',').map(|s| s.trim()).collect();
    if parts.is_empty() || parts.iter().any(|s| s.is_empty()) {
        return Err(ValidateError::BadExpiry);
    }
    for p in &parts {
        if !cidr_re().is_match(p) {
            return Err(ValidateError::BadExpiry);
        }
    }
    Ok(parts.join(", "))
}

fn endpoint_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // host:port или [ipv6]:port. host — домен или IPv4. Запрещаем shell-метасимволы.
    RE.get_or_init(|| Regex::new(r"^(?:\[?[0-9a-fA-F:.]+\]?|[A-Za-z0-9._-]+):[0-9]{1,5}$").unwrap())
}

pub fn parse_endpoint(input: &str) -> Result<String, ValidateError> {
    let v = input.trim();
    if endpoint_re().is_match(v) {
        Ok(v.to_string())
    } else {
        Err(ValidateError::BadExpiry)
    }
}

pub fn parse_modify_value(p: ModifyParam, input: &str) -> Result<String, ValidateError> {
    match p {
        ModifyParam::Keepalive => parse_keepalive(input),
        ModifyParam::Dns => parse_dns(input),
        ModifyParam::AllowedIps => parse_allowed_ips(input),
        ModifyParam::Endpoint => parse_endpoint(input),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_good_names() {
        assert_eq!(validate_name("alice").unwrap(), "alice");
        assert_eq!(validate_name("  bob_1-2  ").unwrap(), "bob_1-2");
    }

    #[test]
    fn rejects_injection_and_bad_names() {
        for bad in [
            "",
            "a b",
            "a;rm -rf /",
            "../etc",
            "имя",
            "a".repeat(33).as_str(),
            "--flag",
            "a/b",
        ] {
            assert_eq!(
                validate_name(bad),
                Err(ValidateError::BadName),
                "should reject {bad:?}"
            );
        }
    }

    #[test]
    fn accepts_good_expiry() {
        for good in ["12h", "10d", "3w", "1d", "9999h"] {
            assert!(validate_expiry(good).is_ok(), "should accept {good}");
        }
    }

    #[test]
    fn rejects_bad_expiry() {
        for bad in ["", "10", "d10", "10x", "1.5d", "10 d", "-5d", "10d;ls"] {
            assert_eq!(
                validate_expiry(bad),
                Err(ValidateError::BadExpiry),
                "should reject {bad:?}"
            );
        }
    }

    #[test]
    fn normalize_replaces_spaces_with_dashes() {
        assert_eq!(normalize_name("work laptop", None).unwrap(), "work-laptop");
        assert_eq!(
            normalize_name("work   laptop", None).unwrap(),
            "work-laptop"
        );
        assert_eq!(normalize_name("  alice  ", None).unwrap(), "alice");
    }

    #[test]
    fn normalize_adds_slug_prefix() {
        assert_eq!(
            normalize_name("alice", Some("k3x9f")).unwrap(),
            "k3x9f-alice"
        );
        assert_eq!(
            normalize_name("work laptop", Some("k3x9f")).unwrap(),
            "k3x9f-work-laptop"
        );
    }

    #[test]
    fn normalize_rejects_empty_and_whitespace_only() {
        assert_eq!(normalize_name("", None), Err(ValidateError::BadName));
        assert_eq!(normalize_name("   ", None), Err(ValidateError::BadName));
        // с включённым слагом пустое имя тоже отклоняется, а не превращается в "k3x9f-"
        assert_eq!(
            normalize_name("   ", Some("k3x9f")),
            Err(ValidateError::BadName)
        );
    }

    #[test]
    fn normalize_rejects_too_long_with_slug() {
        let name26 = "a".repeat(26);
        assert!(normalize_name(&name26, Some("k3x9f")).is_ok()); // 5+1+26 = 32
        let name27 = "a".repeat(27);
        assert_eq!(
            normalize_name(&name27, Some("k3x9f")),
            Err(ValidateError::BadName)
        );
    }

    #[test]
    fn normalize_still_rejects_injection() {
        for bad in ["a;rm -rf /", "../etc", "имя", "--flag"] {
            assert_eq!(
                normalize_name(bad, None),
                Err(ValidateError::BadName),
                "should reject {bad:?}"
            );
        }
    }

    #[test]
    fn normalize_slug_makes_leading_dash_safe() {
        // без слага "--flag" отклоняется правилом первого символа; со слагом
        // первый символ — из слага, ведущего дефиса нет, инъекция CLI-флага невозможна
        assert_eq!(
            normalize_name("--flag", Some("k3x9f")).unwrap(),
            "k3x9f---flag"
        );
    }

    #[test]
    fn gen_slug_is_5_base36_chars() {
        for _ in 0..100 {
            let s = gen_slug();
            assert_eq!(s.len(), 5);
            assert!(
                s.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()),
                "bad slug {s:?}"
            );
        }
    }

    #[test]
    fn keepalive_accepts_valid_range() {
        assert_eq!(parse_keepalive("0").unwrap(), "0");
        assert_eq!(parse_keepalive("25").unwrap(), "25");
        assert_eq!(parse_keepalive("600").unwrap(), "600");
    }

    #[test]
    fn keepalive_rejects_out_of_range_and_non_numeric() {
        for bad in ["", "abc", "-1", "601", "9999", "1.5", "25s"] {
            assert_eq!(
                parse_keepalive(bad),
                Err(ValidateError::BadExpiry),
                "should reject {bad:?}"
            );
        }
    }

    #[test]
    fn dns_accepts_ip_list() {
        assert_eq!(parse_dns("1.1.1.1").unwrap(), "1.1.1.1");
        assert_eq!(parse_dns("1.1.1.1, 8.8.8.8").unwrap(), "1.1.1.1, 8.8.8.8");
        assert!(parse_dns("2606:4700:4700::1111").is_ok());
    }

    #[test]
    fn dns_rejects_non_ip_and_too_many() {
        for bad in [
            "",
            "not-ip",
            "1.1.1.1; rm -rf /",
            "a.b.c.d",
            "1.1.1.1,",
            "8.8.8.8 1.1.1.1",
        ] {
            assert_eq!(
                parse_dns(bad),
                Err(ValidateError::BadExpiry),
                "should reject {bad:?}"
            );
        }
        // > 4 адресов
        let five = "1.1.1.1, 2.2.2.2, 3.3.3.3, 4.4.4.4, 5.5.5.5";
        assert_eq!(parse_dns(five), Err(ValidateError::BadExpiry));
    }

    #[test]
    fn allowed_ips_accepts_cidr() {
        assert!(parse_allowed_ips("0.0.0.0/0").is_ok());
        assert!(parse_allowed_ips("192.168.1.0/24, 10.0.0.0/8").is_ok());
        assert!(parse_allowed_ips("::/0").is_ok());
    }

    #[test]
    fn allowed_ips_rejects_non_cidr_and_shell_meta() {
        for bad in ["", "192.168.1.5", "not-cidr", "1.1.1.1; ls", "../etc"] {
            assert_eq!(
                parse_allowed_ips(bad),
                Err(ValidateError::BadExpiry),
                "should reject {bad:?}"
            );
        }
    }

    #[test]
    fn endpoint_accepts_host_port() {
        assert!(parse_endpoint("vpn.example.com:51820").is_ok());
        assert!(parse_endpoint("1.2.3.4:51820").is_ok());
        assert!(parse_endpoint("[2606:4700::1]:51820").is_ok());
    }

    #[test]
    fn endpoint_rejects_missing_port_and_meta() {
        for bad in ["vpn.example.com", "", ":51820", "a.b:51820; rm", "host:abc"] {
            assert_eq!(
                parse_endpoint(bad),
                Err(ValidateError::BadExpiry),
                "should reject {bad:?}"
            );
        }
    }

    #[test]
    fn modify_param_cli_names() {
        assert_eq!(
            modify_param_cli(ModifyParam::Keepalive),
            "PersistentKeepalive"
        );
        assert_eq!(modify_param_cli(ModifyParam::Dns), "DNS");
        assert_eq!(modify_param_cli(ModifyParam::AllowedIps), "AllowedIPs");
        assert_eq!(modify_param_cli(ModifyParam::Endpoint), "Endpoint");
    }

    #[test]
    fn parse_modify_value_dispatches_by_param() {
        assert!(parse_modify_value(ModifyParam::Keepalive, "25").is_ok());
        assert!(parse_modify_value(ModifyParam::Dns, "1.1.1.1").is_ok());
        assert!(parse_modify_value(ModifyParam::Keepalive, "abc").is_err());
    }
}
