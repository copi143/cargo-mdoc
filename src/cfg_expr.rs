use crate::model::CfgExpr;

pub fn parse_cfg_attrs(attrs: &[String]) -> CfgExpr {
    let mut exprs = Vec::new();

    for attr in attrs {
        if attr.starts_with("#[cfg(") {
            let content = attr
                .trim_start_matches("#[cfg(")
                .trim_end_matches(")]")
                .trim();

            exprs.push(parse_cfg_expr(content));
        } else if attr.starts_with("#[cfg_attr(") {
            if let Some(cfg_expr) = parse_cfg_attr(attr) {
                exprs.push(cfg_expr);
            }
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

pub fn parse_cfg_expr(input: &str) -> CfgExpr {
    let input = input.trim();

    if input.starts_with("feature") {
        if let Some(start) = input.find('"') {
            if let Some(end) = input.rfind('"') {
                return CfgExpr::Feature(input[start + 1..end].to_string());
            }
        }
    }

    if input.starts_with("all(") {
        let inner = input.trim_start_matches("all(").trim_end_matches(')');
        return CfgExpr::All(
            split_cfg_args(inner)
                .into_iter()
                .map(|x| parse_cfg_expr(&x))
                .collect(),
        );
    }

    if input.starts_with("any(") {
        let inner = input.trim_start_matches("any(").trim_end_matches(')');
        return CfgExpr::Any(
            split_cfg_args(inner)
                .into_iter()
                .map(|x| parse_cfg_expr(&x))
                .collect(),
        );
    }

    if input.starts_with("not(") {
        let inner = input.trim_start_matches("not(").trim_end_matches(')');
        return CfgExpr::Not(Box::new(parse_cfg_expr(inner)));
    }

    CfgExpr::Flag(input.to_string())
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

fn parse_cfg_attr(attr: &str) -> Option<CfgExpr> {
    let inner = attr.strip_prefix("#[cfg_attr(")?.strip_suffix(")]")?.trim();

    let split_at = split_top_level_comma(inner)?;
    let condition = inner[..split_at].trim();
    let nested = inner[split_at + 1..].trim();

    if nested.starts_with("cfg(") {
        let nested_cfg = nested
            .trim_start_matches("cfg(")
            .trim_end_matches(')')
            .trim();

        return Some(and_cfg(
            parse_cfg_expr(condition),
            parse_cfg_expr(nested_cfg),
        ));
    }

    Some(parse_cfg_expr(condition))
}

fn split_top_level_comma(input: &str) -> Option<usize> {
    let mut depth = 0;

    for (index, ch) in input.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            ',' if depth == 0 => return Some(index),
            _ => {}
        }
    }

    None
}

fn split_cfg_args(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0;
    let mut current = String::new();

    for c in input.chars() {
        match c {
            '(' => {
                depth += 1;
                current.push(c);
            }
            ')' => {
                depth -= 1;
                current.push(c);
            }
            ',' if depth == 0 => {
                out.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(c),
        }
    }

    if !current.trim().is_empty() {
        out.push(current.trim().to_string());
    }

    out
}
