import { writable } from "svelte/store";
import type {
  ItemDoc,
  ItemKind,
  PackageDoc,
  PackageInfo,
  ResolveGraph,
  GeneratorInfo,
  Features,
} from "../types";

export const itemsStore = writable<ItemDoc[]>([]);
export const packagesStore = writable<PackageDoc[]>([]);
export const resolveGraphStore = writable<ResolveGraph | null>(null);
export const generatorStore = writable<GeneratorInfo | null>(null);
export const packageInfoStore = writable<PackageInfo | null>(null);

export const queryStore = writable("");
export const kindFilterStore = writable<"All" | ItemKind>("All");
export const selectedPackageIdStore = writable("");
export const enabledFeaturesStore = writable<Features>({});
export const selectedPathStore = writable("");
export const viewStore = writable<"home" | "item">("home");
export const graphOpenStore = writable(false);

export const loadingStore = writable(true);
export const errorStore = writable<{ message: string; status: number }>({
  message: "",
  status: 0,
});
