use serde::Serialize;
use std::collections::BTreeMap;

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
