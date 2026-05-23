<script lang="ts">
  import { onMount } from "svelte";
  import { l10nDerive } from "./l10n";
  import DependencyGraph from "./components/depgraph/DependencyGraph.svelte";
  import Topbar from "./components/layout/Topbar.svelte";
  import Sidebar from "./components/layout/Sidebar.svelte";
  import Home from "./pages/Home.svelte";
  import Item from "./pages/Item.svelte";
  import type {
    CfgExpr,
    Counts,
    Features,
    GeneratorInfo,
    ItemDoc,
    ItemKind,
    PackageDoc,
    PackageInfo,
    ResolveGraph,
  } from "./types";

  let loading = true;
  let error = "";
  let errorStatus = 0;

  $: l10n = $l10nDerive("$$_FILE_PATH_$$");
  let items: ItemDoc[] = [];
  let packageInfo: PackageInfo | null = null;
  let packages: PackageDoc[] = [];
  let resolveGraph: ResolveGraph | null = null;
  let generator: GeneratorInfo | null = null;
  let query = "";
  let kindFilter: "All" | ItemKind = "All";
  let selectedPackageId = "";
  let enabledFeatures: Features = {};
  let selectedPath = "";
  let view: "home" | "item" = "home";
  let graphOpen = false;

  const evaluateCfg = (cfg: CfgExpr, enabled: Features): boolean => {
    switch (cfg.kind) {
      case "True":
        return true;
      case "False":
        return false;
      case "Feature":
        return enabled[cfg.value] !== undefined;
      case "Flag":
        return true;
      case "All":
        return cfg.value.every((c) => evaluateCfg(c, enabled));
      case "Any":
        return cfg.value.some((c) => evaluateCfg(c, enabled));
      case "Not":
        return !evaluateCfg(cfg.value, enabled);
    }
  };

  const selectPackage = (id: string) => {
    selectedPackageId = id;
    if (id) {
      const pkg = packages.find((p) => (p.id ?? p.name) === id);
      enabledFeatures = pkg?.features ? { ...pkg.features } : {};
    } else {
      enabledFeatures = {};
    }
  };

  const toggleFeature = (feature: string) => {
    if (enabledFeatures[feature] !== undefined) {
      const next = { ...enabledFeatures };
      delete next[feature];
      enabledFeatures = next;
    } else {
      enabledFeatures = { ...enabledFeatures, [feature]: [] };
    }
  };

  const kindOptions: Array<"All" | ItemKind> = [
    "All",
    "Function",
    "Struct",
    "Enum",
    "Trait",
    "Module",
  ];

  const normalize = (value: string) => (value || "").toLowerCase();

  const formatCfg = (cfg?: CfgExpr | null): string => {
    if (!cfg) return "true";

    switch (cfg.kind) {
      case "True":
        return "true";
      case "False":
        return "false";
      case "Feature":
        return `feature="${cfg.value}"`;
      case "Flag":
        return cfg.value;
      case "All":
        return `all(${(cfg.value || []).map(formatCfg).join(", ")})`;
      case "Any":
        return `any(${(cfg.value || []).map(formatCfg).join(", ")})`;
      case "Not":
        return `not(${formatCfg(cfg.value)})`;
      default:
        return "unknown";
    }
  };

  const onHashChange = () => {
    const hash = window.location.hash;

    if (hash.startsWith("#/item/")) {
      selectedPath = decodeURIComponent(hash.slice(7));
      view = "item";
    } else if (hash.startsWith("#/home")) {
      selectedPath = "";
      view = "home";
    } else {
      selectedPath = "";
      view = "home";
    }
  };

  const selectItem = (item?: ItemDoc | null) => {
    if (!item) return;
    selectedPath = item.path;
    view = "item";
    window.location.hash = `#/item/${encodeURIComponent(item.path)}`;
  };

  const goHome = () => {
    window.location.hash = "#/home";
    selectedPath = "";
    view = "home";
  };

  const openGraph = () => {
    if (!resolveGraph) return;
    graphOpen = true;
  };

  const closeGraph = () => {
    graphOpen = false;
  };

  onMount(() => {
    const load = async (): Promise<void> => {
      try {
        const params = new URLSearchParams(window.location.search);
        const dataUrl = params.get("data") || "/mdoc.json";
        const response = await fetch(dataUrl, { cache: "no-store" });

        if (!response.ok) {
          errorStatus = response.status;
          throw new Error("failed_to_load");
        }

        const data = await response.json();

        if (Array.isArray(data)) {
          packageInfo = null;
          packages = [];
          resolveGraph = null;
          items = data;
        } else {
          packageInfo = data.package || null;
          packages = Array.isArray(data.packages) ? data.packages : [];
          if (
            packageInfo &&
            !packages.find((p) => p.name === packageInfo?.name)
          ) {
            packages = [packageInfo as PackageDoc, ...packages];
          }
          generator = data.generator || null;
          resolveGraph = data.resolve_graph || data.lock_graph || null;
          items = Array.isArray(data.items) ? data.items : [];
        }

        onHashChange();
      } catch {
        error = "failed_to_load";
      } finally {
        loading = false;
      }
    };

    window.addEventListener("hashchange", onHashChange);
    void load();

    return () => window.removeEventListener("hashchange", onHashChange);
  });

  $: workspacePackages = packages.filter(
    (p) => p.is_workspace_member !== false,
  );

  $: selectedPackage = workspacePackages.find(
    (p) => (p.id ?? p.name) === selectedPackageId,
  );
  $: featureNamesList = selectedPackage?.features
    ? Object.keys(selectedPackage.features).sort()
    : [];
  $: selectedPackageName = selectedPackage?.name ?? "";

  $: crateItems = !selectedPackageId
    ? items
    : items.filter((item) => item.package_id === selectedPackageId);

  $: cfgFiltered = selectedPackageId
    ? crateItems.filter((item) => evaluateCfg(item.cfg, enabledFeatures))
    : crateItems;

  $: filteredItems = cfgFiltered
    .filter((item) => {
      if (kindFilter !== "All" && item.kind !== kindFilter) return false;
      const q = normalize(query.trim());
      if (!q) return true;
      return (
        normalize(item.path).includes(q) || normalize(item.docs).includes(q)
      );
    })
    .sort((a, b) => a.path.localeCompare(b.path));

  $: selectedItem = items.find((item) => item.path === selectedPath) || null;

  $: counts = crateItems.reduce<Counts>((acc, item) => {
    acc[item.kind] = (acc[item.kind] || 0) + 1;
    return acc;
  }, {} as Counts);

  const moduleDepth = (path: string) => path.split("::").length;

  $: topModules = items
    .filter((item) => item.kind === "Module" && moduleDepth(item.path) === 1)
    .sort((a, b) => a.path.localeCompare(b.path))
    .slice(0, 8);

  $: prominentItems = items
    .filter((item) => item.kind !== "Module")
    .sort((a, b) => a.path.localeCompare(b.path))
    .slice(0, 8);
