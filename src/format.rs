//! Formatting of expanded macro output.
//!
//! Macro expansion works on token trees, and token trees do not carry any
//! whitespace, so expanded code comes out as a single dense line. This
//! module re-formats that output into a readable, rustfmt-like layout by
//! walking the syntax tree of the expanded code and re-inserting line breaks
//! and indentation at the places the grammar knows about.

use ra_ap_syntax::ast::AstNode;
use ra_ap_syntax::{
    Edition, SourceFile, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken, WalkEvent,
};

const INDENT: &str = "    ";

/// Formats Rust source code, re-indenting it and breaking lines at
/// statement, field and item boundaries.
pub fn format_code(code: &str) -> String {
    let trimmed = code.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let parsed = SourceFile::parse(trimmed, Edition::CURRENT);
    let mut formatter = Formatter::default();
    formatter.walk(parsed.tree().syntax());

    let formatted = formatter.finish();
    if formatted.trim().is_empty() {
        trimmed.to_string()
    } else {
        formatted
    }
}

/// Containers whose members are written one per line.
fn is_block_context(node: &SyntaxNode) -> bool {
    match node.kind() {
        // A block, a struct body, an enum body, a match, a module, ...
        SyntaxKind::STMT_LIST
        | SyntaxKind::RECORD_FIELD_LIST
        | SyntaxKind::VARIANT_LIST
        | SyntaxKind::MATCH_ARM_LIST
        | SyntaxKind::ITEM_LIST
        | SyntaxKind::ASSOC_ITEM_LIST
        | SyntaxKind::MACRO_ITEMS => true,
        // A macro body is only broken up when it is delimited by braces.
        SyntaxKind::TOKEN_TREE => node
            .children_with_tokens()
            .filter_map(|element| element.into_token())
            .find(|token| !token.kind().is_trivia())
            .is_some_and(|token| token.kind() == SyntaxKind::L_CURLY),
        _ => false,
    }
}

/// Containers that hold one item per line.
fn is_item_list(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::SOURCE_FILE
            | SyntaxKind::ITEM_LIST
            | SyntaxKind::ASSOC_ITEM_LIST
            | SyntaxKind::MACRO_ITEMS
    )
}

/// Nodes that start a new item.
fn is_item(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::FN
            | SyntaxKind::STRUCT
            | SyntaxKind::ENUM
            | SyntaxKind::UNION
            | SyntaxKind::TRAIT
            | SyntaxKind::IMPL
            | SyntaxKind::MODULE
            | SyntaxKind::USE
            | SyntaxKind::TYPE_ALIAS
            | SyntaxKind::CONST
            | SyntaxKind::STATIC
            | SyntaxKind::MACRO_RULES
            | SyntaxKind::MACRO_DEF
            | SyntaxKind::EXTERN_CRATE
            | SyntaxKind::EXTERN_BLOCK
    )
}

/// Operators that are surrounded by spaces.
fn is_spaced_operator(text: &str) -> bool {
    matches!(
        text,
        "+" | "-"
            | "*"
            | "/"
            | "%"
            | "="
            | "=="
            | "!="
            | "<"
            | ">"
            | "<="
            | ">="
            | "&&"
            | "||"
            | "|"
            | "^"
            | "+="
            | "-="
            | "*="
            | "/="
            | "%="
            | "<<="
            | ">>="
            | "&="
            | "|="
            | "^="
            | "->"
            | "=>"
    )
}

fn is_literal(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::INT_NUMBER
            | SyntaxKind::FLOAT_NUMBER
            | SyntaxKind::STRING
            | SyntaxKind::BYTE_STRING
            | SyntaxKind::C_STRING
            | SyntaxKind::CHAR
    )
}

/// True when the token sits inside an attribute, e.g. `#[cfg(unix)]`.
fn in_attribute(token: &SyntaxToken) -> bool {
    token.parent().is_some_and(|parent| {
        parent
            .ancestors()
            .any(|node| node.kind() == SyntaxKind::ATTR)
    })
}

/// True when the token sits inside a visibility modifier, e.g. `pub(crate)`.
fn in_visibility(token: &SyntaxToken) -> bool {
    token.parent().is_some_and(|parent| {
        parent
            .ancestors()
            .any(|node| node.kind() == SyntaxKind::VISIBILITY)
    })
}

