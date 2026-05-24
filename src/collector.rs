use anyhow::{Context, Result};
use ra_ap_syntax::ast::{self, AstNode, HasName, HasVisibility};
use ra_ap_syntax::{Edition, SourceFile, SyntaxKind, SyntaxNode};
use std::fs;
use std::path::Path;

use crate::cfg_expr::{and_cfg, parse_cfg_from_node};
use crate::model::{
    CfgExpr, Diagnostic, DiagnosticLevel, ImplDoc, ImplItemDoc, ItemDoc, ItemKind, SourceLocation,
    Visibility,
};
use crate::util::{
    collect_attrs, extract_docs, join_path, module_path_string, resolve_module_file,
    source_location, source_snippet, type_path_from_ast_type, visibility_from_ast,
};

pub struct RaCollector {
    pub items: Vec<ItemDoc>,
    pub impls: Vec<ImplDoc>,
    pub diagnostics: Vec<Diagnostic>,
}

impl RaCollector {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            impls: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    // Entry point: finds lib.rs or main.rs and walks from there.
    // Preferring lib.rs over main.rs matches Cargo's convention
    // where lib.rs is the crate root for library crates.
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

    // Parses a source file and walks its syntax tree.
    // Each file is parsed independently; there is no cross-file cache
    // because Rust's module system typically maps 1:1 to files.
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

