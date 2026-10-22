import { derived } from "svelte/store";
import type { Counts, MacroCallDoc } from "../types";
import {
  itemsStore,
  macroCallsStore,
  packagesStore,
  queryStore,
  kindFilterStore,
  selectedPackageIdStore,
  enabledFeaturesStore,
  selectedPathStore,
} from "./store";
import { filterItems } from "./filter";
import { moduleDepth } from "./path";

export const workspacePackages = derived(packagesStore, ($packages) =>
  $packages.filter((p) => p.is_workspace_member !== false),
);

export const selectedPackage = derived(
  [workspacePackages, selectedPackageIdStore],
  ([$packages, $id]) => $packages.find((p) => (p.id ?? p.name) === $id) ?? null,
);

export const featureNamesList = derived(selectedPackage, ($pkg) =>
  $pkg?.features ? Object.keys($pkg.features).sort() : [],
);

export const selectedPackageName = derived(selectedPackage, ($pkg) => $pkg?.name ?? "");

export const crateItems = derived([itemsStore, selectedPackageIdStore], ([$items, $id]) =>
  !$id ? $items : $items.filter((item) => item.package_id === $id),
);

export const filteredItems = derived(
  [itemsStore, queryStore, kindFilterStore, selectedPackageIdStore, enabledFeaturesStore],
  ([$items, $query, $kindFilter, $selectedPackageId, $enabledFeatures]) =>
    filterItems(
      $items,
      {
        query: $query,
        kindFilter: $kindFilter,
        selectedPackageId: $selectedPackageId,
        enabledFeatures: $enabledFeatures,
      },
      $items,
    ),
);

export const selectedItem = derived(
  [itemsStore, selectedPathStore],
  ([$items, $path]) => $items.find((item) => item.path === $path) ?? null,
);

export const counts = derived(crateItems, ($crateItems) =>
  $crateItems.reduce<Counts>((acc, item) => {
    acc[item.kind] = (acc[item.kind] || 0) + 1;
    return acc;
  }, {} as Counts),
);

export const topModules = derived(itemsStore, ($items) =>
  $items
    .filter((item) => item.kind === "Module" && moduleDepth(item.path) === 1)
    .sort((a, b) => a.path.localeCompare(b.path))
    .slice(0, 8),
);

export const prominentItems = derived(itemsStore, ($items) =>
  $items
    .filter((item) => item.kind !== "Module")
    .sort((a, b) => a.path.localeCompare(b.path))
    .slice(0, 8),
);

// Indexes macro call sites by the macro name they invoke, so a macro
// definition can be matched with every place it is used.
export const macroCallsByName = derived(macroCallsStore, ($calls) => {
  const map = new Map<string, MacroCallDoc[]>();
  for (const call of $calls) {
    const list = map.get(call.name);
    if (list) {
      list.push(call);
    } else {
      map.set(call.name, [call]);
    }
  }
  return map;
});

// Call sites of the currently selected macro; empty for non-macro items.
export const selectedMacroCalls = derived(
  [selectedItem, macroCallsByName],
  ([$item, $calls]) => {
    if (!$item || $item.kind !== "Macro" || !$item.name) return [];
    const calls = $calls.get($item.name) ?? [];
    // Call sites inside the defining crate come first, then the ones
    // coming from other crates (typical for `#[macro_export]` macros).
    return [...calls].sort((a, b) => {
      const sameA = a.package_id === $item.package_id ? 0 : 1;
      const sameB = b.package_id === $item.package_id ? 0 : 1;
      if (sameA !== sameB) return sameA - sameB;
      return (
        a.module_path.localeCompare(b.module_path) ||
        a.source.file.localeCompare(b.source.file) ||
        a.source.line_start - b.source.line_start
      );
    });
  },
);
