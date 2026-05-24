use ra_ap_syntax::ast::AstNode;
use ra_ap_syntax::{Edition, NodeOrToken, SourceFile, SyntaxKind, SyntaxNode};

use crate::{HighlightKind, HighlightToken};

fn map_syntax_kind(kind: SyntaxKind, text: &str) -> HighlightKind {
    match kind {
        SyntaxKind::AS_KW
        | SyntaxKind::ASYNC_KW
        | SyntaxKind::AWAIT_KW
        | SyntaxKind::BOX_KW
        | SyntaxKind::BREAK_KW
        | SyntaxKind::CONST_KW
        | SyntaxKind::CONTINUE_KW
        | SyntaxKind::CRATE_KW
        | SyntaxKind::DYN_KW
        | SyntaxKind::ELSE_KW
        | SyntaxKind::ENUM_KW
        | SyntaxKind::EXTERN_KW
        | SyntaxKind::FALSE_KW
        | SyntaxKind::FN_KW
        | SyntaxKind::FOR_KW
        | SyntaxKind::IF_KW
        | SyntaxKind::IMPL_KW
        | SyntaxKind::IN_KW
        | SyntaxKind::LET_KW
        | SyntaxKind::LOOP_KW
        | SyntaxKind::MACRO_KW
        | SyntaxKind::MATCH_KW
        | SyntaxKind::MOD_KW
        | SyntaxKind::MOVE_KW
        | SyntaxKind::MUT_KW
        | SyntaxKind::PUB_KW
        | SyntaxKind::REF_KW
        | SyntaxKind::RETURN_KW
        | SyntaxKind::SELF_KW
        | SyntaxKind::STATIC_KW
        | SyntaxKind::STRUCT_KW
        | SyntaxKind::SUPER_KW
        | SyntaxKind::TRAIT_KW
        | SyntaxKind::TRUE_KW
        | SyntaxKind::TRY_KW
        | SyntaxKind::TYPE_KW
        | SyntaxKind::UNION_KW
        | SyntaxKind::UNSAFE_KW
        | SyntaxKind::USE_KW
        | SyntaxKind::WHERE_KW
        | SyntaxKind::WHILE_KW
        | SyntaxKind::YIELD_KW
        | SyntaxKind::ABSTRACT_KW
        | SyntaxKind::BECOME_KW
        | SyntaxKind::DO_KW
        | SyntaxKind::FINAL_KW
        | SyntaxKind::OVERRIDE_KW
        | SyntaxKind::PRIV_KW
        | SyntaxKind::TYPEOF_KW
        | SyntaxKind::UNSIZED_KW
        | SyntaxKind::VIRTUAL_KW
        | SyntaxKind::SAFE_KW
        | SyntaxKind::RAW_KW
        | SyntaxKind::GEN_KW
        | SyntaxKind::YEET_KW
        | SyntaxKind::DEFAULT_KW
        | SyntaxKind::SELF_TYPE_KW
        | SyntaxKind::ASM_KW
        | SyntaxKind::ATT_SYNTAX_KW
        | SyntaxKind::AUTO_KW
        | SyntaxKind::BIKESHED_KW
        | SyntaxKind::BUILTIN_KW
        | SyntaxKind::CFG_ATTR_KW
        | SyntaxKind::CFG_KW
        | SyntaxKind::CLOBBER_ABI_KW
        | SyntaxKind::DEREF_KW
        | SyntaxKind::FORMAT_ARGS_KW
        | SyntaxKind::GLOBAL_ASM_KW
        | SyntaxKind::INLATEOUT_KW
        | SyntaxKind::INOUT_KW
        | SyntaxKind::IS_KW
        | SyntaxKind::LABEL_KW
        | SyntaxKind::LATEOUT_KW
        | SyntaxKind::MACRO_RULES_KW
        | SyntaxKind::MAY_UNWIND_KW
        | SyntaxKind::NAKED_ASM_KW
        | SyntaxKind::NOMEM_KW
        | SyntaxKind::NORETURN_KW
        | SyntaxKind::NOSTACK_KW
        | SyntaxKind::NULL_KW
        | SyntaxKind::OFFSET_OF_KW
        | SyntaxKind::OPTIONS_KW
        | SyntaxKind::OUT_KW
        | SyntaxKind::PATTERN_TYPE_KW
        | SyntaxKind::PRESERVES_FLAGS_KW
        | SyntaxKind::PURE_KW
        | SyntaxKind::READONLY_KW
        | SyntaxKind::SYM_KW => HighlightKind::Keyword,

        SyntaxKind::L_PAREN
        | SyntaxKind::R_PAREN
        | SyntaxKind::L_CURLY
        | SyntaxKind::R_CURLY
        | SyntaxKind::L_BRACK
        | SyntaxKind::R_BRACK
        | SyntaxKind::SEMICOLON
        | SyntaxKind::COMMA
        | SyntaxKind::COLON
        | SyntaxKind::AT
        | SyntaxKind::POUND
        | SyntaxKind::UNDERSCORE
        | SyntaxKind::DOT => HighlightKind::Punctuation,

        SyntaxKind::PLUS
        | SyntaxKind::MINUS
        | SyntaxKind::STAR
        | SyntaxKind::SLASH
        | SyntaxKind::AMP
        | SyntaxKind::PIPE
        | SyntaxKind::CARET
        | SyntaxKind::PERCENT
        | SyntaxKind::TILDE
        | SyntaxKind::EQ
        | SyntaxKind::EQ2
        | SyntaxKind::NEQ
        | SyntaxKind::BANG
        | SyntaxKind::L_ANGLE
        | SyntaxKind::R_ANGLE
        | SyntaxKind::DOT2
        | SyntaxKind::DOT3
        | SyntaxKind::DOT2EQ
        | SyntaxKind::COLON2
        | SyntaxKind::FAT_ARROW
        | SyntaxKind::THIN_ARROW
        | SyntaxKind::LTEQ
        | SyntaxKind::GTEQ
        | SyntaxKind::PLUSEQ
        | SyntaxKind::MINUSEQ
        | SyntaxKind::PIPEEQ
        | SyntaxKind::AMPEQ
        | SyntaxKind::CARETEQ
        | SyntaxKind::SLASHEQ
        | SyntaxKind::STAREQ
        | SyntaxKind::PERCENTEQ
        | SyntaxKind::AMP2
        | SyntaxKind::PIPE2
        | SyntaxKind::SHL
        | SyntaxKind::SHR
        | SyntaxKind::SHLEQ
        | SyntaxKind::SHREQ
        | SyntaxKind::QUESTION
        | SyntaxKind::DOLLAR => HighlightKind::Operator,

        SyntaxKind::STRING
        | SyntaxKind::BYTE_STRING
        | SyntaxKind::C_STRING
        | SyntaxKind::BYTE
        | SyntaxKind::CHAR => HighlightKind::StringLit,

        SyntaxKind::INT_NUMBER | SyntaxKind::FLOAT_NUMBER => HighlightKind::Number,

        SyntaxKind::COMMENT => HighlightKind::Comment,

        SyntaxKind::LIFETIME_IDENT => HighlightKind::Lifetime,

        SyntaxKind::IDENT => {
            if text.starts_with(|c: char| c.is_uppercase()) {
                HighlightKind::Type
            } else {
                HighlightKind::Variable
            }
        }

        SyntaxKind::SHEBANG | SyntaxKind::FRONTMATTER | SyntaxKind::ERROR => HighlightKind::Default,

        _ => HighlightKind::Default,
    }
}