    // Recursive syntax walker.
    // Known item kinds (FN, STRUCT, etc.) are dispatched to typed handlers;
    // unknown nodes are recursed into to find nested items (e.g., items inside
    // mod or impl blocks).
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
                            package_id,
                            current_file,
                            source,
                            tr,
                            mod_path.clone(),
                            inherited_cfg.clone(),
                        )?;
                    }
                }
                SyntaxKind::IMPL => {
                    if let Some(imp) = ast::Impl::cast(child.clone()) {
                        self.collect_impl(
                            workspace_root,
                            package_id,
                            current_file,
                            source,
                            imp,
                            &mod_path,
                            inherited_cfg.clone(),
                        )?;
                    }
                    // Still recurse into the impl body so nested items (methods, etc.)
                    // are also collected as regular ItemDoc for global querying.
                    // This matches the previous behavior before impl tracking was added.
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
                // Recurse into unsupported nodes to find nested items;
                // this handles items inside blocks, etc.
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
        let cfg = and_cfg(inherited_cfg, parse_cfg_from_node(func.syntax()));
        let docs = extract_docs(&func);

        self.push_item(
            package_id,
            name,
            docs,
            cfg,
            ItemKind::Function,
            visibility_from_ast(func.visibility()),
            &mod_path,
            source_location(workspace_root, current_file, source, func.syntax()),
            source_snippet(func.syntax()),
            Some(attrs),
        );

        Ok(())
    }

    fn collect_struct(
        &mut self,
        workspace_root: &Path,
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
            extract_docs(&st),
            and_cfg(inherited_cfg, parse_cfg_from_node(st.syntax())),
            ItemKind::Struct,
            visibility_from_ast(st.visibility()),
            &mod_path,
            source_location(workspace_root, current_file, source, st.syntax()),
            source_snippet(st.syntax()),
            Some(attrs),
        );

        Ok(())
    }

    fn collect_enum(
        &mut self,
        workspace_root: &Path,
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
            extract_docs(&en),
            and_cfg(inherited_cfg, parse_cfg_from_node(en.syntax())),
            ItemKind::Enum,
            visibility_from_ast(en.visibility()),
            &mod_path,
            source_location(workspace_root, current_file, source, en.syntax()),
            source_snippet(en.syntax()),
            Some(attrs),
        );

        Ok(())
    }

    fn collect_trait(
        &mut self,
        workspace_root: &Path,
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
            extract_docs(&tr),
            and_cfg(inherited_cfg, parse_cfg_from_node(tr.syntax())),
            ItemKind::Trait,
            visibility_from_ast(tr.visibility()),
            &mod_path,
            source_location(workspace_root, current_file, source, tr.syntax()),
            source_snippet(tr.syntax()),
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
        let cfg = and_cfg(inherited_cfg.clone(), parse_cfg_from_node(module.syntax()));

        self.push_item(
            package_id,
            name.clone(),
            extract_docs(&module),
            cfg.clone(),
            ItemKind::Module,
            visibility_from_ast(module.visibility()),
            &mod_path,
            source_location(workspace_root, current_file, source, module.syntax()),
            source_snippet(module.syntax()),
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
            let Some(file) = resolve_module_file(current_file, &name) else {
                self.diagnostics.push(Diagnostic {
                    level: DiagnosticLevel::Warning,
                    code: "MODULE_FILE_NOT_FOUND".to_string(),
                    message: format!(
                        "Cannot resolve module file for '{}' in {:?}",
                        name, current_file
                    ),
                    package_id: Some(package_id.to_string()),
                    file: Some(current_file.to_string_lossy().to_string()),
                    line: None,
                });
                return Ok(());
            };
            self.walk_file(workspace_root, crate_root, package_id, &file, mod_path, cfg)?;
        }

        Ok(())
    }

    fn collect_impl(
        &mut self,
        workspace_root: &Path,
        package_id: &str,
        current_file: &Path,
        source: &str,
        imp: ast::Impl,
        mod_path: &[String],
        inherited_cfg: CfgExpr,
    ) -> Result<()> {
        let cfg = and_cfg(inherited_cfg, parse_cfg_from_node(imp.syntax()));

        let self_ty = imp.self_ty().map(|ty| type_path_from_ast_type(&ty));
        let target_typename = self_ty.as_deref().unwrap_or("<unknown>");
        let target_path = join_path(mod_path, target_typename);

        let trait_name = imp.trait_().map(|ty| type_path_from_ast_type(&ty));

        // Collect methods from the impl's associated item list
        let mut items = Vec::new();
        if let Some(item_list) = imp.assoc_item_list() {
            for assoc in item_list.assoc_items() {
                match assoc {
                    ast::AssocItem::Fn(func) => {
                        let name = func
                            .name()
                            .map(|n| n.text().to_string())
                            .unwrap_or_else(|| "<anonymous>".into());

                        let item_cfg = and_cfg(cfg.clone(), parse_cfg_from_node(func.syntax()));
                        let docs = extract_docs(&func);

                        items.push(ImplItemDoc {
                            name,
                            kind: ItemKind::Function,
                            docs,
                            visibility: visibility_from_ast(func.visibility()),
                            cfg: item_cfg,
                            source: source_location(
                                workspace_root,
                                current_file,
                                source,
                                func.syntax(),
                            ),
                            source_code: source_snippet(func.syntax()),
                        });
                    }
                    _ => {}
                }
            }
        }

        let source_loc = source_location(workspace_root, current_file, source, imp.syntax());
        let id = format!(
            "impl:{}:{}:{}",
            package_id, target_path, source_loc.line_start
        );

        self.impls.push(ImplDoc {
            id,
            package_id: package_id.to_string(),
            target_path,
            trait_name,
            cfg,
            source: source_loc,
            source_code: source_snippet(imp.syntax()),
            items,
        });

        Ok(())
    }

    // After all collection is done, associate each impl with its target item
    // by matching target_path (stripped of generics) against item.path.
    pub fn associate_impls(&mut self) {
        let impls = std::mem::take(&mut self.impls);
        for imp in impls {
            let base = imp
                .target_path
                .split('<')
                .next()
                .unwrap_or(&imp.target_path)
                .trim();
            if let Some(item) = self
                .items
                .iter_mut()
                .find(|item| item.package_id == imp.package_id && item.path == base)
            {
                item.impls.push(imp);
            }
        }
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
        // id includes line_start to disambiguate items with identical paths;
        // this can happen when macro expansion produces items at the same path
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
            impls: Vec::new(),
        });
    }
}
