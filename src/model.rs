use serde::Serialize;
use std::collections::BTreeMap;

// CfgExpr uses #[serde(tag = "kind", content = "value")] so JSON output
// is a tagged union like {"kind": "All", "value": [...]}.
// This is the project's own IR, decoupled from ra_ap_cfg's representation,
// to avoid leaking library types into the stable output format.
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

// Only basic item kinds are tracked in the MVP;
// TypeAlias, Constant, Static, and Union are intentionally excluded
// to keep the analysis scope manageable.
#[derive(Debug, Clone, Serialize)]
pub enum ItemKind {
    Function,
    Struct,
    Enum,
    Trait,
    Module,
    Macro,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceLocation {
    pub file: String,
    pub line_start: usize,
    pub line_end: usize,
    // Column offsets are not yet tracked; they require byte-to-column mapping
    // which is non-trivial for multi-byte characters
    pub col_start: Option<usize>,
    pub col_end: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GeneratorInfo {
    pub name: String,
    pub version: String,
    // rustc and cargo versions are collected by the build system metadata;
    // intentionally omitted in MVP to avoid running external tooling
    pub rustc_version: Option<String>,
    pub cargo_version: Option<String>,
    pub commit_id: Option<String>,
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

// PackageId is a string with a structured format:
// pkg:<name>@<version>|<source_key>|<manifest_path>
// This is globally unique across workspace + dependency packages.
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
    // VCS metadata is captured from Cargo.lock but not actively used;
    // stored for forward compatibility with workspace audit tools
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
    /// Additional external documentation files discovered in the crate root
    /// (e.g. CHANGELOG.md, LICENSE, CONTRIBUTING.md) beyond Cargo.toml's
    /// declared `readme`.
    pub documents: Vec<ExternalDoc>,
    pub targets: Vec<TargetDoc>,
}

/// An external documentation file found alongside a crate's source.
#[derive(Debug, Clone, Serialize)]
pub struct ExternalDoc {
    /// Relative path from the workspace root, e.g. "CHANGELOG.md"
    pub path: String,
    /// File name, e.g. "CHANGELOG.md"
    pub name: String,
    /// File content (omitted for non-local or overly large files)
    pub content: Option<String>,
    /// Detected kind hint from file name
    pub kind: ExternalDocKind,
}

/// Hint about the role of an external documentation file.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalDocKind {
    Readme,
    Changelog,
    License,
    Contributing,
    CodeOfConduct,
    Other,
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
    // Renamed dependencies (e.g., `tokio = { package = "tokio-util" }`)
    pub package_rename: Option<String>,
    pub req: Option<String>,
    pub kind: DependencyKindDoc,
    pub optional: bool,
    pub uses_default_features: bool,
    pub features: Vec<String>,
    // target_cfg stores the raw cfg string from Cargo.toml;
    // it is stored as an unparsed Flag rather than a recursively parsed CfgExpr
    // because Cargo's target specification syntax differs from Rust #[cfg] syntax
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
pub struct ResolveGraph {
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
    // path is the full qualified path: e.g. "tokio::net::TcpListener"
    pub path: String,
    pub name: String,
    pub module_path: String,
    pub kind: ItemKind,
    pub visibility: Visibility,
    pub docs: String,
    pub cfg: CfgExpr,
    pub source: SourceLocation,
    pub source_code: String,
    // signature captures function/item type signatures from AST,
    // deprecated parses #[deprecated(...)] attributes,
    // export_name is the binary symbol export name from #[export_name] / #[no_mangle].
    pub signature: Option<String>,
    pub attributes: Option<Vec<String>>,
    pub deprecated: Option<Deprecation>,
    pub export_name: Option<String>,
    /// True when the item is a `macro_rules!` definition marked `#[macro_export]`.
    pub is_macro_export: bool,
    /// True when the item was produced by expanding a macro call.
    pub is_macro_expanded: bool,
    /// Name of the macro the item was expanded from, if any.
    pub expanded_from: Option<String>,
    pub impls: Vec<ImplDoc>,
}

/// An item that a macro expansion produced.
#[derive(Debug, Clone, Serialize)]
pub struct ExpandedItemDoc {
    pub name: String,
    pub kind: ItemKind,
    pub docs: String,
    pub signature: Option<String>,
    pub source_code: String,
}

/// A macro invocation (e.g. `my_macro!(...)` or `foo::bar!(...)`).
/// Macro calls are not items themselves, but are collected as call sites
/// so consumers can see which macros are used and where.
#[derive(Debug, Clone, Serialize)]
pub struct MacroCallDoc {
    pub id: String,
    pub package_id: PackageId,
    /// Resolved macro path, e.g. "crate::my_macro" or "serde::Serialize".
    pub path: String,
    /// Short name, e.g. "my_macro".
    pub name: String,
    pub module_path: String,
    pub cfg: CfgExpr,
    pub source: SourceLocation,
    pub source_code: String,
    pub attributes: Option<Vec<String>>,
    /// The expanded token stream rendered as Rust source, when the macro
    /// could be expanded against a definition found in the workspace.
    pub expanded_code: Option<String>,
    /// Items the expansion produced.
    pub expanded_items: Vec<ExpandedItemDoc>,
    /// Index of the macro rule that matched the call.
    pub matched_arm: Option<u32>,
    /// Description of the failure, when the expansion was only partial.
    pub expansion_error: Option<String>,
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
    // code is a stable machine-readable identifier like "CRATE_ROOT_MISSING"
    pub code: String,
    pub message: String,
    pub package_id: Option<PackageId>,
    pub file: Option<String>,
    pub line: Option<usize>,
}

// A method or associated item inside an impl block
#[derive(Debug, Clone, Serialize)]
pub struct ImplItemDoc {
    pub name: String,
    pub kind: ItemKind,
    pub docs: String,
    pub visibility: Visibility,
    pub cfg: CfgExpr,
    pub source: SourceLocation,
    pub source_code: String,
    pub signature: Option<String>,
    pub deprecated: Option<Deprecation>,
    pub export_name: Option<String>,
}

// An impl block associated with a type
#[derive(Debug, Clone, Serialize)]
pub struct ImplDoc {
    pub id: String,
    pub package_id: PackageId,
    /// Full path of the type the impl is for (e.g. "my_crate::MyStruct")
    pub target_path: String,
    /// Present for trait impls like `impl Display for MyStruct`
    pub trait_name: Option<String>,
    pub cfg: CfgExpr,
    pub source: SourceLocation,
    pub source_code: String,
    pub items: Vec<ImplItemDoc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InternalDoc {
    pub format: String,
    pub generated_at: String,
    pub generator: GeneratorInfo,
    pub workspace: WorkspaceInfo,
    pub packages: Vec<PackageDoc>,
    pub declared_dependencies: Vec<DeclaredDependencyEdge>,
    pub resolve_graph: Option<ResolveGraph>,
    pub items: Vec<ItemDoc>,
    /// Macro call sites discovered while walking the crate's sources.
    pub macro_calls: Vec<MacroCallDoc>,
    pub diagnostics: Vec<Diagnostic>,
}
