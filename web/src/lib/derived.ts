import { derived } from "svelte/store";
import type { Counts } from "../types";
import {
  itemsStore,
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