</script>

<div class="shell">
  <Topbar
    {packageInfo}
    {items}
    {generator}
    onOpenGraph={openGraph}
    graphAvailable={Boolean(resolveGraph)}
  />

  {#if error}
    <div class="state error">
      {#if errorStatus}
        {l10n("failed_to_load_status", { status: errorStatus })}
      {:else}
        {l10n("failed_to_load")}
      {/if}
    </div>
  {:else if loading}
    <div class="state loading">{l10n("loading_docs")}</div>
  {/if}

  <main class="content" aria-busy={loading}>
    <Sidebar
      {filteredItems}
      {counts}
      {kindOptions}
      {selectedPath}
      {kindFilter}
      {query}
      {view}
      packages={workspacePackages}
      {selectedPackageId}
      {enabledFeatures}
      {selectedPackageName}
      {featureNamesList}
      onHome={goHome}
      onSelectItem={selectItem}
      onQueryChange={(value) => (query = value)}
      onKindChange={(value) => (kindFilter = value)}
      onPackageChange={(id) => selectPackage(id)}
      onFeatureToggle={(feat) => toggleFeature(feat)}
    />

    <section class="panel detail">
      {#if view === "home"}
        <Home
          {packageInfo}
          {items}
          {counts}
          {topModules}
          {prominentItems}
          {resolveGraph}
          onSelectItem={selectItem}
        />
      {:else}
        <Item {selectedItem} {formatCfg} />
      {/if}
    </section>
  </main>

  {#if graphOpen}
    <DependencyGraph {resolveGraph} onClose={closeGraph} />
  {/if}
</div>
