//! Support for expanding `macro_rules!` invocations.
//!
//! The matching and transcription itself is done by rust-analyzer's
//! macro-by-example engine (`ra_ap_mbe`). That engine expects a `salsa`
//! database, but it only uses it to resolve the edition of a syntax
//! context, and the root context resolves its edition without touching the
//! database, so a minimal database is enough to drive it.
//!
//! Expansion is purely token based. Hygiene rules and macro definitions
//! that live in other crates are not modelled: the result shows *what* a
//! macro produces, it is not meant to reproduce rustc's exact output.

use ra_ap_mbe::{DeclarativeMacro, MacroCallStyle};
use ra_ap_span::{
    Edition, EditionedFileId, FileId, Span, SpanAnchor, SyntaxContext, ROOT_ERASED_FILE_AST_ID,
};
use ra_ap_syntax::ast::{self, AstNode, HasName};
use ra_ap_syntax::{Edition as SyntaxEdition, SourceFile, SyntaxKind, TextRange};
use ra_ap_syntax_bridge::{syntax_node_to_token_tree, DocCommentDesugarMode, SpanMapper};
use ra_ap_tt::TopSubtree;

use crate::model::ItemKind;
use crate::util::{build_signature, collect_attrs, extract_docs, source_snippet};

/// File id used for the synthetic spans of expanded tokens.
const DUMMY_FILE_ID: u32 = 0xe4e4e;

/// A minimal salsa database.
///
/// `ra_ap_mbe` only queries the database for the edition of a syntax
/// context, and the root context resolves the edition without touching the
/// database at all, so an empty database is sufficient.
type ExpandDb = salsa::DatabaseImpl;

/// Maps every token to the same root syntax context.
///
/// Expanded tokens carry no meaningful position: they are a synthetic
/// result, not a slice of the original source file.
struct RootSpanMap;

impl SpanMapper for RootSpanMap {
    fn span_for(&self, range: TextRange) -> Span {
        Span {
            range,
            anchor: SpanAnchor {
                file_id: EditionedFileId::new(FileId::from_raw(DUMMY_FILE_ID), Edition::CURRENT),
                ast_id: ROOT_ERASED_FILE_AST_ID,
            },
            ctx: SyntaxContext::root(Edition::CURRENT),
        }
    }
}

/// An item produced by a macro expansion.
#[derive(Debug, Clone)]
pub struct ExpandedItem {
    pub name: String,
    pub kind: ItemKind,
    pub docs: String,
    pub signature: Option<String>,
    pub source_code: String,
}

/// The outcome of expanding a single macro call.
#[derive(Debug, Clone)]
pub struct MacroExpansion {
    /// The expanded token stream, rendered as Rust source text.
    pub code: String,
    /// Index of the macro rule that matched, when the expansion succeeded.
    pub matched_arm: Option<u32>,
    /// Description of the failure, when the expansion was only partial.
    pub error: Option<String>,
    /// Items the expansion produced.
    pub items: Vec<ExpandedItem>,
}

/// Expands a `macro_rules!` invocation against its definition.
///
/// Returns `None` when the macro cannot be expanded at all, e.g. because
/// the definition or the call has no token tree.
pub fn expand_macro_call(
    def: &ast::MacroRules,
    call: &ast::MacroCall,
) -> Option<MacroExpansion> {
    let def_tt = token_tree(def.token_tree()?)?;
    let call_tt = token_tree(call.token_tree()?)?;

    let db = ExpandDb::new();
    let rules = DeclarativeMacro::parse_macro_rules(&def_tt, |_ctx| Edition::CURRENT);

    if let Some(err) = rules.err() {
        return Some(MacroExpansion {
            code: String::new(),
            matched_arm: None,
            error: Some(err.to_string()),
            items: Vec::new(),
        });
    }

    let result = rules.expand(
        &db,
        &call_tt,
        |_span| {},
        MacroCallStyle::FnLike,
        dummy_span(),
    );
    let (expanded, matched_arm) = result.value;
    let code = crate::format::format_code(&expanded.to_string());

    Some(MacroExpansion {
        items: extract_items(&code),
        code,
        matched_arm,
        error: result.err.map(|err| err.to_string()),
    })
}

/// Converts a syntax token tree into the token tree format used by
/// rust-analyzer's macro engine.
fn token_tree(tree: ast::TokenTree) -> Option<TopSubtree> {
    Some(syntax_node_to_token_tree(
        tree.syntax(),
        &RootSpanMap,
        dummy_span(),
        DocCommentDesugarMode::Mbe,
    ))
}

