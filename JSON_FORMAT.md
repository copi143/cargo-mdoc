# cargo-mdoc Internal JSON Format

This document describes the internal JSON emitted by `cargo-mdoc`. The format is designed to
support workspace member coverage, local path dependencies, full Cargo.lock graph capture, and
precise item-to-package attribution.

## Top-Level Shape

```json
{
  "format": "cargo-mdoc-internal/1.0",
  "generated_at": "2026-05-09T12:00:00Z",
  "generator": { "name": "cargo-mdoc", "version": "0.8.0" },
  "workspace": {},
  "packages": [],
  "declared_dependencies": [],
  "resolve_graph": {},
  "items": [],
  "macro_calls": [],
  "diagnostics": []
}
```

## Workspace

```json
{
  "root_dir": ".",
  "root_manifest_path": "Cargo.toml",
  "target_directory": "target",
  "resolver": "2",
  "members": ["pkg:app@0.1.0|workspace|Cargo.toml"],
  "default_members": ["pkg:app@0.1.0|workspace|Cargo.toml"]
}
```

## Packages

```json
{
  "id": "pkg:app@0.1.0|workspace|Cargo.toml",
  "name": "app",
  "version": "0.1.0",
  "edition": "2021",
  "manifest_path": "Cargo.toml",
  "root_dir": ".",
  "source": { "kind": "workspace_member", "value": "." },
  "is_workspace_member": true,
  "features": {},
  "targets": [
    { "name": "app", "kind": ["bin"], "src_path": "src/main.rs", "doc": true, "test": true }
  ]
}
```

`packages[].id` is globally unique and should be used as the stable reference in other sections.

## Declared Dependencies

```json
{
  "from_package_id": "pkg:app@0.1.0|workspace|Cargo.toml",
  "dep_name": "tokio",
  "to_package_id": "pkg:tokio@1.0.0|path|tokio/Cargo.toml",
  "req": "^1",
  "kind": "normal",
  "optional": false,
  "uses_default_features": true,
  "features": ["rt"],
  "target_cfg": { "kind": "Flag", "value": "cfg(unix)" },
  "path": "tokio"
}
```

`to_package_id` is set only when the dependency resolves to a local or workspace package.

## Resolve Graph

```json
{
  "nodes": [
    {
      "id": "crate:tokio@1.0.0|registry+https://github.com/rust-lang/crates.io-index",
      "name": "tokio",
      "version": "1.0.0",
      "source": "registry+https://github.com/rust-lang/crates.io-index",
      "checksum": "...",
      "workspace_member": false,
      "package_id": null
    }
  ],
  "edges": [
    {
      "from": "crate:app@0.1.0|local",
      "to": "crate:tokio@1.0.0|registry+https://github.com/rust-lang/crates.io-index",
      "kind": "unknown",
      "features": []
    }
  ],
  "roots": ["crate:app@0.1.0|local"]
}
```

The resolve graph is parsed directly from Cargo metadata. Fields not present in the metadata are
left as `null` or `unknown`.

## Items

```json
{
  "id": "item:pkg:app@0.1.0|workspace|Cargo.toml:app::main:10",
  "package_id": "pkg:app@0.1.0|workspace|Cargo.toml",
  "path": "app::main",
  "name": "main",
  "module_path": "app",
  "kind": "Function",
  "visibility": "public",
  "docs": "",
  "cfg": { "kind": "True" },
  "source": { "file": "src/main.rs", "line_start": 10, "line_end": 20 },
  "source_code": "fn main() { ... }"
}
```

`items[].package_id` always references a valid entry from `packages[].id`.

`kind` is one of `Function`, `Struct`, `Enum`, `Trait`, `Module` or `Macro`. Items produced by a
macro expansion are reported as regular items with `is_macro_expanded` set to `true` and
`expanded_from` naming the macro they come from. A `macro_rules!` definition marked with
`#[macro_export]` has `is_macro_export` set to `true`.

## Macro Calls

Every macro invocation is reported as a call site:

```json
{
  "id": "macro_call:pkg:app@0.1.0|workspace|Cargo.toml:double:12",
  "package_id": "pkg:app@0.1.0|workspace|Cargo.toml",
  "path": "double",
  "name": "double",
  "module_path": "app",
  "cfg": { "kind": "True" },
  "source": { "file": "src/lib.rs", "line_start": 12, "line_end": 12 },
  "source_code": "double!(21)",
  "attributes": [],
  "expanded_code": "21 + 21",
  "expanded_items": [],
  "matched_arm": 0,
  "expansion_error": null
}
```

`expanded_code` holds the expanded token stream, re-formatted into a readable, rustfmt-like layout.
It is present when the macro could be expanded against a `macro_rules!` definition found in the
workspace; `expanded_items` lists the items the expansion defines, and `matched_arm` is the index
of the macro rule that matched. `expansion_error` describes why an expansion was only partial.
Expansion is token based: hygiene rules and macros defined in other crates are not modelled.

## CfgExpr

`cfg` uses a tagged format:

```json
{ "kind": "All", "value": [ { "kind": "Feature", "value": "rt" } ] }
```

Supported kinds are: `True`, `False`, `Feature`, `Flag`, `All`, `Any`, `Not`.

## Diagnostics

Diagnostics report partial failures (for example, missing lockfiles). Each entry includes a
`level`, stable `code`, and optional location or package context.
