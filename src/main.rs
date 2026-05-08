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
pub struct SourceLocation {
    pub file: String,
    pub line_start: usize,
    pub line_end: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ItemDoc {
    pub path: String,
    pub docs: String,
    pub cfg: CfgExpr,
    pub kind: ItemKind,
    pub source: SourceLocation,
    pub source_code: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: Option<String>,
    pub edition: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub readme_path: Option<String>,
    pub readme: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OutputDoc {
    pub package: Option<PackageInfo>,
    pub items: Vec<ItemDoc>,
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

    pub fn collect_crate(&mut self, crate_root: &Path) -> Result<()> {
        if let Some(member_roots) = workspace_member_roots(crate_root)? {
            for member_root in member_roots {
                self.collect_crate(&member_root)?;
            }

            return Ok(());
        }

        let src = crate_root.join("src");

        let root = if src.join("lib.rs").exists() {
            src.join("lib.rs")
        } else {
            src.join("main.rs")
        };

        self.walk_file(crate_root, &root, vec![], CfgExpr::True)?;

        Ok(())
    }

    fn walk_file(
        &mut self,
        crate_root: &Path,
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
            crate_root,
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
        crate_root: &Path,
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
                            crate_root,
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
                            crate_root,
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
                            crate_root,
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
                            crate_root,
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
                            crate_root,
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
                        crate_root,
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
        crate_root: &Path,
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
            name,
            docs,
            cfg,
            ItemKind::Function,
            mod_path,
            source_location(crate_root, current_file, source, func.syntax()),
            source_snippet(source, func.syntax()),
        );

        Ok(())
    }

    fn collect_struct(
        &mut self,
        crate_root: &Path,
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
            name,
            extract_docs(&attrs),
            and_cfg(inherited_cfg, parse_cfg_attrs(&attrs)),
            ItemKind::Struct,
            mod_path,
            source_location(crate_root, current_file, source, st.syntax()),
            source_snippet(source, st.syntax()),
        );

        Ok(())
    }

    fn collect_enum(
        &mut self,
        crate_root: &Path,
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
            name,
            extract_docs(&attrs),
            and_cfg(inherited_cfg, parse_cfg_attrs(&attrs)),
            ItemKind::Enum,
            mod_path,
            source_location(crate_root, current_file, source, en.syntax()),
            source_snippet(source, en.syntax()),
        );

        Ok(())
    }

    fn collect_trait(
        &mut self,
        crate_root: &Path,
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
            name,
            extract_docs(&attrs),
            and_cfg(inherited_cfg, parse_cfg_attrs(&attrs)),
            ItemKind::Trait,
            mod_path,
            source_location(crate_root, current_file, source, tr.syntax()),
            source_snippet(source, tr.syntax()),
        );

        Ok(())
    }

    fn collect_module(
        &mut self,
        crate_root: &Path,
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
            name.clone(),
            extract_docs(&attrs),
            cfg.clone(),
            ItemKind::Module,
            mod_path.clone(),
            source_location(crate_root, current_file, source, module.syntax()),
            source_snippet(source, module.syntax()),
        );

        mod_path.push(name.clone());

        if let Some(item_list) = module.item_list() {
            self.walk_syntax_node(
                crate_root,
                item_list.syntax().clone(),
                current_file,
                source,
                mod_path,
                cfg,
            )?;
        } else {
            let file = resolve_module_file(current_file, &name)?;

            self.walk_file(crate_root, &file, mod_path, cfg)?;
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
        source: SourceLocation,
        source_code: String,
    ) {
        let path = join_path(&mod_path, &name);

        self.items.push(ItemDoc {
            path,
            docs,
            cfg,
            kind,
            source,
            source_code,
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

fn source_location(
    crate_root: &Path,
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
        .strip_prefix(crate_root)
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|_| current_file.to_string_lossy().to_string());

    SourceLocation {
        file,
        line_start: start_line,
        line_end: end_line,
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

fn read_package_info(crate_root: &Path) -> Result<Option<PackageInfo>> {
    let path = crate_root.join("Cargo.toml");

    if !path.exists() {
        return Ok(None);
    }

    let text = fs::read_to_string(&path).with_context(|| format!("Failed reading {:?}", path))?;

    let doc = toml::from_str::<toml::Value>(&text)
        .with_context(|| format!("Failed parsing {:?}", path))?;

    let package = doc.get("package").and_then(|v| v.as_table());
    let workspace = doc.get("workspace").and_then(|v| v.as_table());

    if package.is_none() && workspace.is_none() {
        return Ok(None);
    }

    let get_string = |key: &str| {
        package
            .and_then(|table| table.get(key))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    };

    let mut readme_path = match package.and_then(|table| table.get("readme")) {
        Some(value) if value.is_str() => value.as_str().map(|s| s.to_string()),
        Some(value) if value.is_bool() => value.as_bool().and_then(|enabled| {
            if enabled {
                Some("README.md".to_string())
            } else {
                None
            }
        }),
        _ => None,
    };

    if readme_path.is_none() && workspace.is_some() && crate_root.join("README.md").exists() {
        readme_path = Some("README.md".to_string());
    }

    let readme = match readme_path.as_deref() {
        Some(path) => {
            let resolved = crate_root.join(path);

            if resolved.exists() {
                Some(
                    fs::read_to_string(&resolved)
                        .with_context(|| format!("Failed reading {:?}", resolved))?,
                )
            } else {
                None
            }
        }
        None => None,
    };

    let name = match get_string("name") {
        Some(value) => value,
        None => crate_root
            .file_name()
            .and_then(|value| value.to_str())
            .map(|value| value.to_string())
            .unwrap_or_else(|| "workspace".to_string()),
    };

    Ok(Some(PackageInfo {
        name,
        version: get_string("version"),
        edition: get_string("edition"),
        description: get_string("description"),
        license: get_string("license"),
        readme_path,
        readme,
    }))
}

fn workspace_member_roots(crate_root: &Path) -> Result<Option<Vec<PathBuf>>> {
    let path = crate_root.join("Cargo.toml");

    if !path.exists() {
        return Ok(None);
    }

    let text = fs::read_to_string(&path).with_context(|| format!("Failed reading {:?}", path))?;

    let doc = toml::from_str::<toml::Value>(&text)
        .with_context(|| format!("Failed parsing {:?}", path))?;

    let workspace = match doc.get("workspace").and_then(|v| v.as_table()) {
        Some(table) => table,
        None => return Ok(None),
    };

    let members = match workspace.get("members").and_then(|v| v.as_array()) {
        Some(members) => members,
        None => return Ok(None),
    };

    let mut roots = Vec::new();

    for member in members {
        if let Some(member) = member.as_str() {
            let member_root = crate_root.join(member);
            let src = member_root.join("src");

            if src.join("lib.rs").exists() || src.join("main.rs").exists() {
                roots.push(member_root);
            }
        }
    }

    Ok(Some(roots))
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

    std::env::set_current_dir(&crate_root)
        .with_context(|| format!("Failed to change working directory to {:?}", crate_root))?;

    let mut collector = RaCollector::new();

    collector.collect_crate(Path::new("."))?;

    let items = collector.items;
    let items_len = items.len();

    let output = OutputDoc {
        package: read_package_info(Path::new("."))?,
        items,
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

    println!("Collected {} items", items_len);

    Ok(())
}
