use crate::{HighlightKind, HighlightToken};

pub fn decode_from_compact(encoded: &str) -> Vec<HighlightToken> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = encoded.chars().collect();
    let len = chars.len();
    let mut pos = 0;

    while pos < len {
        let mut count: usize = 0;
        let mut has_digits = false;
        while pos < len && chars[pos].is_ascii_digit() {
            count = count * 10 + (chars[pos] as u8 - b'0') as usize;
            pos += 1;
            has_digits = true;
        }
        if !has_digits {
            count = 1;
        }

        if pos >= len {
            break;
        }

        let kind_char = chars[pos];
        pos += 1;

        match kind_char {
            '+' => {
                tokens.push(HighlightToken::new(" ".repeat(count), HighlightKind::Space));
            }
            '$' => {
                tokens.push(HighlightToken::new(
                    "\n".repeat(count),
                    HighlightKind::Newline,
                ));
            }
            _ => {
                let mut text = String::with_capacity(count);
                let mut remaining = count;
                while remaining > 0 && pos < len {
                    let c = chars[pos];
                    let uc = c.len_utf16();
                    if uc <= remaining {
                        text.push(c);
                        remaining -= uc;
                        pos += 1;
                    } else {
                        break;
                    }
                }
                let kind = HighlightKind::from_char(kind_char).unwrap_or(HighlightKind::Default);
                tokens.push(HighlightToken::new(text, kind));
            }
        }
    }

    tokens
}
