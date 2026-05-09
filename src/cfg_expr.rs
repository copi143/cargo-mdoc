use ra_ap_cfg::CfgAtom;
use ra_ap_cfg::CfgExpr as RaCfgExpr;
use ra_ap_syntax::ast::{self, AstNode};
use ra_ap_syntax::{SyntaxKind, SyntaxNode};

use crate::model::CfgExpr;

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
            ast::Meta::CfgMeta(cfg_meta) => {
                if let Some(pred) = cfg_meta.cfg_predicate() {
                    exprs.push(ra_to_model(RaCfgExpr::parse_from_ast(pred)));
                }
            }
            ast::Meta::CfgAttrMeta(cfg_attr_meta) => {
                if let Some(cond) = cfg_attr_meta.cfg_predicate() {
                    let mut combined = ra_to_model(RaCfgExpr::parse_from_ast(cond));

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

    if exprs.is_empty() {
        CfgExpr::True
    } else if exprs.len() == 1 {
        exprs.remove(0)
    } else {
        CfgExpr::All(exprs)
    }
}

fn ra_to_model(expr: RaCfgExpr) -> CfgExpr {
    match expr {
        RaCfgExpr::Invalid => CfgExpr::True,
        RaCfgExpr::Atom(atom) => match atom {
            CfgAtom::Flag(name) => CfgExpr::Flag(name.to_string()),
            CfgAtom::KeyValue { key, value } => {
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
