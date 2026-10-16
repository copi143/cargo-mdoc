use crate::model::{Deprecation, ItemKind, PackageId, SourceLocation, Visibility};
use ra_ap_syntax::ast::{HasAttrs, HasGenericParams, HasName, HasTypeBounds, HasVisibility};
use ra_ap_syntax::{AstNode as _, SyntaxKind, SyntaxNode, ast};
use std::path::{Path, PathBuf};

// Collects all direct child ATTR nodes of a syntax node as raw text strings.
// Only direct children are collected because attributes on an item are
// immediate children of that item's syntax node in the RA AST.
pub fn collect_attrs(node: &SyntaxNode) -> Vec<String> {
    let mut out = Vec::new();

    for child in node.children() {
        if child.kind() == SyntaxKind::ATTR {
            out.push(child.text().to_string());
        }
    }

    out
}

// Extracts doc text from any syntax node that may carry doc comments.
// Uses ra_ap_syntax's HasAttrs trait, which properly handles
// ///, //!, /** ... */, and #[doc = "..."] forms including escaped quotes.
pub fn extract_docs<N: HasAttrs>(node: &N) -> String {
    node.doc_comments()
        .map(|comment| comment.text().to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

// Resolves a Rust module declaration to its file path.
// Rust 2018+ module resolution searches multiple paths:
//   1. <parent>/<mod_name>.rs        (direct file)
//   2. <parent>/<mod_name>/mod.rs     (directory module)
// Additionally, we support:
//   3. <parent>/<stem>/<mod_name>.rs (sibling directory, for mod foo in bar.rs)
//   4. <parent>/<stem>/<mod_name>/mod.rs
// This is intentionally more permissive than rustc's resolution to handle
// non-standard project layouts commonly found in real-world crates.
pub fn resolve_module_file(current_file: &Path, mod_name: &str) -> Option<PathBuf> {
    let parent = current_file.parent()?;
    let stem = current_file.file_stem()?.to_str()?;

    let direct = parent.join(format!("{}.rs", mod_name));
    let nested = parent.join(mod_name).join("mod.rs");
    let sibling = parent.join(stem).join(format!("{}.rs", mod_name));
    let sibling_nested = parent.join(stem).join(mod_name).join("mod.rs");

    if direct.exists() {
        Some(direct)
    } else if sibling.exists() {
        Some(sibling)
    } else if nested.exists() {
        Some(nested)
    } else if sibling_nested.exists() {
        Some(sibling_nested)
    } else {
        None
    }
}

pub fn join_path(path: &[String], item: &str) -> String {
    if path.is_empty() {
        item.to_string()
    } else {
        format!("{}::{}", path.join("::"), item)
    }
}

pub fn module_path_string(path: &[String]) -> String {
    path.join("::")
}

// Visibility is determined by the presence and content of the pub keyword.
// None means no visibility modifier = crate-private (default).
// "pub" alone means fully public; any other form (pub(crate), pub(super), etc.)
// is grouped as Restricted without preserving the exact path.
pub fn visibility_from_ast(vis: Option<ast::Visibility>) -> Visibility {
    match vis {
        None => Visibility::Crate,
        Some(visibility) => {
            let text = visibility.syntax().text().to_string();
            if text == "pub" {
                Visibility::Public
            } else {
                Visibility::Restricted
            }
        }
    }
}

pub fn source_location(
    workspace_root: &Path,
    current_file: &Path,
    source: &str,
    node: &SyntaxNode,
) -> SourceLocation {
    let range = node.text_range();
    let start: usize = range.start().into();
    let end: usize = range.end().into();

    let start_line = line_number_at(source, start);
    let end_line = line_number_at(source, end.saturating_sub(1).max(start));

    let file = current_file
        .strip_prefix(workspace_root)
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|_| current_file.to_string_lossy().to_string());

    SourceLocation {
        file,
        line_start: start_line,
        line_end: end_line,
        col_start: None,
        col_end: None,
    }
}

// Reconstructs source code from the AST by walking children_with_tokens,
// skipping doc comments (///, //!, /** , /*!, #[doc = ...]) since they
// are already presented in the separate `docs` field.
//
// Token text is emitted as-is to preserve original formatting, then
// dedented consistently so all items share the same base indentation.
pub fn source_snippet(node: &SyntaxNode) -> String {
    let mut raw = String::new();

    for child in node.children_with_tokens() {
        if is_doc_element(&child) {
            continue;
        }
        raw.push_str(&child.to_string());
    }

    let lines: Vec<&str> = raw.lines().collect();
    if lines.is_empty() {
        return String::new();
    }

    // Find the minimum indentation across non-empty lines,
    // skipping the first line (signature) since it often has no leading
    // whitespace and would otherwise force common_indent to 0,
    // preventing dedent of the indented body lines below it.
    let common_indent = lines
        .iter()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);

    // Strip the common indentation from every line,
    // then collapse runs of empty lines to at most one.
    // The first line (signature) is trimmed of all leading whitespace
    // since it carries no relative indentation.
    let mut prev_empty = false;
    let mut result: String = lines
        .iter()
        .enumerate()
        .filter_map(|(i, line)| {
            let stripped = if i == 0 {
                line.trim_start()
            } else if line.len() >= common_indent {
                &line[common_indent..]
            } else {
                line.trim_start()
            };
            let empty = stripped.trim().is_empty();
            if empty && prev_empty {
                None
            } else {
                prev_empty = empty;
                Some(stripped)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Drop leading/trailing blank lines left behind when doc comments at the
    // very start of an item are removed; the docs themselves live in `docs`.
    result = result.trim_start().trim_end().to_string();
    if result.is_empty() {
        String::new()
    } else {
        result
    }
}

// Checks whether a SyntaxElement (node or token) is a doc comment.
fn is_doc_element(element: &ra_ap_syntax::SyntaxElement) -> bool {
    let text = element.to_string();
    let kind = element.kind();
    if kind == SyntaxKind::ATTR {
        text.trim_start().starts_with("#[doc")
    } else if kind == SyntaxKind::COMMENT {
        let trimmed = text.trim_start();
        !(trimmed.starts_with("////") || trimmed.starts_with("/***"))
            && (trimmed.starts_with("///")
                || trimmed.starts_with("//!")
                || trimmed.starts_with("/**")
                || trimmed.starts_with("/*!"))
    } else if kind == SyntaxKind::DOC_COMMENT {
        true
    } else {
        false
    }
}

// Extract the string representation of a type from an AST type node.
// Handles simple path types (MyStruct), generics (Vec<u8>), etc.
// Used for resolving impl target types like `impl Foo for Bar`.
pub fn type_path_from_ast_type(ty: &ast::Type) -> String {
    ty.syntax().text().to_string().trim().to_string()
}

// Counts newlines up to a byte offset to compute 1-indexed line numbers
pub fn line_number_at(source: &str, byte_offset: usize) -> usize {
    source
        .as_bytes()
        .iter()
        .take(byte_offset.min(source.len()))
        .filter(|&&byte| byte == b'\n')
        .count()
        + 1
}

// Computes a relative path from base to path, falling back to the original
// path if it's not under base (e.g., for external dependencies)
pub fn rel_path(base: &Path, path: &Path) -> String {
    path.strip_prefix(base)
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string_lossy().to_string())
}

// Creates a globally unique PackageId from name, version, source, and manifest path.
// The format is: pkg:<name>@<version>|<source_key>|<manifest_rel_path>
// This is stable across runs and unique across workspace + dependency packages.
pub fn package_id_for(
    name: &str,
    version: &str,
    source_key: &str,
    manifest_path: &str,
) -> PackageId {
    format!("pkg:{}@{}|{}|{}", name, version, source_key, manifest_path)
}

// ===== Deprecated extraction =====

/// Extracts deprecation info from item attributes.
/// Strictly follows Rust grammar for #[deprecated] attribute forms:
///   #[deprecated]
///   #[deprecated = "note"]
///   #[deprecated(since = "1.0.0", note = "use X")]
///   #[deprecated(note = "use X")]
pub fn extract_deprecated(attrs: &[String]) -> Option<Deprecation> {
    for attr in attrs {
        let trimmed = attr.trim();
        if !trimmed.starts_with("#[deprecated") {
            continue;
        }

        // #[deprecated] — no parens, no key-value
        if trimmed == "#[deprecated]" || trimmed == "#[deprecated]()" {
            return Some(Deprecation { since: None, note: None });
        }

        // #[deprecated = "note"] — shorthand for note
        if let Some(rest) = trimmed.strip_prefix("#[deprecated = ") {
            let note = rest.trim_end_matches(']').trim();
            let note = strip_quotes(note);
            return Some(Deprecation {
                since: None,
                note: Some(note.to_string()),
            });
        }

        // #[deprecated(...)] — parenthesized key-value list
        if let Some(inner) = extract_paren_content(trimmed) {
            return Some(parse_deprecated_inner(&inner));
        }
    }
    None
}

fn extract_paren_content(s: &str) -> Option<String> {
    let start = s.find('(')?;
    let end = s.rfind(')')?;
    if end <= start {
        return None;
    }
    Some(s[start + 1..end].to_string())
}

fn parse_deprecated_inner(inner: &str) -> Deprecation {
    let mut since = None;
    let mut note = None;
    for part in inner.split(',') {
        let part = part.trim();
        if let Some(val) = part.strip_prefix("since = ") {
            since = Some(strip_quotes(val).to_string());
        } else if let Some(val) = part.strip_prefix("note = ") {
            note = Some(strip_quotes(val).to_string());
        }
    }
    Deprecation { since, note }
}

fn strip_quotes(s: &str) -> &str {
    let s = s.trim();
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

// ===== Signature construction =====

/// Builds a structural signature string for a given AST node and item kind.
/// Strictly follows Rust grammar using ra_ap_syntax typed nodes.
pub fn build_signature(node: &SyntaxNode, kind: &ItemKind) -> Option<String> {
    match kind {
        ItemKind::Function => {
            let func = ast::Fn::cast(node.clone())?;
            signature_for_fn(&func)
        }
        ItemKind::Struct => {
            let st = ast::Struct::cast(node.clone())?;
            signature_for_struct(&st)
        }
        ItemKind::Enum => {
            let en = ast::Enum::cast(node.clone())?;
            signature_for_enum(&en)
        }
        ItemKind::Trait => {
            let tr = ast::Trait::cast(node.clone())?;
            signature_for_trait(&tr)
        }
        ItemKind::Module => None,
    }
}

fn visibility_to_string(vis: Option<ast::Visibility>) -> String {
    match vis {
        None => String::new(),
        Some(v) => {
            let text = v.syntax().text().to_string();
            if text.is_empty() {
                String::new()
            } else {
                format!("{} ", text)
            }
        }
    }
}

fn path_to_string(path: &ast::Path) -> String {
    let mut parts = Vec::new();
    let mut current = Some(path.clone());
    while let Some(p) = current {
        if let Some(seg) = p.segment() {
            parts.push(path_segment_to_string(&seg));
        }
        current = p.qualifier();
    }
    parts.reverse();
    parts.join("::")
}

fn path_segment_to_string(seg: &ast::PathSegment) -> String {
    let mut s = String::new();
    if let Some(name) = seg.name_ref() {
        s.push_str(&name_ref_to_string(&name));
    }
    if let Some(gal) = seg.parenthesized_arg_list() {
        s.push_str(&gal.syntax().text().to_string());
    }
    if let Some(rt) = seg.ret_type() {
        if let Some(ty) = rt.ty() {
            s.push_str(" -> ");
            s.push_str(&type_to_string(&ty));
        }
    }
    s
}

fn name_ref_to_string(nr: &ast::NameRef) -> String {
    nr.ident_token()
        .map(|t| t.text().to_string())
        .or_else(|| nr.Self_token().map(|_| "Self".to_string()))
        .or_else(|| nr.self_token().map(|_| "self".to_string()))
        .or_else(|| nr.super_token().map(|_| "super".to_string()))
        .or_else(|| nr.crate_token().map(|_| "crate".to_string()))
        .or_else(|| nr.int_number_token().map(|t| t.text().to_string()))
        .unwrap_or_else(|| "<unknown>".to_string())
}

fn type_to_string(ty: &ast::Type) -> String {
    match ty {
        ast::Type::PathType(pt) => {
            if let Some(path) = pt.path() {
                path_to_string(&path)
            } else {
                "<unknown>".to_string()
            }
        }
        ast::Type::PtrType(pt) => {
            let mut s = "*".to_string();
            if pt.const_token().is_some() {
                s.push_str("const ");
            } else if pt.mut_token().is_some() {
                s.push_str("mut ");
            }
            if let Some(inner) = pt.ty() {
                s.push_str(&type_to_string(&inner));
            }
            s
        }
        ast::Type::RefType(rt) => {
            let mut s = "&".to_string();
            if let Some(lt) = rt.lifetime() {
                if let Some(t) = lt.lifetime_ident_token() {
                    s.push_str("'");
                    s.push_str(&t.text().to_string());
                    s.push(' ');
                }
            }
            if rt.mut_token().is_some() {
                s.push_str("mut ");
            }
            if let Some(inner) = rt.ty() {
                s.push_str(&type_to_string(&inner));
            }
            s
        }
        ast::Type::SliceType(st) => {
            let mut s = "[".to_string();
            if let Some(inner) = st.ty() {
                s.push_str(&type_to_string(&inner));
            }
            s.push(']');
            s
        }
        ast::Type::ArrayType(at) => {
            let mut s = "[".to_string();
            if let Some(inner) = at.ty() {
                s.push_str(&type_to_string(&inner));
            }
            s.push(';');
            if let Some(ca) = at.const_arg() {
                s.push_str(&ca.syntax().text().to_string());
            }
            s.push(']');
            s
        }
        ast::Type::TupleType(tt) => {
            let mut s = "(".to_string();
            let fields: Vec<String> = tt.fields().map(|t| type_to_string(&t)).collect();
            s.push_str(&fields.join(", "));
            if tt.fields().count() == 1 {
                s.push(',');
            }
            s.push(')');
            s
        }
        ast::Type::FnPtrType(fpt) => {
            let mut s = "fn".to_string();
            if let Some(pl) = fpt.param_list() {
                s.push('(');
                let params: Vec<String> = pl
                    .params()
                    .map(|p| {
                        if let Some(ty) = p.ty() {
                            type_to_string(&ty)
                        } else {
                            "_".to_string()
                        }
                    })
                    .collect();
                s.push_str(&params.join(", "));
                s.push(')');
            }
            if let Some(rt) = fpt.ret_type() {
                if let Some(ty) = rt.ty() {
                    s.push_str(" -> ");
                    s.push_str(&type_to_string(&ty));
                }
            }
            s
        }
        ast::Type::ImplTraitType(itt) => {
            let mut s = "impl ".to_string();
            if let Some(tbl) = itt.type_bound_list() {
                let bounds: Vec<String> =
                    tbl.bounds().map(|b| type_bound_to_string(&b)).collect();
                s.push_str(&bounds.join(" + "));
            }
            s
        }
        ast::Type::DynTraitType(dtt) => {
            let mut s = "dyn ".to_string();
            if let Some(tbl) = dtt.type_bound_list() {
                let bounds: Vec<String> =
                    tbl.bounds().map(|b| type_bound_to_string(&b)).collect();
                s.push_str(&bounds.join(" + "));
            }
            s
        }
        ast::Type::NeverType(_) => "!".to_string(),
        ast::Type::InferType(_) => "_".to_string(),
        ast::Type::ParenType(_) => "()".to_string(),
        ast::Type::MacroType(mt) => mt.syntax().text().to_string(),
        ast::Type::PatternType(_) => "#[pat]".to_string(),
        ast::Type::ForType(_) => "for<..>".to_string(),
    }
}

fn type_bound_to_string(b: &ast::TypeBound) -> String {
    let mut s = String::new();
    if let Some(fb) = b.for_binder() {
        s.push_str("for<");
        if let Some(gpl) = fb.generic_param_list() {
            s.push_str(&generic_params_to_string(&gpl));
        }
        s.push_str("> ");
    }
    if let Some(lt) = b.lifetime() {
        if let Some(t) = lt.lifetime_ident_token() {
            s.push_str("'");
            s.push_str(&t.text().to_string());
            s.push(' ');
        }
    }
    if let Some(ty) = b.ty() {
        s.push_str(&type_to_string(&ty));
    }
    if b.question_mark_token().is_some() {
        s.push('?');
    }
    s
}

fn generic_params_to_string(gpl: &ast::GenericParamList) -> String {
    let params: Vec<String> = gpl
        .generic_params()
        .map(|gp| generic_param_to_string(&gp))
        .collect();
    format!("<{}>", params.join(", "))
}

fn generic_param_to_string(gp: &ast::GenericParam) -> String {
    match gp {
        ast::GenericParam::TypeParam(tp) => {
            let mut s = String::new();
            if let Some(name) = tp.name() {
                s.push_str(&name.text().to_string());
            }
            if let Some(tbl) = tp.type_bound_list() {
                let bounds: Vec<String> =
                    tbl.bounds().map(|b| type_bound_to_string(&b)).collect();
                if !bounds.is_empty() {
                    s.push(':');
                    s.push_str(&bounds.join(" + "));
                }
            }
            if let Some(eq) = tp.eq_token() {
                let _ = eq;
                s.push_str(" = ");
                if let Some(dt) = tp.default_type() {
                    s.push_str(&type_to_string(&dt));
                }
            }
            s
        }
        ast::GenericParam::LifetimeParam(lp) => {
            let mut s = String::new();
            if let Some(lt) = lp.lifetime() {
                if let Some(t) = lt.lifetime_ident_token() {
                    s.push_str("'");
                    s.push_str(&t.text().to_string());
                }
            }
            if let Some(tbl) = lp.type_bound_list() {
                let bounds: Vec<String> =
                    tbl.bounds().map(|b| type_bound_to_string(&b)).collect();
                if !bounds.is_empty() {
                    s.push(':');
                    s.push_str(&bounds.join(" + "));
                }
            }
            s
        }
        ast::GenericParam::ConstParam(cp) => {
            let mut s = String::new();
            s.push_str("const ");
            if let Some(name) = cp.name() {
                s.push_str(&name.text().to_string());
            }
            if let Some(ty) = cp.ty() {
                s.push(':');
                s.push_str(&type_to_string(&ty));
            }
            if let Some(dv) = cp.default_val() {
                s.push_str(" = ");
                s.push_str(&dv.syntax().text().to_string());
            }
            s
        }
    }
}

fn where_clause_to_string(wc: &ast::WhereClause) -> String {
    let preds: Vec<String> = wc
        .predicates()
        .map(|p| where_pred_to_string(&p))
        .collect();
    format!("where {}", preds.join(", "))
}

fn where_pred_to_string(p: &ast::WherePred) -> String {
    let mut s = String::new();
    if let Some(fb) = p.for_binder() {
        s.push_str("for<");
        if let Some(gpl) = fb.generic_param_list() {
            s.push_str(&generic_params_to_string(&gpl));
        }
        s.push_str("> ");
    }
    if let Some(lt) = p.lifetime() {
        if let Some(t) = lt.lifetime_ident_token() {
            s.push_str("'");
            s.push_str(&t.text().to_string());
            s.push(':');
            s.push(' ');
        }
    }
    if let Some(ty) = p.ty() {
        s.push_str(&type_to_string(&ty));
    }
    if let Some(tbl) = p.type_bound_list() {
        s.push(':');
        let bounds: Vec<String> = tbl
            .bounds()
            .map(|b| type_bound_to_string(&b))
            .collect();
        s.push_str(&bounds.join(" + "));
    }
    s
}

fn self_param_to_string(sp: &ast::SelfParam) -> String {
    let mut s = String::new();
    if let Some(lt) = sp.lifetime() {
        if let Some(t) = lt.lifetime_ident_token() {
            s.push_str("'");
            s.push_str(&t.text().to_string());
            s.push(' ');
        }
    }
    if sp.amp_token().is_some() {
        s.push('&');
    }
    if sp.mut_token().is_some() {
        s.push_str("mut ");
    }
    if sp.ty().is_some() {
        s.push_str("self: ");
        if let Some(ty) = sp.ty() {
            s.push_str(&type_to_string(&ty));
        }
    } else {
        s.push_str("self");
    }
    s
}

fn param_to_string(p: &ast::Param) -> String {
    let mut s = String::new();
    if let Some(pat) = p.pat() {
        s.push_str(&pat.syntax().text().to_string());
    }
    if p.colon_token().is_some() {
        s.push(':');
        if let Some(ty) = p.ty() {
            s.push(' ');
            s.push_str(&type_to_string(&ty));
        }
    }
    if p.dotdotdot_token().is_some() {
        s.push_str("...");
    }
    s
}

fn record_field_to_string(f: &ast::RecordField) -> String {
    let mut s = String::new();
    if let Some(vis) = f.visibility() {
        let text = vis.syntax().text().to_string();
        if !text.is_empty() {
            s.push_str(&text);
            s.push(' ');
        }
    }
    if let Some(name) = f.name() {
        s.push_str(&name.text().to_string());
    }
    if let Some(ty) = f.ty() {
        s.push_str(": ");
        s.push_str(&type_to_string(&ty));
    }
    if let Some(dv) = f.default_val() {
        s.push_str(" = ");
        s.push_str(&dv.syntax().text().to_string());
    }
    s
}

fn tuple_field_to_string(f: &ast::TupleField) -> String {
    let mut s = String::new();
    if let Some(vis) = f.visibility() {
        let text = vis.syntax().text().to_string();
        if !text.is_empty() {
            s.push_str(&text);
            s.push(' ');
        }
    }
    if f.mut_restriction().is_some() {
        s.push_str("mut ");
    }
    if let Some(ty) = f.ty() {
        s.push_str(&type_to_string(&ty));
    }
    s
}

fn signature_for_fn(func: &ast::Fn) -> Option<String> {
    let mut s = String::new();

    s.push_str(&visibility_to_string(func.visibility()));

    if func.async_token().is_some() {
        s.push_str("async ");
    }
    if func.const_token().is_some() {
        s.push_str("const ");
    }
    if func.unsafe_token().is_some() {
        s.push_str("unsafe ");
    }
    if func.default_token().is_some() {
        s.push_str("default ");
    }

    if let Some(abi) = func.abi() {
        s.push_str(&abi.syntax().text().to_string());
        s.push(' ');
    }

    s.push_str("fn ");

    if let Some(name) = func.name() {
        s.push_str(&name.text().to_string());
    }

    if let Some(gpl) = func.generic_param_list() {
        s.push_str(&generic_params_to_string(&gpl));
    }

    if let Some(pl) = func.param_list() {
        s.push('(');
        let mut first = true;
        if let Some(sp) = pl.self_param() {
            s.push_str(&self_param_to_string(&sp));
            first = false;
        }
        let params: Vec<String> = pl.params().map(|p| param_to_string(&p)).collect();
        if !params.is_empty() {
            if !first {
                s.push_str(", ");
            }
            s.push_str(&params.join(", "));
        }
        s.push(')');
    }

    if let Some(rt) = func.ret_type() {
        if let Some(ty) = rt.ty() {
            s.push_str(" -> ");
            s.push_str(&type_to_string(&ty));
        }
    }

    if let Some(wc) = func.where_clause() {
        s.push(' ');
        s.push_str(&where_clause_to_string(&wc));
    }

    Some(s)
}

fn signature_for_struct(st: &ast::Struct) -> Option<String> {
    let mut s = String::new();

    s.push_str(&visibility_to_string(st.visibility()));
    s.push_str("struct ");

    if let Some(name) = st.name() {
        s.push_str(&name.text().to_string());
    }

    if let Some(gpl) = st.generic_param_list() {
        s.push_str(&generic_params_to_string(&gpl));
    }

    if let Some(fl) = st.field_list() {
        match fl {
            ast::FieldList::RecordFieldList(rfl) => {
                s.push(' ');
                s.push('{');
                let fields: Vec<String> = rfl
                    .fields()
                    .map(|f| record_field_to_string(&f))
                    .collect();
                s.push_str(&fields.join(", "));
                s.push('}');
            }
            ast::FieldList::TupleFieldList(tfl) => {
                s.push('(');
                let fields: Vec<String> = tfl
                    .fields()
                    .map(|f| tuple_field_to_string(&f))
                    .collect();
                s.push_str(&fields.join(", "));
                s.push(')');
            }
        }
    } else if st.semicolon_token().is_some() {
        s.push(';');
    }

    Some(s)
}

fn signature_for_enum(en: &ast::Enum) -> Option<String> {
    let mut s = String::new();

    s.push_str(&visibility_to_string(en.visibility()));
    s.push_str("enum ");

    if let Some(name) = en.name() {
        s.push_str(&name.text().to_string());
    }

    if let Some(gpl) = en.generic_param_list() {
        s.push_str(&generic_params_to_string(&gpl));
    }

    if let Some(vl) = en.variant_list() {
        s.push(' ');
        s.push('{');
        let variants: Vec<String> = vl
            .variants()
            .map(|v| variant_to_string(&v))
            .collect();
        s.push_str(&variants.join(", "));
        s.push('}');
    }

    Some(s)
}

fn variant_to_string(v: &ast::Variant) -> String {
    let mut s = String::new();
    if let Some(name) = v.name() {
        s.push_str(&name.text().to_string());
    }
    if let Some(fl) = v.field_list() {
        match fl {
            ast::FieldList::RecordFieldList(rfl) => {
                s.push(' ');
                s.push('{');
                let fields: Vec<String> = rfl
                    .fields()
                    .map(|f| record_field_to_string(&f))
                    .collect();
                s.push_str(&fields.join(", "));
                s.push('}');
            }
            ast::FieldList::TupleFieldList(tfl) => {
                s.push('(');
                let fields: Vec<String> = tfl
                    .fields()
                    .map(|f| tuple_field_to_string(&f))
                    .collect();
                s.push_str(&fields.join(", "));
                s.push(')');
            }
        }
    }
    if let Some(eq) = v.eq_token() {
        let _ = eq;
        s.push_str(" = ...");
    }
    if v.const_arg().is_some() {
        s.push_str(" = ...");
    }
    if v.underscore_token().is_some() {
        s.push_str(" _");
    }
    s
}

fn signature_for_trait(tr: &ast::Trait) -> Option<String> {
    let mut s = String::new();

    s.push_str(&visibility_to_string(tr.visibility()));

    if tr.unsafe_token().is_some() {
        s.push_str("unsafe ");
    }
    if tr.auto_token().is_some() {
        s.push_str("auto ");
    }

    s.push_str("trait ");

    if let Some(name) = tr.name() {
        s.push_str(&name.text().to_string());
    }

    if let Some(gpl) = tr.generic_param_list() {
        s.push_str(&generic_params_to_string(&gpl));
    }

    if let Some(tbl) = tr.type_bound_list() {
        s.push(':');
        let bounds: Vec<String> = tbl
            .bounds()
            .map(|b| type_bound_to_string(&b))
            .collect();
        s.push_str(&bounds.join(" + "));
    }

    if let Some(wc) = tr.where_clause() {
        s.push(' ');
        s.push_str(&where_clause_to_string(&wc));
    }

    if tr.semicolon_token().is_some() {
        s.push(';');
    } else {
        s.push(' ');
        s.push('{');
        s.push('}');
    }

    Some(s)
}

/// Extracts the binary symbol export name from item attributes.
/// Follows Rust grammar:
///   #[export_name = "symbol_name"]  — explicit export name
///   #[no_mangle]                    — export name is the item's own name (C style)
/// When both are present, #[export_name] takes precedence.
/// Returns None when no export-controlling attribute is present.
pub fn extract_export_name(attrs: &[String], item_name: &str) -> Option<String> {
    let mut no_mangle = false;
    let mut export_name = None;

    for attr in attrs {
        let trimmed = attr.trim();
        if trimmed == "#[no_mangle]" || trimmed == "#[no_mangle]()" {
            no_mangle = true;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("#[export_name = ") {
            let val = rest.trim_end_matches(']').trim();
            export_name = Some(strip_quotes(val).to_string());
            continue;
        }
    }

    export_name.or_else(|| {
        if no_mangle {
            Some(item_name.to_string())
        } else {
            None
        }
    })
}
