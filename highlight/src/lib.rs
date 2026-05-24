mod decode;
mod encode;
mod extract;

pub use decode::decode_from_compact;
pub use encode::encode_to_compact;
pub use extract::{extract_tokens, extract_tokens_from_source};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HighlightKind {
    Keyword,
    Type,
    Function,
    Variable,
    StringLit,
    Number,
    Comment,
    Lifetime,
    Operator,
    Punctuation,
    Attribute,
    Macro,
    Default,
    Space,
    Newline,
}

impl HighlightKind {
    pub fn to_char(self) -> char {
        match self {
            Self::Keyword => 'k',
            Self::Type => 't',
            Self::Function => 'f',
            Self::Variable => 'v',
            Self::Macro => 'm',
            Self::StringLit => 's',
            Self::Number => 'n',
            Self::Comment => 'c',
            Self::Lifetime => 'l',
            Self::Operator => 'o',
            Self::Punctuation => 'p',
            Self::Attribute => 'a',
            Self::Default => '_',
            Self::Space => '+',
            Self::Newline => '$',
        }
    }

    pub fn from_char(c: char) -> Option<Self> {
        match c {
            'k' => Some(Self::Keyword),
            't' => Some(Self::Type),
            'f' => Some(Self::Function),
            'v' => Some(Self::Variable),
            'm' => Some(Self::Macro),
            's' => Some(Self::StringLit),
            'n' => Some(Self::Number),
            'c' => Some(Self::Comment),
            'l' => Some(Self::Lifetime),
            'o' => Some(Self::Operator),
            'p' => Some(Self::Punctuation),
            'a' => Some(Self::Attribute),
            '_' => Some(Self::Default),
            '+' => Some(Self::Space),
            '$' => Some(Self::Newline),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightToken {
    pub text: String,
    pub kind: HighlightKind,
}

impl HighlightToken {
    pub fn new(text: impl Into<String>, kind: HighlightKind) -> Self {
        Self {
            text: text.into(),
            kind,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_to_string(tokens: &[HighlightToken]) -> String {
        let mut s = String::new();
        for t in tokens {
            match t.kind {
                HighlightKind::Space => s.push_str(&" ".repeat(t.text.encode_utf16().count())),
                HighlightKind::Newline => s.push_str(&"\n".repeat(t.text.encode_utf16().count())),
                _ => s.push_str(&t.text),
            }
        }
        s
    }

    #[test]
    fn test_roundtrip_simple() {
        let source = "pub fn main() {\n    let a = 0;\n}\n";
        let tokens = extract_tokens_from_source(source);
        let encoded = encode_to_compact(&tokens);
        let decoded = decode_from_compact(&encoded);
        let restored = decode_to_string(&decoded);
        assert_eq!(restored, source);
    }

    #[test]
    fn test_roundtrip_no_trailing_newline() {
        let source = "pub fn main() { }";
        let tokens = extract_tokens_from_source(source);
        let encoded = encode_to_compact(&tokens);
        let decoded = decode_from_compact(&encoded);
        let restored = decode_to_string(&decoded);
        assert_eq!(restored, source);
    }

    #[test]
    fn test_roundtrip_struct() {
        let source = "#[derive(Debug)]\npub struct Foo {\n    x: i32,\n}\n";
        let tokens = extract_tokens_from_source(source);
        let encoded = encode_to_compact(&tokens);
        let decoded = decode_from_compact(&encoded);
        let restored = decode_to_string(&decoded);
        assert_eq!(restored, source);
    }

    #[test]
    fn test_roundtrip_tabs_and_crlf() {
        let source = "pub fn main() {\r\n\tlet a = 0;\r\n}\r\n";
        let tokens = extract_tokens_from_source(source);
        let encoded = encode_to_compact(&tokens);
        let decoded = decode_from_compact(&encoded);
        let restored = decode_to_string(&decoded);
        let expected = "pub fn main() {\n    let a = 0;\n}\n";
        assert_eq!(restored, expected);
    }

    #[test]
    fn test_encode_decode_direct() {
        let tokens = vec![
            HighlightToken::new("pub", HighlightKind::Keyword),
            HighlightToken::new(" ", HighlightKind::Space),
            HighlightToken::new("fn", HighlightKind::Keyword),
            HighlightToken::new(" ", HighlightKind::Space),
            HighlightToken::new("main", HighlightKind::Function),
            HighlightToken::new("(", HighlightKind::Punctuation),
            HighlightToken::new(")", HighlightKind::Punctuation),
            HighlightToken::new(" ", HighlightKind::Space),
            HighlightToken::new("{", HighlightKind::Punctuation),
            HighlightToken::new("\n", HighlightKind::Newline),
            HighlightToken::new("    ", HighlightKind::Space),
            HighlightToken::new("let", HighlightKind::Keyword),
            HighlightToken::new(" ", HighlightKind::Space),
            HighlightToken::new("a", HighlightKind::Variable),
            HighlightToken::new(" ", HighlightKind::Space),
            HighlightToken::new("=", HighlightKind::Operator),
            HighlightToken::new(" ", HighlightKind::Space),
            HighlightToken::new("0", HighlightKind::Number),
            HighlightToken::new(";", HighlightKind::Punctuation),
            HighlightToken::new("\n", HighlightKind::Newline),
            HighlightToken::new("}", HighlightKind::Punctuation),
            HighlightToken::new("\n", HighlightKind::Newline),
        ];
        let expected = "3kpub+2kfn+4fmainp(p)+p{$4+3klet+va+o=+n0p;$p}$";
        let encoded = encode_to_compact(&tokens);
        assert_eq!(encoded, expected);
        let decoded = decode_from_compact(&encoded);
        assert_eq!(decoded, tokens);
    }

    #[test]
    fn test_empty() {
        assert_eq!(encode_to_compact(&[]), "");
        assert_eq!(decode_from_compact(""), vec![] as Vec<HighlightToken>);
    }

    #[test]
    fn test_kind_to_from_char_roundtrip() {
        for kind in &[
            HighlightKind::Keyword,
            HighlightKind::Type,
            HighlightKind::Function,
            HighlightKind::Variable,
            HighlightKind::Macro,
            HighlightKind::StringLit,
            HighlightKind::Number,
            HighlightKind::Comment,
            HighlightKind::Lifetime,
            HighlightKind::Operator,
            HighlightKind::Punctuation,
            HighlightKind::Attribute,
            HighlightKind::Default,
            HighlightKind::Space,
            HighlightKind::Newline,
        ] {
            let c = kind.to_char();
            let back = HighlightKind::from_char(c).unwrap();
            assert_eq!(*kind, back);
        }
    }

    #[test]
    fn test_decode_known_compact() {
        let encoded = "3kpub+2kfn+4fmainp(p)+p{$4+3klet+va+o=+n0p;$p}$";
        let decoded = decode_from_compact(encoded);
        let restored = decode_to_string(&decoded);
        assert_eq!(restored, "pub fn main() {\n    let a = 0;\n}\n");
    }

    #[test]
    fn test_utf16_complex_chars() {
        let tokens = vec![HighlightToken::new("🦀", HighlightKind::Variable)];
        let encoded = encode_to_compact(&tokens);
        assert_eq!(encoded, "2v🦀");
        let decoded = decode_from_compact(&encoded);
        assert_eq!(decoded, tokens);
    }

    #[test]
    fn test_consecutive_spaces_compressed() {
        let tokens = vec![
            HighlightToken::new(" ", HighlightKind::Space),
            HighlightToken::new("  ", HighlightKind::Space),
            HighlightToken::new("   ", HighlightKind::Space),
        ];
        let encoded = encode_to_compact(&tokens);
        assert_eq!(encoded, "6+");
        let decoded = decode_from_compact(&encoded);
        assert_eq!(
            decoded,
            vec![HighlightToken::new("      ", HighlightKind::Space),]
        );
    }
}
