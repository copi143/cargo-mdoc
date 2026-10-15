use ra_ap_syntax::ast::HasAttrs;
use ra_ap_syntax::{AstNode as _, SyntaxKind, SyntaxNode, ast};
use std::path::{Path, PathBuf};

use crate::model::{PackageId, SourceLocation, Visibility};

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
    lines
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
        .join("\n")
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