fn dummy_span() -> Span {
    Span {
        range: ra_ap_span::TextRange::empty(ra_ap_span::TextSize::new(0)),
        anchor: SpanAnchor {
            file_id: EditionedFileId::new(FileId::from_raw(DUMMY_FILE_ID), Edition::CURRENT),
            ast_id: ROOT_ERASED_FILE_AST_ID,
        },
        ctx: SyntaxContext::root(Edition::CURRENT),
    }
}

/// Parses expanded text and collects the items it defines.
///
/// Macro expansions frequently expand to expressions rather than items, in
/// which case no item is reported.
fn extract_items(code: &str) -> Vec<ExpandedItem> {
    let parsed = SourceFile::parse(code, SyntaxEdition::CURRENT);
    let mut items = Vec::new();

    for child in parsed.tree().syntax().children() {
        let (name, kind, docs) = match child.kind() {
            SyntaxKind::FN => {
                let Some(func) = ast::Fn::cast(child.clone()) else {
                    continue;
                };
                (func.name(), ItemKind::Function, extract_docs(&func))
            }
            SyntaxKind::STRUCT => {
                let Some(st) = ast::Struct::cast(child.clone()) else {
                    continue;
                };
                (st.name(), ItemKind::Struct, extract_docs(&st))
            }
            SyntaxKind::ENUM => {
                let Some(en) = ast::Enum::cast(child.clone()) else {
                    continue;
                };
                (en.name(), ItemKind::Enum, extract_docs(&en))
            }
            SyntaxKind::TRAIT => {
                let Some(tr) = ast::Trait::cast(child.clone()) else {
                    continue;
                };
                (tr.name(), ItemKind::Trait, extract_docs(&tr))
            }
            _ => continue,
        };

        let Some(name) = name else { continue };
        let signature = build_signature(&child, &kind);

        items.push(ExpandedItem {
            name: name.text().to_string(),
            kind,
            docs,
            signature,
            source_code: source_snippet(&child),
        });
    }

    items
}

/// Collects the attributes of a macro call site.
pub fn call_attributes(call: &ast::MacroCall) -> Vec<String> {
    collect_attrs(call.syntax())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expand(def_src: &str, call_src: &str) -> Option<MacroExpansion> {
        let def_file = SourceFile::parse(def_src, SyntaxEdition::CURRENT);
        let call_file = SourceFile::parse(call_src, SyntaxEdition::CURRENT);

        let def = def_file
            .tree()
            .syntax()
            .children()
            .find_map(ast::MacroRules::cast)?;
        let call = call_file
            .tree()
            .syntax()
            .children()
            .find_map(ast::MacroCall::cast)?;

        expand_macro_call(&def, &call)
    }

    #[test]
    fn expands_single_metavariable() {
        let expansion = expand(
            "macro_rules! double { ($x:expr) => { $x + $x }; }",
            "double!(21);",
        )
        .expect("expansion should be produced");

        assert!(expansion.error.is_none(), "{:?}", expansion.error);
        assert!(
            expansion.code.contains("21 + 21"),
            "unexpected expansion: {}",
            expansion.code
        );
    }

    #[test]
    fn expands_repetition() {
        let expansion = expand(
            "macro_rules! sum { ($($x:expr),* $(,)?) => { 0 $(+ $x)* }; }",
            "sum!(1, 2, 3);",
        )
        .expect("expansion should be produced");

        assert!(expansion.error.is_none(), "{:?}", expansion.error);
        assert!(
            expansion.code.contains("0 + 1 + 2 + 3"),
            "unexpected expansion: {}",
            expansion.code
        );
    }

    #[test]
    fn reports_items_of_an_item_expansion() {
        let expansion = expand(
            "macro_rules! make_fn { () => { pub fn generated() {} }; }",
            "make_fn!();",
        )
        .expect("expansion should be produced");

        assert_eq!(expansion.items.len(), 1);
        assert_eq!(expansion.items[0].name, "generated");
        assert!(matches!(expansion.items[0].kind, ItemKind::Function));
    }

    #[test]
    fn keeps_error_of_a_failing_expansion() {
        let expansion = expand(
            "macro_rules! needs_two { ($a:expr, $b:expr) => { $a + $b }; }",
            "needs_two!(1);",
        )
        .expect("expansion should be produced");

        assert!(expansion.error.is_some());
    }
}
