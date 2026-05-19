<script lang="ts">
  import type {
    Counts,
    Features,
    ItemDoc,
    ItemKind,
    PackageDoc,
  } from "../types";

  export let filteredItems: ItemDoc[] = [];
  export let counts: Counts = {};
  export let kindOptions: Array<"All" | ItemKind> = [];
  export let selectedPath = "";
  export let kindFilter: "All" | ItemKind = "All";
  export let query = "";
  export let view: "home" | "item" = "home";
  export let packages: PackageDoc[] = [];
  export let selectedPackageId = "";
  export let enabledFeatures: Features = {};

  export let onHome: () => void = () => {};
  export let onSelectItem: (item: ItemDoc) => void = () => {};
  export let onQueryChange: (value: string) => void = () => {};
  export let onKindChange: (value: "All" | ItemKind) => void = () => {};
  export let onPackageChange: (id: string) => void = () => {};
  export let onFeatureToggle: (feature: string) => void = () => {};
  export let selectedPackageName = "";
  export let featureNamesList: string[] = [];

  let buttonRefs: HTMLButtonElement[] = [];
  let sliderStyle = "";

  $: {
    const idx = kindOptions.indexOf(kindFilter);
    if (idx >= 0 && buttonRefs[idx]) {
      const btn = buttonRefs[idx];
      sliderStyle = `transform: translateX(${btn.offsetLeft}px); width: ${btn.offsetWidth}px;`;
    } else {
      sliderStyle = "opacity: 0;";
    }
  }

  const formatSource = (item: ItemDoc): string => {
    if (!item.source) return "Source unavailable";
    const { file, line_start, line_end } = item.source;
    const range =
      line_start === line_end ? `${line_start}` : `${line_start}-${line_end}`;
    return `${file}:${range}`;
  };
</script>

<aside class="panel list">
  <div class="panel-header">
    <div>
      <h2>Index</h2>
      <p class="panel-subtitle">{filteredItems.length} results</p>
    </div>
    <button class="ghost" on:click={onHome} disabled={view === "home"}>
      Home
    </button>
  </div>

  {#if packages.length > 0}
    <div class="crates">
      <div class="section-label">Crate</div>
      <div class="crate-pills">
        <button
          class="crate-pill"
          class:active={!selectedPackageId}
          on:click={() => onPackageChange("")}>All crates</button
        >
        {#each packages as pkg}
          <button
            class="crate-pill"
            class:active={(pkg.id ?? pkg.name) === selectedPackageId}
            on:click={() => onPackageChange(pkg.id ?? pkg.name)}
          >
            {pkg.name}
          </button>
        {/each}
      </div>
    </div>
  {/if}

  {#if featureNamesList.length > 0}
    <div class="features">
      <div class="section-label">Features — {selectedPackageName}</div>
      <div class="feature-pills">
        {#each featureNamesList as feat}
          <button
            class="feature-pill"
            class:on={enabledFeatures[feat] !== undefined}
            on:click={() => onFeatureToggle(feat)}
          >
            {feat}
          </button>
        {/each}
      </div>
    </div>
  {/if}

  <div class="search">
    <input
      type="search"
      placeholder="Search paths or docs"
      value={query}
      on:input={(event) => onQueryChange(event.currentTarget.value)}
      aria-label="Search"
    />
    <div>
      <div class="filters">
        <div class="filter-slider" style={sliderStyle}></div>
        {#each kindOptions as option, i}
          <button
            bind:this={buttonRefs[i]}
            class:active={kindFilter === option}
            on:click={() => onKindChange(option)}
          >
            {option}
            {#if option !== "All"}
              <span class="count">{counts[option] ?? 0}</span>
            {/if}
          </button>
        {/each}
      </div>
    </div>
  </div>

  <div class="list-items">
    {#if filteredItems.length === 0}
      <div class="empty">No matches. Try a broader search.</div>
    {:else}
      {#each filteredItems as item, index}
        <button
          class:selected={item.path === selectedPath}
          style={`--i: ${index}`}
          on:click={() => onSelectItem(item)}
        >
          <span class="row-main">
            <span class="path">{item.path}</span>
            <span class="source-line">{formatSource(item)}</span>
          </span>
          <span class={`kind kind-${item.kind.toLowerCase()}`}>{item.kind}</span
          >
        </button>
      {/each}
    {/if}
  </div>
</aside>
