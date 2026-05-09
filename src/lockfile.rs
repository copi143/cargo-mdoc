use cargo_metadata::Metadata;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::model::{
    Diagnostic, DiagnosticLevel, LockEdge, LockEdgeKind, LockNode, PackageId, ResolveGraph,
};

pub fn resolve_graph(
    _workspace_root: &Path,
    metadata: &Metadata,
    package_id_by_metadata: &HashMap<String, PackageId>,
    workspace_member_ids: &HashSet<PackageId>,
) -> (Option<ResolveGraph>, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();

    let resolve = match &metadata.resolve {
        Some(resolve) => resolve,
        None => {
            diagnostics.push(Diagnostic {
                level: DiagnosticLevel::Warning,
                code: "RESOLVE_GRAPH_MISSING".to_string(),
                message: "cargo metadata returned no resolve graph; resolve_graph omitted"
                    .to_string(),
                package_id: None,
                file: None,
                line: None,
            });

            return (None, diagnostics);
        }
    };

    let mut pkg_by_id = HashMap::new();
    for pkg in &metadata.packages {
        pkg_by_id.insert(pkg.id.clone(), pkg);
    }

    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for node in &resolve.nodes {
        let Some(pkg) = pkg_by_id.get(&node.id) else {
            diagnostics.push(Diagnostic {
                level: DiagnosticLevel::Warning,
                code: "RESOLVE_NODE_PACKAGE_MISSING".to_string(),
                message: format!("Package for resolve node not found: {}", node.id),
                package_id: None,
                file: None,
                line: None,
            });
            continue;
        };

        let source = pkg.source.as_ref().map(|s| s.to_string());
        let package_id = package_id_by_metadata.get(&node.id.to_string()).cloned();
        let workspace_member = package_id
            .as_ref()
            .map(|id| workspace_member_ids.contains(id))
            .unwrap_or(false);

        nodes.push(LockNode {
            id: node.id.to_string(),
            name: pkg.name.to_string(),
            version: pkg.version.to_string(),
            source,
            checksum: None,
            replace: None,
            workspace_member,
            package_id,
        });

        for dep in &node.deps {
            edges.push(LockEdge {
                from: node.id.to_string(),
                to: dep.pkg.to_string(),
                kind: LockEdgeKind::Unknown,
                req: None,
                optional: None,
                uses_default_features: None,
                features: Vec::new(),
            });
        }
    }

    nodes.sort_by(|a, b| {
        (a.name.as_str(), a.version.as_str(), a.source.as_deref()).cmp(&(
            b.name.as_str(),
            b.version.as_str(),
            b.source.as_deref(),
        ))
    });

    let roots = if let Some(root) = &resolve.root {
        vec![root.to_string()]
    } else {
        metadata
            .workspace_members
            .iter()
            .map(|id| id.to_string())
            .collect()
    };

    let graph = ResolveGraph {
        generated_by: Some("cargo_metadata.resolve".to_string()),
        nodes,
        edges,
        roots,
    };

    (Some(graph), diagnostics)
}
