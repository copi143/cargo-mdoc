import type { CfgExpr, ItemDoc, ItemKind, Features } from "../types";
import { evaluateCfg } from "./cfg";

export type FilterState = {
  query: string;
  kindFilter: "All" | ItemKind;
  selectedPackageId: string;
  enabledFeatures: Features;
};

export const filterItems = (
  items: ItemDoc[],
  state: FilterState,
  allItems: ItemDoc[],
): ItemDoc[] => {
  const { query, kindFilter, selectedPackageId, enabledFeatures } = state;

  const crateItems = !selectedPackageId
    ? allItems
    : allItems.filter((item) => item.package_id === selectedPackageId);

  const cfgFiltered = selectedPackageId
    ? crateItems.filter((item) => evaluateCfg(item.cfg, enabledFeatures))
    : crateItems;

  const q = query.trim().toLowerCase();

  return cfgFiltered
    .filter((item) => {
      if (kindFilter !== "All" && item.kind !== kindFilter) return false;
      if (!q) return true;
      return (
        item.path.toLowerCase().includes(q) || item.docs.toLowerCase().includes(q)
      );
    })
    .sort((a, b) => a.path.localeCompare(b.path));
};
