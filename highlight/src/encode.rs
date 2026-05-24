use crate::{HighlightKind, HighlightToken};

fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

fn normalize_text(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                result.push('\n');
            }
            '\t' => {
                result.push_str("    ");
            }
            other => result.push(other),
        }
    }
    result
}

pub fn encode_to_compact(tokens: &[HighlightToken]) -> String {
    let mut output = String::new();
    let mut i = 0;

    while i < tokens.len() {
        let kind = tokens[i].kind;
        let text = normalize_text(&tokens[i].text);
        let mut count = utf16_len(&text);

        match kind {
            HighlightKind::Space => {
                while i + 1 < tokens.len() && tokens[i + 1].kind == HighlightKind::Space {
                    let next = normalize_text(&tokens[i + 1].text);
                    count += utf16_len(&next);
                    i += 1;
                }
                if count == 1 {
                    output.push('+');
                } else {
                    output.push_str(&count.to_string());
                    output.push('+');
                }
            }
            HighlightKind::Newline => {
                while i + 1 < tokens.len() && tokens[i + 1].kind == HighlightKind::Newline {
                    let next = normalize_text(&tokens[i + 1].text);
                    count += utf16_len(&next);
                    i += 1;
                }
                if count == 1 {
                    output.push('$');
                } else {
                    output.push_str(&count.to_string());
                    output.push('$');
                }
            }
            _ => {
                let ch = kind.to_char();
                if count == 1 {
                    output.push(ch);
                } else {
                    output.push_str(&count.to_string());
                    output.push(ch);
                }
                output.push_str(&text);
            }
        }

        i += 1;
    }

    output
}
