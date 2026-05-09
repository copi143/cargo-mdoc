use anyhow::{Context, Result};
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

// Extracts doc text from #[doc = "..."] attributes.
// The RA parser converts /// and //! comments into #[doc = "..."] form,
// so this function handles both explicit and doc-comment attributes.
// It uses simple string matching rather than AST traversal because
// doc extraction is a hot path called for every item.
pub fn extract_docs(attrs: &[String]) -> String {
    let mut out = String::new();

    for attr in attrs {
        if attr.starts_with("#[doc") {
            if let Some(start) = attr.find('"') {
                if let Some(end) = attr.rfind('"') {
                    if end > start {
                        out.push_str(&attr[start + 1..end]);
                        out.push('\n');
                    }
                }
            }
        }
    }

    out.trim().to_string()
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
pub fn resolve_module_file(current_file: &Path, mod_name: &str) -> Result<PathBuf> {
    let parent = current_file.parent().context("Missing parent")?;
    let stem = current_file
        .file_stem()
        .and_then(|value| value.to_str())
        .context("Missing file stem")?;

    let direct = parent.join(format!("{}.rs", mod_name));
    let nested = parent.join(mod_name).join("mod.rs");
    let sibling = parent.join(stem).join(format!("{}.rs", mod_name));
    let sibling_nested = parent.join(stem).join(mod_name).join("mod.rs");

    if direct.exists() {
        Ok(direct)
    } else if sibling.exists() {
        Ok(sibling)
    } else if nested.exists() {
        Ok(nested)
    } else if sibling_nested.exists() {
        Ok(sibling_nested)
    } else {
        anyhow::bail!("Cannot resolve module {}", mod_name)
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

// Extracts the source code text for a syntax node.
// The RA SyntaxNode's text_range starts at the first token of the node,
// which may be mid-line (e.g., at `fn` rather than the line start).
//
// To produce clean indented output, we:
//  1. Extend the range to the beginning of the line if the gap is
//     entirely whitespace (indentation). This preserves the original
//     indentation of the first line instead of starting at column 0.
//  2. Compute the common leading whitespace across all lines and strip
//     it, mimicking Python's textwrap.dedent semantics.
//
// This ensures that a function like:
//     fn foo() {
//         bar();
//     }
// produces "fn foo() {\n    bar();\n}" not "fn foo() {\n        bar();\n    }".
pub fn source_snippet(source: &str, node: &SyntaxNode) -> String {
    let range = node.text_range();
    let end: usize = range.end().into();
    let mut start: usize = range.start().into();

    let line_start = source[..start]
        .rfind('\n')
        .map(|i| i + 1)
        .unwrap_or(0);

    // Only extend to line start if the gap is pure whitespace;
    // if there's non-whitespace before the token (e.g. mid-line expression),
    // we keep the original range to avoid capturing unrelated code
    if source[line_start..start].chars().all(|c| c == ' ' || c == '\t') {
        start = line_start;
    }

    let raw = source.get(start..end).unwrap_or("");
    let lines: Vec<&str> = raw.lines().collect();

    if lines.is_empty() {
        return String::new();
    }

    // Find the minimum indentation across non-empty lines
    let common_indent = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);

    // Strip the common indentation from every line
    lines
        .iter()
        .map(|line| {
            if line.len() >= common_indent {
                &line[common_indent..]
            } else {
                line.trim_start()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
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