#[derive(Default)]
struct Formatter {
    out: String,
    indent: usize,
    /// The next token starts on a new line.
    pending_break: bool,
    /// The next token is separated by a space.
    pending_space: bool,
    /// The next token starts a new item.
    pending_item: bool,
    last: Option<SyntaxKind>,
}

impl Formatter {
    fn walk(&mut self, node: &SyntaxNode) {
        for event in node.preorder_with_tokens() {
            match event {
                // Tokens are reported as `Enter` events of a token element.
                // Whitespace and comments carry no layout information here,
                // the formatter inserts its own.
                WalkEvent::Enter(SyntaxElement::Token(token)) => {
                    if !token.kind().is_trivia() {
                        self.token(&token);
                    }
                }
                WalkEvent::Enter(SyntaxElement::Node(node)) => self.enter_node(&node),
                // Every attribute goes on its own line.
                WalkEvent::Leave(SyntaxElement::Node(node)) => {
                    if node.kind() == SyntaxKind::ATTR {
                        self.pending_break = true;
                    }
                }
                _ => {}
            }
        }
    }

    fn finish(self) -> String {
        self.out
    }

    /// Items start on their own line.
    fn enter_node(&mut self, node: &SyntaxNode) {
        if node.kind() == SyntaxKind::ATTR {
            // Attributes are written on their own line.
            if !self.out.is_empty() {
                self.pending_break = true;
            }
            return;
        }

        if !is_item(node.kind()) {
            return;
        }
        if node
            .parent()
            .is_some_and(|parent| is_item_list(parent.kind()))
        {
            self.pending_item = true;
        }
    }

    fn token(&mut self, token: &SyntaxToken) {
        let kind = token.kind();
        let text = token.text().to_string();
        let parent = token.parent();
        let block = parent.as_ref().is_some_and(is_block_context);

        if self.pending_item {
            self.pending_item = false;
            if !self.out.is_empty() {
                self.pending_break = true;
            }
        }

        match kind {
            SyntaxKind::L_CURLY => {
                // `path::{A, B}` keeps the brace attached to the path.
                let space = !matches!(self.last, Some(SyntaxKind::COLON2));
                self.push(&text, space);
                if block {
                    self.indent += 1;
                    self.pending_break = true;
                }
            }
            SyntaxKind::R_CURLY => {
                if block {
                    self.indent = self.indent.saturating_sub(1);
                    if self.out.ends_with('{') {
                        // An empty block stays on a single line.
                        self.pending_break = false;
                    } else {
                        self.pending_break = true;
                    }
                }
                self.push(&text, false);
            }
            SyntaxKind::SEMICOLON => {
                self.push(&text, false);
                if block {
                    self.pending_break = true;
                } else {
                    self.pending_space = true;
                }
            }
            SyntaxKind::COMMA => {
                self.push(&text, false);
                if block {
                    self.pending_break = true;
                } else {
                    self.pending_space = true;
                }
            }
            SyntaxKind::COLON => {
                // `name: Type` keeps a space after the colon, `::` does not.
                self.push(&text, false);
                self.pending_space = true;
            }
            _ => {
                let space = self.space_before(token, &text);
                self.push(&text, space);
                if is_spaced_operator(&text) {
                    self.pending_space = true;
                }
            }
        }

        self.last = Some(kind);
    }

