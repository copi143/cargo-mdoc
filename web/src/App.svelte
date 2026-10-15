<script lang="ts">
  import { onMount } from "svelte";
  import { locale, l10nDerive } from "./l10n";
  import Topbar from "./components/layout/Topbar.svelte";
  import Sidebar from "./components/layout/Sidebar.svelte";
  import Home from "./pages/Home.svelte";
  import Item from "./pages/Item.svelte";
  import DependencyGraph from "./components/depgraph/DependencyGraph.svelte";
  import {
    itemsStore,
    packagesStore,
    resolveGraphStore,
    generatorStore,
    packageInfoStore,
    queryStore,
    kindFilterStore,
    selectedPackageIdStore,
    enabledFeaturesStore,
    selectedPathStore,
    viewStore,
    graphOpenStore,
    loadingStore,
    errorStore,
  } from "./lib/store";
  import type {
  GeneratorInfo,
  ItemDoc,
  ItemKind,
  PackageDoc,
  ResolveGraph,
} from "./types";

  $: l10n = $l10nDerive("$$_FILE_PATH_$$");
  $: document.documentElement.lang = $locale;

  const selectPackage = (id: string) => {
    selectedPackageIdStore.set(id);
    if (id) {
      const pkg = $packagesStore.find((p) => (p.id ?? p.name) === id);
      enabledFeaturesStore.set(pkg?.features ? { ...pkg.features } : {});
    } else {
      enabledFeaturesStore.set({});
    }
  };

  const toggleFeature = (feature: string) => {
    enabledFeaturesStore.update((next) => {
      if (next[feature] !== undefined) {
        const copy = { ...next };
        delete copy[feature];
        return copy;
      }
      return { ...next, [feature]: [] };
    });
  };

  const kindOptions: Array<"All" | ItemKind> = [
    "All",
    "Function",
    "Struct",
    "Enum",
    "Trait",
    "Module",
  ];

  const onHashChange = () => {
    const hash = window.location.hash;
    if (hash.startsWith("#/item/")) {
      selectedPathStore.set(decodeURIComponent(hash.slice(7)));
      viewStore.set("item");
    } else if (hash.startsWith("#/home")) {
      selectedPathStore.set("");
      viewStore.set("home");
    } else {
      selectedPathStore.set("");
      viewStore.set("home");
    }
  };

  const selectItem = (item?: ItemDoc | null) => {
    if (!item) return;
    selectedPathStore.set(item.path);
    viewStore.set("item");
    window.location.hash = `#/item/${encodeURIComponent(item.path)}`;
  };

  const goHome = () => {
    window.location.hash = "#/home";
    selectedPathStore.set("");
    viewStore.set("home");
  };

  const openGraph = () => {
    if (!$resolveGraphStore) return;
    graphOpenStore.set(true);
  };

  const closeGraph = () => {
    graphOpenStore.set(false);
  };

  onMount(() => {
    const load = async (): Promise<void> => {
      try {
        const params = new URLSearchParams(window.location.search);
        const dataUrl = params.get("data") || "/mdoc.json";
        const response = await fetch(dataUrl, { cache: "no-store" });

        if (!response.ok) {
          errorStore.set({ message: "failed_to_load", status: response.status });
          throw new Error("failed_to_load");
        }

        const data = (await response.json()) as {
          package?: PackageDoc | null;
          packages?: PackageDoc[];
          generator?: GeneratorInfo | null;
          resolve_graph?: ResolveGraph | null;
          lock_graph?: ResolveGraph | null;
          items?: ItemDoc[];
        };

        if (Array.isArray(data)) {
          packageInfoStore.set(null);
          packagesStore.set([]);
          resolveGraphStore.set(null);
          itemsStore.set(data);
        } else {
          packageInfoStore.set(data.package ?? null);
          const pkgs = Array.isArray(data.packages) ? data.packages : [];
          if (
            data.package &&
            !pkgs.find((p) => p.name === data.package?.name)
          ) {
            packagesStore.set([data.package as PackageDoc, ...pkgs]);
          } else {
            packagesStore.set(pkgs);
          }
          generatorStore.set(data.generator ?? null);
          resolveGraphStore.set(
            data.resolve_graph || data.lock_graph || null,
          );
          itemsStore.set(Array.isArray(data.items) ? data.items : []);
        }

        onHashChange();
      } catch {
        errorStore.set({ message: "failed_to_load", status: 0 });
      } finally {
        loadingStore.set(false);
      }
    };

    window.addEventListener("hashchange", onHashChange);
    void load();

    return () => window.removeEventListener("hashchange", onHashChange);
  });
</script>

<div class="shell">
  <Topbar onOpenGraph={openGraph} />

  {#if $errorStore.message}
    <div class="state error">
      {#if $errorStore.status}
        {l10n("failed_to_load_status", { status: $errorStore.status })}
      {:else}
        {l10n("failed_to_load")}
      {/if}
    </div>
  {:else if $loadingStore}
    <div class="state loading">{l10n("loading_docs")}</div>
  {/if}

  <main class="content" aria-busy={$loadingStore}>
    <Sidebar
      onHome={goHome}
      onSelectItem={selectItem}
      onQueryChange={(value) => queryStore.set(value)}
      onKindChange={(value) => kindFilterStore.set(value)}
      onPackageChange={selectPackage}
      onFeatureToggle={toggleFeature}
    />

    <section class="panel detail">
      {#if $viewStore === "home"}
        <Home onSelectItem={selectItem} />
      {:else}
        <Item />
      {/if}
    </section>
  </main>

  {#if $graphOpenStore}
    <DependencyGraph resolveGraph={$resolveGraphStore} onClose={closeGraph} />
  {/if}
</div>
