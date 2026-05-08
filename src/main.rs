use anyhow::{Context, Result};
use ra_ap_syntax::ast::{self, AstNode, HasName};
use ra_ap_syntax::{Edition, SourceFile, SyntaxKind, SyntaxNode};
use serde::Serialize;
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
pub struct ItemDoc {
    pub path: String,
    pub docs: String,
    pub cfg: CfgExpr,
    pub kind: ItemKind,
}

#[derive(Debug, Clone, Serialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: Option<String>,
    pub edition: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OutputDoc {
    pub package: Option<PackageInfo>,
    pub items: Vec<ItemDoc>,
}

pub struct RaCollector {
    pub items: Vec<ItemDoc>,
}

impl RaCollector {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn collect_crate(&mut self, crate_root: &Path) -> Result<()> {
        let src = crate_root.join("src");

        let root = if src.join("lib.rs").exists() {
            src.join("lib.rs")
        } else {
            src.join("main.rs")
        };

        self.walk_file(&root, vec![], CfgExpr::True)?;

        Ok(())
    }

    fn walk_file(
        &mut self,
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

        self.walk_syntax_node(syntax, file_path, mod_path, inherited_cfg)?;

        Ok(())
    }

    fn walk_syntax_node(
        &mut self,
        node: SyntaxNode,
        current_file: &Path,
        mod_path: Vec<String>,
        inherited_cfg: CfgExpr,
    ) -> Result<()> {
        for child in node.children() {
            match child.kind() {
                SyntaxKind::FN => {
                    if let Some(func) = ast::Fn::cast(child.clone()) {
                        self.collect_fn(func, mod_path.clone(), inherited_cfg.clone())?;
                    }
                }

                SyntaxKind::STRUCT => {
                    if let Some(st) = ast::Struct::cast(child.clone()) {
                        self.collect_struct(st, mod_path.clone(), inherited_cfg.clone())?;
                    }
                }

                SyntaxKind::ENUM => {
                    if let Some(en) = ast::Enum::cast(child.clone()) {
                        self.collect_enum(en, mod_path.clone(), inherited_cfg.clone())?;
                    }
                }

                SyntaxKind::TRAIT => {
                    if let Some(tr) = ast::Trait::cast(child.clone()) {
                        self.collect_trait(tr, mod_path.clone(), inherited_cfg.clone())?;
                    }
                }

                SyntaxKind::MODULE => {
                    if let Some(module) = ast::Module::cast(child.clone()) {
                        self.collect_module(
                            module,
                            current_file,
                            mod_path.clone(),
                            inherited_cfg.clone(),
                        )?;
                    }
                }

                _ => {
                    self.walk_syntax_node(
                        child,
                        current_file,
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

        self.push_item(name, docs, cfg, ItemKind::Function, mod_path);

        Ok(())
    }

    fn collect_struct(
        &mut self,
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
            name,
            extract_docs(&attrs),
            and_cfg(inherited_cfg, parse_cfg_attrs(&attrs)),
            ItemKind::Struct,
            mod_path,
        );

        Ok(())
    }

    fn collect_enum(
        &mut self,
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
            name,
            extract_docs(&attrs),
            and_cfg(inherited_cfg, parse_cfg_attrs(&attrs)),
            ItemKind::Enum,
            mod_path,
        );

        Ok(())
    }

    fn collect_trait(
        &mut self,
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
            name,
            extract_docs(&attrs),
            and_cfg(inherited_cfg, parse_cfg_attrs(&attrs)),
            ItemKind::Trait,
            mod_path,
        );

        Ok(())
    }

    fn collect_module(
        &mut self,
        module: ast::Module,
        current_file: &Path,
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
            name.clone(),
            extract_docs(&attrs),
            cfg.clone(),
            ItemKind::Module,
            mod_path.clone(),
        );

        mod_path.push(name.clone());

        if let Some(item_list) = module.item_list() {
            self.walk_syntax_node(item_list.syntax().clone(), current_file, mod_path, cfg)?;
        } else {
            let file = resolve_module_file(current_file, &name)?;

            self.walk_file(&file, mod_path, cfg)?;
        }

        Ok(())
    }

    fn push_item(
        &mut self,
        name: String,
        docs: String,
        cfg: CfgExpr,
        kind: ItemKind,
        mod_path: Vec<String>,
    ) {
        let path = join_path(&mod_path, &name);

        self.items.push(ItemDoc {
            path,
            docs,
            cfg,
            kind,
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

    let direct = parent.join(format!("{}.rs", mod_name));
    let nested = parent.join(mod_name).join("mod.rs");

    if direct.exists() {
        Ok(direct)
    } else if nested.exists() {
        Ok(nested)
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

fn read_package_info(crate_root: &Path) -> Result<Option<PackageInfo>> {
    let path = crate_root.join("Cargo.toml");

    if !path.exists() {
        return Ok(None);
    }

    let text = fs::read_to_string(&path).with_context(|| format!("Failed reading {:?}", path))?;

    let doc = text
        .parse::<toml::Value>()
        .with_context(|| format!("Failed parsing {:?}", path))?;

    let package = match doc.get("package").and_then(|v| v.as_table()) {
        Some(table) => table,
        None => return Ok(None),
    };

    let get_string = |key: &str| {
        package
            .get(key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    };

    let name = match get_string("name") {
        Some(value) => value,
        None => return Ok(None),
    };

    Ok(Some(PackageInfo {
        name,
        version: get_string("version"),
        edition: get_string("edition"),
        description: get_string("description"),
        license: get_string("license"),
    }))
}

fn main() -> Result<()> {
    let current = std::env::current_dir()?;

    let mut collector = RaCollector::new();

    collector.collect_crate(&current)?;

    let items = collector.items;
    let items_len = items.len();

    let output = OutputDoc {
        package: read_package_info(&current)?,
        items,
    };

    let json = serde_json::to_string_pretty(&output)?;

    fs::write("mdoc-ra.json", json)?;

    println!("Collected {} items", items_len);
    println!("Output: mdoc-ra.json");

    Ok(())
}
