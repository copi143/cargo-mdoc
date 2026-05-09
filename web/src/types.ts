export type ItemKind = "Function" | "Struct" | "Enum" | "Trait" | "Module";

export type SourceLocation = {
  file: string;
  line_start: number;
  line_end: number;
};

export type CfgExpr =
  | { kind: "True" }
  | { kind: "False" }
  | { kind: "Feature"; value: string }
  | { kind: "Flag"; value: string }
  | { kind: "All"; value: CfgExpr[] }
  | { kind: "Any"; value: CfgExpr[] }
  | { kind: "Not"; value: CfgExpr };

export type ItemDoc = {
  path: string;
  docs: string;
  cfg: CfgExpr;
  kind: ItemKind;
  source?: SourceLocation;
  source_code?: string;
};

export type PackageInfo = {
  name: string;
  version?: string;
  edition?: string;
  description?: string;
  license?: string;
  readme_path?: string;
  readme?: string;
};

export type Counts = Partial<Record<ItemKind, number>>;

export type ResolveGraphNode = {
  id: string;
  name: string;
  version: string;
  source?: string | null;
  checksum?: string | null;
  replace?: string | null;
  workspace_member: boolean;
  package_id?: string | null;
};

export type ResolveGraphEdge = {
  from: string;
  to: string;
  kind: "normal" | "dev" | "build" | "unknown";
  req?: string | null;
  optional?: boolean | null;
  uses_default_features?: boolean | null;
  features: string[];
};

export type ResolveGraph = {
  generated_by?: string | null;
  nodes: ResolveGraphNode[];
  edges: ResolveGraphEdge[];
  roots: string[];
};