    /// Decides whether the token is separated from the previous one.
    fn space_before(&self, token: &SyntaxToken, text: &str) -> bool {
        let kind = token.kind();
        let parent_kind = token.parent().map(|parent| parent.kind());
        let last = self.last;
        let last_is_word = last.is_some_and(|kind| {
            kind == SyntaxKind::IDENT
                || is_literal(kind)
                || kind.is_keyword(Edition::CURRENT)
        });

        // `pub(crate)` is written without any space.
        if in_visibility(token) {
            return false;
        }

        // Inside an attribute only the `=` of a key/value pair is spaced.
        if in_attribute(token) {
            return text == "=";
        }

        // Tokens that never have a space in front of them.
        match kind {
            SyntaxKind::R_PAREN
            | SyntaxKind::R_BRACK
            | SyntaxKind::R_CURLY
            | SyntaxKind::COLON
            | SyntaxKind::SEMICOLON
            | SyntaxKind::COMMA
            | SyntaxKind::DOT2
            | SyntaxKind::DOT2EQ
            | SyntaxKind::DOT
            | SyntaxKind::COLON2
            | SyntaxKind::QUESTION
            | SyntaxKind::POUND
            | SyntaxKind::AT
            | SyntaxKind::R_ANGLE => return false,
            _ => {}
        }

        // A path segment or a generic argument list binds tightly.
        if matches!(kind, SyntaxKind::L_ANGLE | SyntaxKind::R_ANGLE)
            && matches!(
                parent_kind,
                Some(SyntaxKind::GENERIC_ARG_LIST)
                    | Some(SyntaxKind::PATH_SEGMENT)
                    | Some(SyntaxKind::PATH_TYPE)
            )
        {
            return false;
        }

        // References, pointers and closures bind tightly as well.
        match (kind, parent_kind) {
            (SyntaxKind::AMP, Some(SyntaxKind::REF_EXPR)) => return false,
            (SyntaxKind::STAR, Some(SyntaxKind::PTR_TYPE)) => return false,
            (SyntaxKind::STAR, Some(SyntaxKind::PREFIX_EXPR)) => return false,
            (SyntaxKind::BANG, Some(SyntaxKind::MACRO_CALL)) => return false,
            (SyntaxKind::PIPE, Some(SyntaxKind::CLOSURE_EXPR)) => return false,
            _ => {}
        }

        // `&` and `*` are binary operators in expressions, references in
        // types; the parent node tells them apart.
        if matches!(kind, SyntaxKind::AMP | SyntaxKind::STAR)
            && parent_kind == Some(SyntaxKind::BIN_EXPR)
        {
            return true;
        }

        // A macro invocation binds to its name, a negation does not.
        if kind == SyntaxKind::BANG {
            return true;
        }

        if is_spaced_operator(text) {
            return true;
        }

        // Two words in a row always need a separator.
        if last_is_word && (kind == SyntaxKind::IDENT || kind.is_keyword(Edition::CURRENT)) {
            return true;
        }

        // A word followed by a literal, and the other way round.
        if last_is_word && is_literal(kind) {
            return true;
        }

        // `foo(` is a call, `my_macro!(` is a macro call, `if (` is not.
        if matches!(kind, SyntaxKind::L_PAREN | SyntaxKind::L_BRACK) {
            return !matches!(
                last,
                Some(SyntaxKind::IDENT) | Some(SyntaxKind::BANG)
            );
        }

        // A keyword never touches the token in front of it.
        if kind.is_keyword(Edition::CURRENT) {
            return true;
        }

        false
    }

    fn push(&mut self, text: &str, space: bool) {
        if self.pending_break {
            // `} else {` stays on a single line.
            if text == "else" {
                self.pending_break = false;
                self.pending_space = true;
            } else {
                self.out.push('\n');
                for _ in 0..self.indent {
                    self.out.push_str(INDENT);
                }
                self.pending_break = false;
                self.pending_space = false;
            }
        } else if (space || self.pending_space) && !self.out.is_empty() && !self.out.ends_with(' ') {
            self.out.push(' ');
        }
        self.pending_space = false;

        self.out.push_str(text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_a_function_body() {
        let formatted = format_code("pub fn generated_fn () -> u32 {42}");
        assert_eq!(formatted, "pub fn generated_fn() -> u32 {\n    42\n}");
    }

    #[test]
    fn formats_a_struct() {
        let formatted = format_code("pub struct Foo {pub a : u32 ,pub b : u32 ,}");
        assert_eq!(
            formatted,
            "pub struct Foo {\n    pub a: u32,\n    pub b: u32,\n}"
        );
    }

    #[test]
    fn formats_an_expression() {
        assert_eq!(format_code("21 + 21"), "21 + 21");
    }

    #[test]
    fn formats_several_items() {
        let formatted = format_code("pub fn a () {} pub fn b () {}");
        assert_eq!(formatted, "pub fn a() {}\npub fn b() {}");
    }

    #[test]
    fn formats_a_macro_call() {
        let formatted = format_code("my_macro ! (1 , 2)");
        assert_eq!(formatted, "my_macro!(1, 2)");
    }

    #[test]
    fn formats_attributes() {
        let formatted = format_code("#[ cfg (feature = \"macros\")] mod select ;");
        assert_eq!(formatted, "#[cfg(feature = \"macros\")]\nmod select;");
    }

    #[test]
    fn formats_visibility() {
        let formatted = format_code("pub ( crate) use x :: y ;");
        assert_eq!(formatted, "pub(crate) use x::y;");
    }
}
