use anyhow::{Context, Result};
use ra_ap_syntax::ast::{self, AstNode, HasName, HasVisibility};
use ra_ap_syntax::{Edition, SourceFile, SyntaxKind, SyntaxNode};
use std::path::Path;

use crate::cfg_expr::{and_cfg, parse_cfg_from_node};
use crate::model::{
    CfgExpr, Deprecation, Diagnostic, DiagnosticLevel, ImplDoc, ImplItemDoc, ItemDoc, ItemKind,
    SourceLocation, Visibility, MacroCallDoc,
};
use crate::util::{
    build_signature, collect_attrs, extract_deprecated, extract_docs, extract_export_name,
    has_macro_export, join_path, module_path_string, path_to_string, read_file_if_not_ignored, resolve_module_file,
    source_location, source_snippet, type_path_from_ast_type, visibility_from_ast, GitIgnoreFilter,
};

pub struct RaCollector {
    pub items: Vec<ItemDoc>,
    pub macro_calls: Vec<MacroCallDoc>,
    pub impls: Vec<ImplDoc>,
    pub diagnostics: Vec<Diagnostic>,
}

impl RaCollector {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            macro_calls: Vec::new(),
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
        git_ignore: Option<&GitIgnoreFilter>,
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
            git_ignore,
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
        git_ignore: Option<&GitIgnoreFilter>,
    ) -> Result<()> {
        let source = read_file_if_not_ignored(git_ignore, file_path)
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
            git_ignore,
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
        git_ignore: Option<&GitIgnoreFilter>,
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
                    // Recurse into the function body so macro calls and other
                    // nested constructs inside are also discovered.
                    self.walk_syntax_node(
                        workspace_root,
                        crate_root,
                        package_id,
                        child,
                        current_file,
                        source,
                        mod_path.clone(),
                        inherited_cfg.clone(),
                        git_ignore,
                    )?;
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
                    // Recurse into the item body so nested macro calls are
                    // also discovered.
                    self.walk_syntax_node(
                        workspace_root,
                        crate_root,
                        package_id,
                        child,
                        current_file,
                        source,
                        mod_path.clone(),
                        inherited_cfg.clone(),
                        git_ignore,
                    )?;
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
                    // Recurse into the item body so nested macro calls are
                    // also discovered.
                    self.walk_syntax_node(
                        workspace_root,
                        crate_root,
                        package_id,
                        child,
                        current_file,
                        source,
                        mod_path.clone(),
                        inherited_cfg.clone(),
                        git_ignore,
                    )?;
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
                    // Recurse into the item body so nested macro calls are
                    // also discovered.
                    self.walk_syntax_node(
                        workspace_root,
                        crate_root,
                        package_id,
                        child,
                        current_file,
                        source,
                        mod_path.clone(),
                        inherited_cfg.clone(),
                        git_ignore,
                    )?;
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
                        git_ignore,
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
                            git_ignore,
                        )?;
                    }
                }
                SyntaxKind::MACRO_RULES => {
                    if let Some(mr) = ast::MacroRules::cast(child.clone()) {
                        self.collect_macro_rules(
                            workspace_root,
                            package_id,
                            current_file,
                            source,
                            mr,
                            mod_path.clone(),
                            inherited_cfg.clone(),
                        )?;
                    }
                }
                SyntaxKind::MACRO_CALL => {
                    if let Some(call) = ast::MacroCall::cast(child.clone()) {
                        self.collect_macro_call(
                            workspace_root,
                            crate_root,
                            package_id,
                            current_file,
                            source,
                            call,
                            mod_path.clone(),
                            inherited_cfg.clone(),
                            git_ignore,
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
                        git_ignore,
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
        let deprecated = extract_deprecated(&attrs);
        let signature = build_signature(func.syntax(), &ItemKind::Function);
        let export_name = extract_export_name(&attrs, &name);
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
            signature,
            deprecated,
            export_name,
            false,
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
        let deprecated = extract_deprecated(&attrs);
        let signature = build_signature(st.syntax(), &ItemKind::Struct);
        let export_name = extract_export_name(&attrs, &name);

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
            signature,
            deprecated,
            export_name,
            false,
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
        let deprecated = extract_deprecated(&attrs);
        let signature = build_signature(en.syntax(), &ItemKind::Enum);
        let export_name = extract_export_name(&attrs, &name);

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
            signature,
            deprecated,
            export_name,
            false,
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
        let deprecated = extract_deprecated(&attrs);
        let signature = build_signature(tr.syntax(), &ItemKind::Trait);
        let export_name = extract_export_name(&attrs, &name);

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
            signature,
            deprecated,
            export_name,
            false,
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
        git_ignore: Option<&GitIgnoreFilter>,
    ) -> Result<()> {
        let name = module
            .name()
            .map(|n| n.text().to_string())
            .unwrap_or_else(|| "<anonymous>".into());

        let attrs = collect_attrs(module.syntax());
        let deprecated = extract_deprecated(&attrs);
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
            None,
            deprecated,
            None,
            false,
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
                git_ignore,
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
            self.walk_file(workspace_root, crate_root, package_id, &file, mod_path, cfg, git_ignore)?;
        }

        Ok(())
    }

    fn collect_macro_rules(
        &mut self,
        workspace_root: &Path,
        package_id: &str,
        current_file: &Path,
        source: &str,
        mr: ast::MacroRules,
        mod_path: Vec<String>,
        inherited_cfg: CfgExpr,
    ) -> Result<()> {
        let name = mr
            .name()
            .map(|n| n.text().to_string())
            .unwrap_or_else(|| "<anonymous>".into());

        let attrs = collect_attrs(mr.syntax());
        let deprecated = extract_deprecated(&attrs);
        let signature = build_signature(mr.syntax(), &ItemKind::Macro);
        let export_name = extract_export_name(&attrs, &name);
        let cfg = and_cfg(inherited_cfg, parse_cfg_from_node(mr.syntax()));
        let docs = extract_docs(&mr);
        let is_export = has_macro_export(&attrs);

        self.push_item(
            package_id,
            name,
            docs,
            cfg,
            ItemKind::Macro,
            visibility_from_ast(mr.visibility()),
            &mod_path,
            source_location(workspace_root, current_file, source, mr.syntax()),
            source_snippet(mr.syntax()),
            Some(attrs),
            signature,
            deprecated,
            export_name,
            is_export,
        );

        Ok(())
    }

    // Collects a macro call site, e.g. `my_macro!(...)` or `foo::bar!(...)`.
    // Macro calls are not items, but recording them lets consumers see which
    // macros are used and where, and to correlate call sites with definitions.
    fn collect_macro_call(
        &mut self,
        workspace_root: &Path,
        crate_root: &Path,
        package_id: &str,
        current_file: &Path,
        source: &str,
        call: ast::MacroCall,
        mod_path: Vec<String>,
        inherited_cfg: CfgExpr,
        git_ignore: Option<&GitIgnoreFilter>,
    ) -> Result<()> {
        let path = call
            .path()
            .map(|p| path_to_string(&p))
            .unwrap_or_else(|| "<anonymous>".into());
        let name = path
            .split("::")
            .last()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "<anonymous>".into());

        let attrs = collect_attrs(call.syntax());
        let source_loc = source_location(workspace_root, current_file, source, call.syntax());
        let id = format!(
            "macro_call:{}:{}:{}",
            package_id, path, source_loc.line_start
        );

        self.macro_calls.push(MacroCallDoc {
            id,
            package_id: package_id.to_string(),
            path,
            name,
            module_path: module_path_string(&mod_path),
            source: source_loc,
            source_code: source_snippet(call.syntax()),
            attributes: Some(attrs),
        });

        // Still recurse into the call's token tree in case it contains nested
        // items or further macro calls worth recording.
        self.walk_syntax_node(
            workspace_root,
            crate_root,
            package_id,
            call.syntax().clone(),
            current_file,
            source,
            mod_path,
            inherited_cfg,
            git_ignore,
        )?;

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

                        let attrs = collect_attrs(func.syntax());
                        let deprecated = extract_deprecated(&attrs);
                        let signature = build_signature(func.syntax(), &ItemKind::Function);
                        let export_name = extract_export_name(&attrs, &name);
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
                            signature,
                            deprecated,
                            export_name,
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
        signature: Option<String>,
        deprecated: Option<Deprecation>,
        export_name: Option<String>,
        is_macro_export: bool,
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
            signature,
            attributes,
            deprecated,
            export_name,
            is_macro_export,
            impls: Vec::new(),
        });
    }
}