fn classify_whitespace(text: &str) -> Vec<HighlightToken> {
    let chars: Vec<char> = text.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '\n' || chars[i] == '\r' {
            if chars[i] == '\r' && i + 1 < chars.len() && chars[i + 1] == '\n' {
                i += 2;
            } else {
                i += 1;
            }
            tokens.push(HighlightToken::new("\n", HighlightKind::Newline));
        } else {
            let start = i;
            while i < chars.len() && chars[i] != '\n' && chars[i] != '\r' {
                i += 1;
            }
            tokens.push(HighlightToken::new(
                chars[start..i].iter().collect::<String>(),
                HighlightKind::Space,
            ));
        }
    }

    tokens
}

pub fn extract_tokens(node: &SyntaxNode) -> Vec<HighlightToken> {
    let mut tokens = Vec::new();
    for element in node.descendants_with_tokens() {
        if let NodeOrToken::Token(token) = element {
            let kind = token.kind();
            let text = token.text().to_string();

            if kind == SyntaxKind::WHITESPACE || kind == SyntaxKind::NEWLINE {
                tokens.extend(classify_whitespace(&text));
            } else {
                let hl = map_syntax_kind(kind, &text);
                tokens.push(HighlightToken::new(text, hl));
            }
        }
    }
    tokens
}

pub fn extract_tokens_from_source(source: &str) -> Vec<HighlightToken> {
    let parsed = SourceFile::parse(source, Edition::CURRENT);
    let node = parsed.tree().syntax().clone();
    extract_tokens(&node)
}
