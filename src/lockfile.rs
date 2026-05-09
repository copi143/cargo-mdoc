use cargo_metadata::Metadata;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use crate::model::{
    Diagnostic, DiagnosticLevel, LockEdge, LockEdgeKind, LockGraph, LockNode, PackageId,
};
use crate::util::{lock_node_id, parse_lock_dep, rel_path};

pub fn lock_graph(
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
