use anyhow::{Context, Result};
use cargo_metadata::{DependencyKind, Metadata, MetadataCommand, Package, Target};
use chrono::{SecondsFormat, Utc};
use ra_ap_syntax::ast::{self, AstNode, HasName, HasVisibility};
use ra_ap_syntax::{Edition, SourceFile, SyntaxKind, SyntaxNode};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", content = "value")]
pub enum CfgExpr {
    True,
    False,

    Feature(String),
    Flag(String),

    All(Vec<CfgExpr>),
    Any(Vec<CfgExpr>),
    Not(Box<CfgExpr>),
}

#[derive(Debug, Clone, Serialize)]
pub enum ItemKind {
    Function,
    Struct,
    Enum,
    Trait,
    Module,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceLocation {
    pub file: String,
    pub line_start: usize,
    pub line_end: usize,
    pub col_start: Option<usize>,
    pub col_end: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GeneratorInfo {
    pub name: String,
    pub version: String,
    pub rustc_version: Option<String>,
    pub cargo_version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceInfo {
    pub root_dir: String,
    pub root_manifest_path: String,
    pub target_directory: Option<String>,
    pub resolver: Option<String>,
    pub members: Vec<PackageId>,
    pub default_members: Vec<PackageId>,
}

pub type PackageId = String;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageSourceKind {
    WorkspaceMember,
    Path,
    Registry,
    Git,
}

#[derive(Debug, Clone, Serialize)]
pub struct PackageSource {
    pub kind: PackageSourceKind,
    pub value: Option<String>,
    pub rev: Option<String>,
    pub branch: Option<String>,
    pub tag: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TargetDoc {
    pub name: String,
    pub kind: Vec<String>,
    pub crate_types: Option<Vec<String>>,
    pub src_path: String,
    pub edition: Option<String>,
    pub doc: bool,
    pub test: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PackageDoc {
    pub id: PackageId,
    pub name: String,
    pub version: Option<String>,
    pub edition: Option<String>,
    pub rust_version: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub homepage: Option<String>,
    pub manifest_path: String,
    pub root_dir: String,
    pub source: PackageSource,
    pub is_workspace_member: bool,
    pub features: BTreeMap<String, Vec<String>>,
    pub readme_path: Option<String>,
    pub readme: Option<String>,
    pub targets: Vec<TargetDoc>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyKindDoc {
    Normal,
    Dev,
    Build,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeclaredDependencyEdge {
    pub from_package_id: PackageId,
    pub dep_name: String,
    pub to_package_id: Option<PackageId>,
    pub package_rename: Option<String>,
    pub req: Option<String>,
    pub kind: DependencyKindDoc,
    pub optional: bool,
    pub uses_default_features: bool,
    pub features: Vec<String>,
    pub target_cfg: Option<CfgExpr>,
    pub path: Option<String>,
    pub registry: Option<String>,
    pub git: Option<String>,
    pub branch: Option<String>,
    pub tag: Option<String>,
    pub rev: Option<String>,
}

pub type LockNodeId = String;

#[derive(Debug, Clone, Serialize)]
pub struct LockNode {
    pub id: LockNodeId,
    pub name: String,
    pub version: String,
    pub source: Option<String>,
    pub checksum: Option<String>,
    pub replace: Option<String>,
    pub workspace_member: bool,
    pub package_id: Option<PackageId>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LockEdgeKind {
    Normal,
    Dev,
    Build,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct LockEdge {
    pub from: LockNodeId,
    pub to: LockNodeId,
    pub kind: LockEdgeKind,
    pub req: Option<String>,
    pub optional: Option<bool>,
    pub uses_default_features: Option<bool>,
    pub features: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LockGraph {
    pub lockfile_path: String,
    pub lockfile_version: Option<u32>,
    pub generated_by: Option<String>,
    pub nodes: Vec<LockNode>,
    pub edges: Vec<LockEdge>,
    pub roots: Vec<LockNodeId>,
}

pub type ItemId = String;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Public,
    Crate,
    Restricted,
}

#[derive(Debug, Clone, Serialize)]
pub struct Deprecation {
    pub since: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ItemDoc {
    pub id: ItemId,
    pub package_id: PackageId,
    pub path: String,
    pub name: String,
    pub module_path: String,
    pub kind: ItemKind,
    pub visibility: Visibility,
    pub docs: String,
    pub cfg: CfgExpr,
    pub source: SourceLocation,
    pub source_code: String,
    pub signature: Option<String>,
    pub attributes: Option<Vec<String>>,
    pub deprecated: Option<Deprecation>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticLevel {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub code: String,
    pub message: String,
    pub package_id: Option<PackageId>,
    pub file: Option<String>,
    pub line: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InternalDoc {
    pub format: String,
    pub generated_at: String,
    pub generator: GeneratorInfo,
    pub workspace: WorkspaceInfo,
    pub packages: Vec<PackageDoc>,
    pub declared_dependencies: Vec<DeclaredDependencyEdge>,
    pub lock_graph: Option<LockGraph>,
    pub items: Vec<ItemDoc>,
    pub diagnostics: Vec<Diagnostic>,
}

struct CliOptions {
    crate_root: PathBuf,
    output: Option<PathBuf>,
    stdout: bool,
}

pub struct RaCollector {
    pub items: Vec<ItemDoc>,
}

impl RaCollector {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn collect_crate(
        &mut self,
        workspace_root: &Path,
        crate_root: &Path,
        package_id: &str,
        package_name: &str,
    ) -> Result<()> {
        let src = crate_root.join("src");
        let lib_rs = src.join("lib.rs");
        let main_rs = src.join("main.rs");
        let root = if lib_rs.exists() { lib_rs } else { main_rs };

        if !root.exists() {
            return Ok(());
        }

        self.walk_file(
            workspace_root,
            crate_root,
            package_id,
            &root,
            vec![package_name.to_string()],
            CfgExpr::True,
        )?;

        Ok(())
    }

    fn walk_file(
        &mut self,
        workspace_root: &Path,
        crate_root: &Path,
        package_id: &str,
        file_path: &Path,
        mod_path: Vec<String>,
        inherited_cfg: CfgExpr,
    ) -> Result<()> {
        let source = fs::read_to_string(file_path)
            .with_context(|| format!("Failed reading {:?}", file_path))?;

        let parsed = SourceFile::parse(&source, Edition::CURRENT);

        if !parsed.errors().is_empty() {
            eprintln!("Parse warnings in {:?}", file_path);
        }

        let syntax = parsed.tree().syntax().clone();

        self.walk_syntax_node(
            workspace_root,
            crate_root,
            package_id,
            syntax,
            file_path,
            &source,
            mod_path,
            inherited_cfg,
        )?;

        Ok(())
    }

    fn walk_syntax_node(
        &mut self,
        workspace_root: &Path,
        crate_root: &Path,
        package_id: &str,
        node: SyntaxNode,
        current_file: &Path,
        source: &str,
        mod_path: Vec<String>,
        inherited_cfg: CfgExpr,
    ) -> Result<()> {
        for child in node.children() {
            match child.kind() {
                SyntaxKind::FN => {
                    if let Some(func) = ast::Fn::cast(child.clone()) {
                        self.collect_fn(
                            workspace_root,
                            crate_root,
                            package_id,
                            current_file,
                            source,
                            func,
                            mod_path.clone(),
                            inherited_cfg.clone(),
                        )?;
                    }
                }

                SyntaxKind::STRUCT => {
                    if let Some(st) = ast::Struct::cast(child.clone()) {
                        self.collect_struct(
                            workspace_root,
                            crate_root,
                            package_id,
                            current_file,
                            source,
                            st,
                            mod_path.clone(),
                            inherited_cfg.clone(),
                        )?;
                    }
                }

                SyntaxKind::ENUM => {
                    if let Some(en) = ast::Enum::cast(child.clone()) {
                        self.collect_enum(
                            workspace_root,
                            crate_root,
                            package_id,
                            current_file,
                            source,
                            en,
                            mod_path.clone(),
                            inherited_cfg.clone(),
                        )?;
                    }
                }

                SyntaxKind::TRAIT => {
                    if let Some(tr) = ast::Trait::cast(child.clone()) {
                        self.collect_trait(
                            workspace_root,
                            crate_root,
                            package_id,
                            current_file,
                            source,
                            tr,
                            mod_path.clone(),
                            inherited_cfg.clone(),
                        )?;
                    }
                }

                SyntaxKind::MODULE => {
                    if let Some(module) = ast::Module::cast(child.clone()) {
                        self.collect_module(
                            workspace_root,
                            crate_root,
                            package_id,
                            module,
                            current_file,
                            source,
                            mod_path.clone(),
                            inherited_cfg.clone(),
                        )?;
                    }
                }

                _ => {
                    self.walk_syntax_node(
                        workspace_root,
                        crate_root,
                        package_id,
                        child,
                        current_file,
                        source,
                        mod_path.clone(),
                        inherited_cfg.clone(),
                    )?;
                }
            }
        }

        Ok(())
    }

    fn collect_fn(
        &mut self,
        workspace_root: &Path,
        crate_root: &Path,
        package_id: &str,
        current_file: &Path,
        source: &str,
        func: ast::Fn,
        mod_path: Vec<String>,
        inherited_cfg: CfgExpr,
    ) -> Result<()> {
        let name = func
            .name()
            .map(|n| n.text().to_string())
            .unwrap_or_else(|| "<anonymous>".into());

        let attrs = collect_attrs(func.syntax());

        let cfg = and_cfg(inherited_cfg, parse_cfg_attrs(&attrs));

        let docs = extract_docs(&attrs);

        self.push_item(
            package_id,
            name,
            docs,
            cfg,
            ItemKind::Function,
            visibility_from_ast(func.visibility()),
            &mod_path,
            source_location(workspace_root, current_file, source, func.syntax()),
            source_snippet(source, func.syntax()),
            Some(attrs),
        );

        Ok(())
    }

    fn collect_struct(
        &mut self,
        workspace_root: &Path,
        crate_root: &Path,
        package_id: &str,
        current_file: &Path,
        source: &str,
        st: ast::Struct,
        mod_path: Vec<String>,
        inherited_cfg: CfgExpr,
    ) -> Result<()> {
        let name = st
            .name()
            .map(|n| n.text().to_string())
            .unwrap_or_else(|| "<anonymous>".into());

        let attrs = collect_attrs(st.syntax());

        self.push_item(
            package_id,
            name,
            extract_docs(&attrs),
            and_cfg(inherited_cfg, parse_cfg_attrs(&attrs)),
            ItemKind::Struct,
            visibility_from_ast(st.visibility()),
            &mod_path,
            source_location(workspace_root, current_file, source, st.syntax()),
            source_snippet(source, st.syntax()),
            Some(attrs),
        );

        Ok(())
    }

    fn collect_enum(
        &mut self,
        workspace_root: &Path,
        crate_root: &Path,
        package_id: &str,
        current_file: &Path,
        source: &str,
        en: ast::Enum,
        mod_path: Vec<String>,
        inherited_cfg: CfgExpr,
    ) -> Result<()> {
        let name = en
            .name()
            .map(|n| n.text().to_string())
            .unwrap_or_else(|| "<anonymous>".into());

        let attrs = collect_attrs(en.syntax());

        self.push_item(
            package_id,
            name,
            extract_docs(&attrs),
            and_cfg(inherited_cfg, parse_cfg_attrs(&attrs)),
            ItemKind::Enum,
            visibility_from_ast(en.visibility()),
            &mod_path,
            source_location(workspace_root, current_file, source, en.syntax()),
            source_snippet(source, en.syntax()),
            Some(attrs),
        );

        Ok(())
    }

    fn collect_trait(
        &mut self,
        workspace_root: &Path,
        crate_root: &Path,
        package_id: &str,
        current_file: &Path,
        source: &str,
        tr: ast::Trait,
        mod_path: Vec<String>,
        inherited_cfg: CfgExpr,
    ) -> Result<()> {
        let name = tr
            .name()
            .map(|n| n.text().to_string())
            .unwrap_or_else(|| "<anonymous>".into());

        let attrs = collect_attrs(tr.syntax());

        self.push_item(
            package_id,
            name,
            extract_docs(&attrs),
            and_cfg(inherited_cfg, parse_cfg_attrs(&attrs)),
            ItemKind::Trait,
            visibility_from_ast(tr.visibility()),
            &mod_path,
            source_location(workspace_root, current_file, source, tr.syntax()),
            source_snippet(source, tr.syntax()),
            Some(attrs),
        );

        Ok(())
    }

    fn collect_module(
        &mut self,
        workspace_root: &Path,
        crate_root: &Path,
        package_id: &str,
        module: ast::Module,
        current_file: &Path,
        source: &str,
        mut mod_path: Vec<String>,
        inherited_cfg: CfgExpr,
    ) -> Result<()> {
        let name = module
            .name()
            .map(|n| n.text().to_string())
            .unwrap_or_else(|| "<anonymous>".into());

        let attrs = collect_attrs(module.syntax());

        let cfg = and_cfg(inherited_cfg, parse_cfg_attrs(&attrs));

        self.push_item(
            package_id,
            name.clone(),
            extract_docs(&attrs),
            cfg.clone(),
            ItemKind::Module,
            visibility_from_ast(module.visibility()),
            &mod_path,
            source_location(workspace_root, current_file, source, module.syntax()),
            source_snippet(source, module.syntax()),
            Some(attrs.clone()),
        );

        mod_path.push(name.clone());

        if let Some(item_list) = module.item_list() {
            self.walk_syntax_node(
                workspace_root,
                crate_root,
                package_id,
                item_list.syntax().clone(),
                current_file,
                source,
                mod_path,
                cfg,
            )?;
        } else {
            let file = resolve_module_file(current_file, &name)?;

            self.walk_file(workspace_root, crate_root, package_id, &file, mod_path, cfg)?;
        }

        Ok(())
    }

    fn push_item(
        &mut self,
        package_id: &str,
        name: String,
        docs: String,
        cfg: CfgExpr,
        kind: ItemKind,
        visibility: Visibility,
        mod_path: &[String],
        source: SourceLocation,
        source_code: String,
        attributes: Option<Vec<String>>,
    ) {
        let path = join_path(mod_path, &name);
        let module_path = module_path_string(mod_path);
        let id = format!("item:{}:{}:{}", package_id, path, source.line_start);

        self.items.push(ItemDoc {
            id,
            package_id: package_id.to_string(),
            path,
            name,
            module_path,
            kind,
            visibility,
            docs,
            cfg,
            source,
            source_code,
            signature: None,
            attributes,
            deprecated: None,
        });
    }
}

fn collect_attrs(node: &SyntaxNode) -> Vec<String> {
    let mut out = Vec::new();

    for child in node.children() {
        if child.kind() == SyntaxKind::ATTR {
            out.push(child.text().to_string());
        }
    }

    out
}

fn extract_docs(attrs: &[String]) -> String {
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

fn parse_cfg_attrs(attrs: &[String]) -> CfgExpr {
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

fn parse_cfg_expr(input: &str) -> CfgExpr {
    // VERY SIMPLE prototype parser.
    //
    // Replace later with:
    //
    // ra_ap_cfg
    // or cfg-expr.

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

fn and_cfg(a: CfgExpr, b: CfgExpr) -> CfgExpr {
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

fn resolve_module_file(current_file: &Path, mod_name: &str) -> Result<PathBuf> {
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

fn join_path(path: &[String], item: &str) -> String {
    if path.is_empty() {
        item.to_string()
    } else {
        format!("{}::{}", path.join("::"), item)
    }
}

fn module_path_string(path: &[String]) -> String {
    path.join("::")
}

fn visibility_from_ast(vis: Option<ast::Visibility>) -> Visibility {
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

fn source_location(
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

fn source_snippet(source: &str, node: &SyntaxNode) -> String {
    let range = node.text_range();
    let start: usize = range.start().into();
    let end: usize = range.end().into();

    source.get(start..end).unwrap_or("").trim().to_string()
}

fn line_number_at(source: &str, byte_offset: usize) -> usize {
    source
        .as_bytes()
        .iter()
        .take(byte_offset.min(source.len()))
        .filter(|&&byte| byte == b'\n')
        .count()
        + 1
}

fn rel_path(base: &Path, path: &Path) -> String {
    path.strip_prefix(base)
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string_lossy().to_string())
}

fn package_id_for(name: &str, version: &str, source_key: &str, manifest_path: &str) -> PackageId {
    format!("pkg:{}@{}|{}|{}", name, version, source_key, manifest_path)
}

fn lock_node_id(name: &str, version: &str, source: Option<&str>) -> LockNodeId {
    let source_key = source.unwrap_or("local");

    format!("crate:{}@{}|{}", name, version, source_key)
}

fn parse_lock_dep(spec: &str) -> Option<(String, String, Option<String>)> {
    let trimmed = spec.trim();
    let (main, source) = if let Some(start) = trimmed.rfind(" (") {
        let source = trimmed.strip_suffix(')')?;
        let source = source[start + 2..].trim();
        let main = trimmed[..start].trim();

        (main, Some(source.to_string()))
    } else {
        (trimmed, None)
    };

    let mut parts = main.split_whitespace();
    let name = parts.next()?.to_string();
    let version = parts.next()?.to_string();

    Some((name, version, source))
}

fn load_metadata(manifest_path: &Path) -> Result<Metadata> {
    MetadataCommand::new()
        .manifest_path(manifest_path)
        .exec()
        .with_context(|| format!("Failed running cargo metadata for {:?}", manifest_path))
}

fn workspace_resolver(workspace_root: &Path) -> Option<String> {
    let path = workspace_root.join("Cargo.toml");
    let text = fs::read_to_string(&path).ok()?;
    let doc = toml::from_str::<toml::Value>(&text).ok()?;
    let workspace = doc.get("workspace").and_then(|v| v.as_table())?;

    workspace
        .get("resolver")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
}

fn package_source_kind(source: Option<&str>, is_workspace_member: bool) -> PackageSourceKind {
    if is_workspace_member {
        return PackageSourceKind::WorkspaceMember;
    }

    match source {
        Some(value) if value.starts_with("git+") => PackageSourceKind::Git,
        Some(_) => PackageSourceKind::Registry,
        None => PackageSourceKind::Path,
    }
}

fn target_doc(target: &Target, workspace_root: &Path) -> TargetDoc {
    let crate_types = if target.crate_types.is_empty() {
        None
    } else {
        Some(
            target
                .crate_types
                .iter()
                .map(|value| value.to_string())
                .collect(),
        )
    };

    TargetDoc {
        name: target.name.to_string(),
        kind: target.kind.iter().map(|value| value.to_string()).collect(),
        crate_types,
        src_path: rel_path(workspace_root, Path::new(target.src_path.as_str())),
        edition: Some(target.edition.to_string()),
        doc: target.doc,
        test: target.test,
    }
}

fn package_doc(
    package: &Package,
    workspace_root: &Path,
    package_id: PackageId,
    is_workspace_member: bool,
) -> PackageDoc {
    let manifest_path = Path::new(package.manifest_path.as_str());
    let manifest_rel = rel_path(workspace_root, manifest_path);
    let root_dir = rel_path(
        workspace_root,
        manifest_path.parent().unwrap_or(manifest_path),
    );
    let source_string = package.source.as_ref().map(|value| value.to_string());
    let source_kind = package_source_kind(source_string.as_deref(), is_workspace_member);
    let source_value = match source_kind {
        PackageSourceKind::WorkspaceMember | PackageSourceKind::Path => Some(root_dir.clone()),
        _ => source_string.clone(),
    };

    let readme_path = package
        .readme
        .as_ref()
        .map(|value| rel_path(workspace_root, Path::new(value.as_str())));
    let is_local = manifest_path.starts_with(workspace_root);
    let readme = if is_local {
        readme_path.as_deref().and_then(|path| {
            let resolved = workspace_root.join(path);
            fs::read_to_string(&resolved).ok()
        })
    } else {
        None
    };

    PackageDoc {
        id: package_id,
        name: package.name.to_string(),
        version: Some(package.version.to_string()),
        edition: Some(package.edition.to_string()),
        rust_version: package.rust_version.as_ref().map(|value| value.to_string()),
        description: package.description.clone(),
        license: package.license.clone(),
        repository: package.repository.clone(),
        homepage: package.homepage.clone(),
        manifest_path: manifest_rel,
        root_dir,
        source: PackageSource {
            kind: source_kind,
            value: source_value,
            rev: None,
            branch: None,
            tag: None,
        },
        is_workspace_member,
        features: package.features.clone(),
        readme_path,
        readme,
        targets: package
            .targets
            .iter()
            .map(|target| target_doc(target, workspace_root))
            .collect(),
    }
}

fn dependency_kind(kind: DependencyKind) -> DependencyKindDoc {
    match kind {
        DependencyKind::Normal => DependencyKindDoc::Normal,
        DependencyKind::Development => DependencyKindDoc::Dev,
        DependencyKind::Build => DependencyKindDoc::Build,
        _ => DependencyKindDoc::Normal,
    }
}

fn declared_dependencies(
    metadata: &Metadata,
    workspace_root: &Path,
    package_id_by_manifest: &HashMap<String, PackageId>,
    package_id_by_metadata: &HashMap<String, PackageId>,
) -> Vec<DeclaredDependencyEdge> {
    let mut out = Vec::new();

    for package in &metadata.packages {
        let from_id = match package_id_by_metadata.get(&package.id.to_string()) {
            Some(value) => value.clone(),
            None => continue,
        };

        for dep in &package.dependencies {
            let path = dep
                .path
                .as_ref()
                .map(|value| rel_path(workspace_root, Path::new(value.as_str())));
            let manifest_path = path.as_deref().map(|value| {
                if value.ends_with("Cargo.toml") {
                    value.to_string()
                } else {
                    format!("{}/Cargo.toml", value.trim_end_matches('/'))
                }
            });
            let to_package_id = manifest_path
                .as_deref()
                .and_then(|path| package_id_by_manifest.get(path))
                .cloned();

            out.push(DeclaredDependencyEdge {
                from_package_id: from_id.clone(),
                dep_name: dep.name.clone(),
                to_package_id,
                package_rename: dep.rename.clone(),
                req: Some(dep.req.to_string()),
                kind: dependency_kind(dep.kind),
                optional: dep.optional,
                uses_default_features: dep.uses_default_features,
                features: dep.features.clone(),
                target_cfg: dep
                    .target
                    .as_ref()
                    .map(|value| CfgExpr::Flag(value.to_string())),
                path,
                registry: dep.registry.clone(),
                git: None,
                branch: None,
                tag: None,
                rev: None,
            });
        }
    }

    out
}

fn lock_graph(
    workspace_root: &Path,
    metadata: &Metadata,
    package_id_by_lock_key: &HashMap<String, PackageId>,
    workspace_member_ids: &HashSet<PackageId>,
) -> (Option<LockGraph>, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let lock_path = workspace_root.join("Cargo.lock");

    if !lock_path.exists() {
        diagnostics.push(Diagnostic {
            level: DiagnosticLevel::Warning,
            code: "LOCKFILE_MISSING".to_string(),
            message: "Cargo.lock not found; lock_graph omitted".to_string(),
            package_id: None,
            file: None,
            line: None,
        });

        return (None, diagnostics);
    }

    let content = match fs::read_to_string(&lock_path) {
        Ok(value) => value,
        Err(err) => {
            diagnostics.push(Diagnostic {
                level: DiagnosticLevel::Warning,
                code: "LOCKFILE_READ_FAILED".to_string(),
                message: format!("Failed reading Cargo.lock: {}", err),
                package_id: None,
                file: None,
                line: None,
            });

            return (None, diagnostics);
        }
    };

    let doc = match toml::from_str::<toml::Value>(&content) {
        Ok(value) => value,
        Err(err) => {
            diagnostics.push(Diagnostic {
                level: DiagnosticLevel::Warning,
                code: "LOCKFILE_PARSE_FAILED".to_string(),
                message: format!("Failed parsing Cargo.lock: {}", err),
                package_id: None,
                file: None,
                line: None,
            });

            return (None, diagnostics);
        }
    };

    let lockfile_version = doc
        .get("version")
        .and_then(|value| value.as_integer())
        .and_then(|value| u32::try_from(value).ok());
    let packages = doc
        .get("package")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();

    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for package in packages {
        let table = match package.as_table() {
            Some(value) => value,
            None => continue,
        };

        let name = match table.get("name").and_then(|value| value.as_str()) {
            Some(value) => value.to_string(),
            None => continue,
        };
        let version = match table.get("version").and_then(|value| value.as_str()) {
            Some(value) => value.to_string(),
            None => continue,
        };
        let source = table
            .get("source")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string());
        let checksum = table
            .get("checksum")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string());
        let replace = table
            .get("replace")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string());

        let node_id = lock_node_id(&name, &version, source.as_deref());
        let lock_key = format!(
            "{}@{}|{}",
            name,
            version,
            source.as_deref().unwrap_or("local")
        );
        let package_id = package_id_by_lock_key.get(&lock_key).cloned();
        let workspace_member = package_id
            .as_ref()
            .map(|value| workspace_member_ids.contains(value))
            .unwrap_or(false);

        nodes.push(LockNode {
            id: node_id.clone(),
            name: name.clone(),
            version: version.clone(),
            source: source.clone(),
            checksum,
            replace,
            workspace_member,
            package_id,
        });

        if let Some(deps) = table.get("dependencies").and_then(|value| value.as_array()) {
            for dep in deps {
                let dep_spec = match dep.as_str() {
                    Some(value) => value,
                    None => continue,
                };

                if let Some((dep_name, dep_version, dep_source)) = parse_lock_dep(dep_spec) {
                    let to_id = lock_node_id(&dep_name, &dep_version, dep_source.as_deref());

                    edges.push(LockEdge {
                        from: node_id.clone(),
                        to: to_id,
                        kind: LockEdgeKind::Unknown,
                        req: None,
                        optional: None,
                        uses_default_features: None,
                        features: Vec::new(),
                    });
                }
            }
        }
    }

    nodes.sort_by(|a, b| {
        (a.name.as_str(), a.version.as_str(), a.source.as_deref()).cmp(&(
            b.name.as_str(),
            b.version.as_str(),
            b.source.as_deref(),
        ))
    });

    let mut roots = Vec::new();
    for member_id in &metadata.workspace_members {
        if let Some(package) = metadata.packages.iter().find(|pkg| pkg.id == *member_id) {
            let source = package.source.as_ref().map(|value| value.to_string());
            roots.push(lock_node_id(
                &package.name,
                &package.version.to_string(),
                source.as_deref(),
            ));
        }
    }

    let graph = LockGraph {
        lockfile_path: rel_path(workspace_root, &lock_path),
        lockfile_version,
        generated_by: None,
        nodes,
        edges,
        roots,
    };

    (Some(graph), diagnostics)
}

fn parse_args() -> Result<CliOptions> {
    let mut crate_root = None;
    let mut output = None;
    let mut stdout = false;

    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_usage();
                std::process::exit(0);
            }

            "-r" | "--crate-root" => {
                let value = args.next().context("Missing value after --crate-root")?;
                crate_root = Some(PathBuf::from(value));
            }

            "-o" | "--output" => {
                let value = args.next().context("Missing value after --output")?;
                output = Some(PathBuf::from(value));
            }

            "--stdout" => {
                stdout = true;
            }

            value if value.starts_with('-') => {
                anyhow::bail!("Unknown argument: {}", value);
            }

            value => {
                if crate_root.is_some() {
                    anyhow::bail!("Unexpected positional argument: {}", value);
                }

                crate_root = Some(PathBuf::from(value));
            }
        }
    }

    if stdout && output.is_some() {
        anyhow::bail!("--stdout cannot be combined with --output");
    }

    let crate_root = match crate_root {
        Some(value) => value,
        None if cfg!(debug_assertions) => PathBuf::from("example"),
        None => std::env::current_dir().context("Failed to resolve current directory")?,
    };

    let output = match (stdout, output) {
        (true, output) => output,
        (false, Some(output)) => Some(output),
        (false, None) if cfg!(debug_assertions) => Some(PathBuf::from("web/mdoc.json")),
        (false, None) => None,
    };

    Ok(CliOptions {
        crate_root,
        output,
        stdout,
    })
}

fn print_usage() {
    eprintln!("Usage: cargo-mdoc [crate-root] [--output PATH] [--stdout]");
    eprintln!("       cargo-mdoc --crate-root PATH [--output PATH] [--stdout]");
}

fn main() -> Result<()> {
    let cli = parse_args()?;

    let cwd = std::env::current_dir().context("Failed to resolve current directory")?;
    let crate_root = if cli.crate_root.is_absolute() {
        cli.crate_root.clone()
    } else {
        cwd.join(&cli.crate_root)
    };
    let manifest_path = if crate_root.is_file() {
        crate_root.clone()
    } else {
        crate_root.join("Cargo.toml")
    };

    let output_path = if cli.stdout {
        None
    } else {
        let raw_output = cli.output.unwrap_or_else(|| PathBuf::from("mdoc.json"));

        Some(if raw_output.is_absolute() {
            raw_output
        } else {
            cwd.join(raw_output)
        })
    };

    let metadata = load_metadata(&manifest_path)?;
    let workspace_root = PathBuf::from(metadata.workspace_root.as_str());
    let resolver = workspace_resolver(&workspace_root);

    let mut package_id_by_metadata = HashMap::new();
    let mut package_id_by_manifest = HashMap::new();
    let mut package_id_by_lock_key = HashMap::new();

    let workspace_members: HashSet<String> = metadata
        .workspace_members
        .iter()
        .map(|value| value.to_string())
        .collect();

    let mut packages = Vec::new();

    for package in &metadata.packages {
        let manifest_rel = rel_path(&workspace_root, Path::new(package.manifest_path.as_str()));
        let is_workspace_member = workspace_members.contains(&package.id.to_string());
        let source_string = package.source.as_ref().map(|value| value.to_string());
        let source_key = if is_workspace_member {
            "workspace".to_string()
        } else {
            source_string.clone().unwrap_or_else(|| "path".to_string())
        };
        let package_id = package_id_for(
            &package.name,
            &package.version.to_string(),
            &source_key,
            &manifest_rel,
        );

        package_id_by_metadata.insert(package.id.to_string(), package_id.clone());
        package_id_by_manifest.insert(manifest_rel.clone(), package_id.clone());

        let lock_key = format!(
            "{}@{}|{}",
            package.name,
            package.version,
            source_string.as_deref().unwrap_or("local")
        );
        package_id_by_lock_key.insert(lock_key, package_id.clone());

        packages.push(package_doc(
            package,
            &workspace_root,
            package_id,
            is_workspace_member,
        ));
    }

    packages.sort_by(|a, b| a.id.cmp(&b.id));

    let mut declared_dependencies = declared_dependencies(
        &metadata,
        &workspace_root,
        &package_id_by_manifest,
        &package_id_by_metadata,
    );

    declared_dependencies.sort_by(|a, b| {
        (a.from_package_id.as_str(), a.dep_name.as_str())
            .cmp(&(b.from_package_id.as_str(), b.dep_name.as_str()))
    });

    let workspace_member_ids: HashSet<PackageId> = metadata
        .workspace_members
        .iter()
        .filter_map(|value| package_id_by_metadata.get(&value.to_string()).cloned())
        .collect();

    let (lock_graph, mut diagnostics) = lock_graph(
        &workspace_root,
        &metadata,
        &package_id_by_lock_key,
        &workspace_member_ids,
    );

    let mut collector = RaCollector::new();

    for package in &metadata.packages {
        let is_workspace_member = workspace_members.contains(&package.id.to_string());
        let source_string = package.source.as_ref().map(|value| value.to_string());
        let source_kind = package_source_kind(source_string.as_deref(), is_workspace_member);
        let is_local = matches!(
            source_kind,
            PackageSourceKind::WorkspaceMember | PackageSourceKind::Path
        );

        if !is_local {
            continue;
        }

        let package_id = match package_id_by_metadata.get(&package.id.to_string()) {
            Some(value) => value.clone(),
            None => continue,
        };
        let manifest_path = Path::new(package.manifest_path.as_str());
        let crate_root = match manifest_path.parent() {
            Some(value) => value.to_path_buf(),
            None => continue,
        };

        if !crate_root.exists() {
            diagnostics.push(Diagnostic {
                level: DiagnosticLevel::Warning,
                code: "CRATE_ROOT_MISSING".to_string(),
                message: format!("Crate root missing: {}", crate_root.display()),
                package_id: Some(package_id),
                file: None,
                line: None,
            });
            continue;
        }

        collector.collect_crate(&workspace_root, &crate_root, &package_id, &package.name)?;
    }

    let mut items = collector.items;
    items.sort_by(|a, b| {
        (a.package_id.as_str(), a.path.as_str(), a.source.line_start).cmp(&(
            b.package_id.as_str(),
            b.path.as_str(),
            b.source.line_start,
        ))
    });

    let workspace_members = metadata
        .workspace_members
        .iter()
        .filter_map(|value| package_id_by_metadata.get(&value.to_string()).cloned())
        .collect::<Vec<_>>();
    let default_members = metadata
        .workspace_default_members
        .iter()
        .filter_map(|value| package_id_by_metadata.get(&value.to_string()).cloned())
        .collect::<Vec<_>>();

    let workspace = WorkspaceInfo {
        root_dir: ".".to_string(),
        root_manifest_path: rel_path(&workspace_root, &workspace_root.join("Cargo.toml")),
        target_directory: Some(rel_path(
            &workspace_root,
            Path::new(metadata.target_directory.as_str()),
        )),
        resolver,
        members: workspace_members,
        default_members,
    };

    let output = InternalDoc {
        format: "cargo-mdoc-internal/1.0".to_string(),
        generated_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        generator: GeneratorInfo {
            name: "cargo-mdoc".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            rustc_version: None,
            cargo_version: None,
        },
        workspace,
        packages,
        declared_dependencies,
        lock_graph,
        items,
        diagnostics,
    };

    let json = serde_json::to_string_pretty(&output)?;

    if cli.stdout {
        println!("{}", json);
    } else {
        let output_path = output_path.expect("output path must exist when not writing to stdout");

        if let Some(parent) = output_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create output directory {:?}", parent))?;
        }

        fs::write(&output_path, json)
            .with_context(|| format!("Failed writing {:?}", output_path))?;

        println!("Output: {}", output_path.display());
    }

    println!("Collected {} items", output.items.len());

    Ok(())
}
