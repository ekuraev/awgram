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
}
