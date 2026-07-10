use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Lang {
    #[default]
    Ru,
    En,
}

pub fn parse_lang(code: &str) -> Option<Lang> {
    match code {
        "ru" => Some(Lang::Ru),
        "en" => Some(Lang::En),
        _ => None,
    }
}

pub fn lang_code(l: Lang) -> &'static str {
    match l {
        Lang::Ru => "ru",
        Lang::En => "en",
    }
}

pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_html_specials() {
        assert_eq!(html_escape("a<b>&c"), "a&lt;b&gt;&amp;c");
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("plain"), "plain");
    }

    #[test]
    fn amp_escaped_first() {
        // & должен экранироваться до < и >, иначе получим двойное экранирование
        assert_eq!(html_escape("<"), "&lt;");
        assert!(!html_escape("a & b").contains("&amp;amp;"));
    }

    #[test]
    fn lang_roundtrip() {
        assert_eq!(parse_lang("ru"), Some(Lang::Ru));
        assert_eq!(parse_lang("en"), Some(Lang::En));
        assert_eq!(parse_lang("xx"), None);
        assert_eq!(lang_code(Lang::Ru), "ru");
        assert_eq!(lang_code(Lang::En), "en");
        assert_eq!(Lang::default(), Lang::Ru);
    }
}
