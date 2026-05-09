use anyhow::{Context, Result};
use cargo_metadata::{DependencyKind, Metadata, MetadataCommand, Package, Target};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::model::{
    CfgExpr, DeclaredDependencyEdge, DependencyKindDoc, PackageDoc, PackageId, PackageSource,
    PackageSourceKind, TargetDoc,
};
use crate::util::rel_path;

pub fn load_metadata(manifest_path: &Path) -> Result<Metadata> {
    MetadataCommand::new()
        .manifest_path(manifest_path)
        .exec()
        .with_context(|| format!("Failed running cargo metadata for {:?}", manifest_path))
}

// cargo_metadata::Metadata exposes workspace_resolver only in newer versions;
// we parse the resolver from Cargo.toml directly for broader compatibility.
pub fn workspace_resolver(workspace_root: &Path) -> Option<String> {
    let path = workspace_root.join("Cargo.toml");
    let text = fs::read_to_string(&path).ok()?;
    let doc = toml::from_str::<toml::Value>(&text).ok()?;
    let workspace = doc.get("workspace").and_then(|v| v.as_table())?;

    workspace
        .get("resolver")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
}

pub fn package_source_kind(source: Option<&str>, is_workspace_member: bool) -> PackageSourceKind {
    if is_workspace_member {
        return PackageSourceKind::WorkspaceMember;
    }

    match source {
        Some(value) if value.starts_with("git+") => PackageSourceKind::Git,
        Some(_) => PackageSourceKind::Registry,
        None => PackageSourceKind::Path,
    }
}

pub fn target_doc(target: &Target, workspace_root: &Path) -> TargetDoc {
    let crate_types = if target.crate_types.is_empty() {
        // Empty crate_types is the default (rlib for lib, bin for binary);
        // omit from output to reduce verbosity
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
        src_path: rel_path(workspace_root, Path::new(target.src_path.as_std_path())),
        edition: Some(target.edition.to_string()),
        doc: target.doc,
        test: target.test,
    }
}

pub fn package_doc(
    package: &Package,
    workspace_root: &Path,
    package_id: PackageId,
    is_workspace_member: bool,
) -> PackageDoc {
    let manifest_path = package.manifest_path.as_std_path();
    let manifest_rel = rel_path(workspace_root, manifest_path);
    let root_dir = rel_path(
        workspace_root,
        manifest_path.parent().unwrap_or(manifest_path),
    );

    let source_string = package.source.as_ref().map(|value| value.to_string());
    let source_kind = package_source_kind(source_string.as_deref(), is_workspace_member);
    // For local packages, source.value is the root directory relative to workspace;
    // for registry/git packages, source.value is the URL or registry source
    let source_value = match source_kind {
        PackageSourceKind::WorkspaceMember | PackageSourceKind::Path => Some(root_dir.clone()),
        _ => source_string.clone(),
    };

    let readme_path = package
        .readme
        .as_ref()
        .map(|value| rel_path(workspace_root, value.as_std_path()));

    // Only read README content for local packages to avoid network access
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

pub fn dependency_kind(kind: DependencyKind) -> DependencyKindDoc {
    match kind {
        DependencyKind::Normal => DependencyKindDoc::Normal,
        DependencyKind::Development => DependencyKindDoc::Dev,
        DependencyKind::Build => DependencyKindDoc::Build,
        // cargo_metadata may introduce new kinds; fall back to Normal
        _ => DependencyKindDoc::Normal,
    }
}

pub fn declared_dependencies(
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
                .map(|value| rel_path(workspace_root, value.as_std_path()));

            // Normalize path to always point to Cargo.toml;
            // dep.path may be the directory or the Cargo.toml itself
            let manifest_path = path.as_deref().map(|value| {
                if value.ends_with("Cargo.toml") {
                    value.to_string()
                } else {
                    format!("{}/Cargo.toml", value.trim_end_matches('/'))
                }
            });

            // Resolve the dependency's package_id by its manifest path;
            // returns None for non-local dependencies (registry/git)
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
                // target cfg from Cargo.toml uses a different syntax than
                // Rust #[cfg]; it is stored as an unparsed string flag
                // rather than a recursive CfgExpr
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
