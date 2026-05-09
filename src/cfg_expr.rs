use ra_ap_cfg::CfgAtom;
use ra_ap_cfg::CfgExpr as RaCfgExpr;
use ra_ap_syntax::ast::{self, AstNode};
use ra_ap_syntax::{SyntaxKind, SyntaxNode};

use crate::model::CfgExpr;

// Parses all #[cfg(...)] and #[cfg_attr(...)] attributes from a syntax node
// using the rust-analyzer cfg library instead of hand-written parsing.
// This delegates to ra_ap_cfg::CfgExpr::parse_from_ast which handles
// nested all/any/not combinators and proper token-level key-value extraction.
pub fn parse_cfg_from_node(node: &SyntaxNode) -> CfgExpr {
    let mut exprs = Vec::new();

    for child in node.children() {
        if child.kind() != SyntaxKind::ATTR {
            continue;
        }

        let Some(attr) = ast::Attr::cast(child) else {
            continue;
        };

        let Some(meta) = attr.meta() else {
            continue;
        };

        match meta {
            // #[cfg(predicate)] — parse the predicate via RA's AST-aware parser
            ast::Meta::CfgMeta(cfg_meta) => {
                if let Some(pred) = cfg_meta.cfg_predicate() {
                    exprs.push(ra_to_model(RaCfgExpr::parse_from_ast(pred)));
                }
            }
            // #[cfg_attr(condition, attr)] — the condition ANDes with nested cfg if present
            ast::Meta::CfgAttrMeta(cfg_attr_meta) => {
                if let Some(cond) = cfg_attr_meta.cfg_predicate() {
                    let mut combined = ra_to_model(RaCfgExpr::parse_from_ast(cond));

                    // If the inner attr is also cfg(...), combine both conditions so
                    // the resulting expression reflects both the cfg_attr condition
                    // and the nested cfg predicate
                    for inner_meta in cfg_attr_meta.metas() {
                        if let ast::Meta::CfgMeta(inner_cfg) = inner_meta {
                            if let Some(inner_pred) = inner_cfg.cfg_predicate() {
                                let inner_expr =
                                    ra_to_model(RaCfgExpr::parse_from_ast(inner_pred));
                                combined = and_cfg(combined, inner_expr);
                            }
                        }
                    }

                    exprs.push(combined);
                }
            }
            _ => {}
        }
    }

    // Multiple cfg attributes on the same item are equivalent to AND
    if exprs.is_empty() {
        CfgExpr::True
    } else if exprs.len() == 1 {
        exprs.remove(0)
    } else {
        CfgExpr::All(exprs)
    }
}

// Converts ra_ap_cfg's CfgExpr into the project's own model IR.
// This decouples the serialization format from the RA library types.
fn ra_to_model(expr: RaCfgExpr) -> CfgExpr {
    match expr {
        // Invalid cfg expressions are treated as True so the item remains visible;
        // a malformed cfg should not silently hide an item.
        RaCfgExpr::Invalid => CfgExpr::True,
        RaCfgExpr::Atom(atom) => match atom {
            CfgAtom::Flag(name) => CfgExpr::Flag(name.to_string()),
            CfgAtom::KeyValue { key, value } => {
                // Feature-specific keys get their own variant for cleaner downstream filtering
                if key.as_str() == "feature" {
                    CfgExpr::Feature(value.to_string())
                } else {
                    CfgExpr::Flag(format!("{} = \"{}\"", key, value))
                }
            }
        },
        RaCfgExpr::All(exprs) => {
            let converted: Vec<CfgExpr> = exprs.into_vec().into_iter().map(ra_to_model).collect();
            CfgExpr::All(converted)
        }
        RaCfgExpr::Any(exprs) => {
            let converted: Vec<CfgExpr> = exprs.into_vec().into_iter().map(ra_to_model).collect();
            CfgExpr::Any(converted)
        }
        RaCfgExpr::Not(inner) => CfgExpr::Not(Box::new(ra_to_model(*inner))),
    }
}

// Combines two cfg expressions with AND, flattening nested All variants
// to avoid deep nesting like All(All(a, b), c) vs All(a, b, c).
pub fn and_cfg(a: CfgExpr, b: CfgExpr) -> CfgExpr {
    match (a, b) {
        (CfgExpr::True, rhs) => rhs,
        (lhs, CfgExpr::True) => lhs,
        (CfgExpr::All(mut lhs), CfgExpr::All(rhs)) => {
            lhs.extend(rhs);
            CfgExpr::All(lhs)
        }
        (CfgExpr::All(mut lhs), rhs) => {
            lhs.push(rhs);
            CfgExpr::All(lhs)
        }
        (lhs, CfgExpr::All(mut rhs)) => {
            let mut out = vec![lhs];
            out.append(&mut rhs);
            CfgExpr::All(out)
        }
        (lhs, rhs) => CfgExpr::All(vec![lhs, rhs]),
    }
}
