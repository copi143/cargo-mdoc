use anyhow::{Context, Result};
use chrono::{SecondsFormat, Utc};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

mod cfg_expr;
mod cli;
mod collector;
mod lockfile;
mod metadata_build;
mod model;
mod util;

use cli::parse_args;
use collector::RaCollector;
use lockfile::lock_graph;
use metadata_build::{
    declared_dependencies, load_metadata, package_doc, package_source_kind, workspace_resolver,
};
use model::{
    Diagnostic, DiagnosticLevel, GeneratorInfo, InternalDoc, PackageId, PackageSourceKind,
    WorkspaceInfo,
};
use util::{package_id_for, rel_path};

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
    let workspace_root = PathBuf::from(metadata.workspace_root.as_std_path());
    let resolver = workspace_resolver(&workspace_root);

    let mut package_id_by_metadata: HashMap<String, PackageId> = HashMap::new();
    let mut package_id_by_manifest: HashMap<String, PackageId> = HashMap::new();
    let mut package_id_by_lock_key: HashMap<String, PackageId> = HashMap::new();

    let workspace_members: HashSet<String> = metadata
        .workspace_members
        .iter()
        .map(|value| value.to_string())
        .collect();

    let mut packages = Vec::new();

    for package in &metadata.packages {
        let manifest_rel = rel_path(&workspace_root, package.manifest_path.as_std_path());
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

        let manifest_path = package.manifest_path.as_std_path();
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
            metadata.target_directory.as_std_path(),
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
