use std::sync::OnceLock;
use regex::Regex;

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
        for bad in ["", "a b", "a;rm -rf /", "../etc", "имя", "a".repeat(33).as_str(), "--flag", "a/b"] {
            assert_eq!(validate_name(bad), Err(ValidateError::BadName), "should reject {bad:?}");
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
            assert_eq!(validate_expiry(bad), Err(ValidateError::BadExpiry), "should reject {bad:?}");
        }
    }
}
