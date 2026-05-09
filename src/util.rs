use anyhow::{Context, Result};
use ra_ap_syntax::{AstNode as _, SyntaxKind, SyntaxNode, ast};
use std::path::{Path, PathBuf};

use crate::model::{LockNodeId, PackageId, SourceLocation, Visibility};

pub fn collect_attrs(node: &SyntaxNode) -> Vec<String> {
    let mut out = Vec::new();

    for child in node.children() {
        if child.kind() == SyntaxKind::ATTR {
            out.push(child.text().to_string());
        }
    }

    out
}

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

pub fn source_snippet(source: &str, node: &SyntaxNode) -> String {
    let range = node.text_range();
    let start: usize = range.start().into();
    let end: usize = range.end().into();

    source.get(start..end).unwrap_or("").trim().to_string()
}

pub fn line_number_at(source: &str, byte_offset: usize) -> usize {
    source
        .as_bytes()
        .iter()
        .take(byte_offset.min(source.len()))
        .filter(|&&byte| byte == b'\n')
        .count()
        + 1
}

pub fn rel_path(base: &Path, path: &Path) -> String {
    path.strip_prefix(base)
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string_lossy().to_string())
}

pub fn package_id_for(
    name: &str,
    version: &str,
    source_key: &str,
    manifest_path: &str,
) -> PackageId {
    format!("pkg:{}@{}|{}|{}", name, version, source_key, manifest_path)
}
