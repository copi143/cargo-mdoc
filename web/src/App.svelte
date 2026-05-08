<script>
  import { onMount } from "svelte";
  import Topbar from "./components/Topbar.svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import Home from "./pages/Home.svelte";
  import Item from "./pages/Item.svelte";

  let loading = true;
  let error = "";
  let items = [];
  let packageInfo = null;
  let query = "";
  let kindFilter = "all";
  let selectedPath = "";
  let view = "home";

  const kindOptions = ["all", "Function", "Struct", "Enum", "Trait", "Module"];

  const normalize = (value) => (value || "").toLowerCase();

  const formatCfg = (cfg) => {
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

  const selectItem = (item) => {
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

  onMount(async () => {
    try {
      const response = await fetch("/mdoc-ra.json", { cache: "no-store" });

      if (!response.ok) {
        throw new Error(`Failed to load mdoc-ra.json (${response.status})`);
      }

      const data = await response.json();

      packageInfo = data.package || null;
      items = Array.isArray(data.items) ? data.items : [];

      onHashChange();

      window.addEventListener("hashchange", onHashChange);
    } catch (err) {
      error = err instanceof Error ? err.message : "Failed to load data";
    } finally {
      loading = false;
    }

    return () => window.removeEventListener("hashchange", onHashChange);
  });

  $: filteredItems = items
    .filter((item) => {
      if (kindFilter !== "all" && item.kind !== kindFilter) return false;
      const q = normalize(query.trim());
      if (!q) return true;
      return (
        normalize(item.path).includes(q) || normalize(item.docs).includes(q)
      );
    })
    .sort((a, b) => a.path.localeCompare(b.path));

  $: selectedItem = items.find((item) => item.path === selectedPath) || null;

  $: counts = items.reduce((acc, item) => {
    acc[item.kind] = (acc[item.kind] || 0) + 1;
    return acc;
  }, {});

  const moduleDepth = (path) => path.split("::").length;

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
  <Topbar {packageInfo} {items} {counts} />

  {#if error}
    <div class="state error">{error}</div>
  {:else if loading}
    <div class="state loading">Loading documentation index...</div>
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
      onHome={goHome}
      onSelectItem={selectItem}
      onQueryChange={(value) => (query = value)}
      onKindChange={(value) => (kindFilter = value)}
    />

    <section class="panel detail">
      {#if view === "home"}
        <Home
          {items}
          {counts}
          {topModules}
          {prominentItems}
          onSelectItem={selectItem}
        />
      {:else}
        <Item {selectedItem} {formatCfg} />
      {/if}
    </section>
  </main>
</div>
