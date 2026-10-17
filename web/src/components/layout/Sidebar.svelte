<script lang="ts">
  import type { ItemDoc, ItemKind, PackageDoc } from "../../types";
  import { l10nDerive } from "../../l10n";
  import {
    selectedPathStore,
    kindFilterStore,
    queryStore,
    viewStore,
    selectedPackageIdStore,
    enabledFeaturesStore,
  } from "../../lib/store";
  import {
    filteredItems as filteredItemsStore,
    counts as countsStore,
    workspacePackages as workspacePackagesStore,
    selectedPackageName as selectedPackageNameStore,
    featureNamesList as featureNamesListStore,
  } from "../../lib/derived";
  import ListItem from "./ListItem.svelte";

  export let onHome: () => void = () => {};
  export let onSelectItem: (item: ItemDoc) => void = () => {};
  export let onQueryChange: (value: string) => void = () => {};
  export let onKindChange: (value: "All" | ItemKind) => void = () => {};
  export let onPackageChange: (id: string) => void = () => {};
  export let onFeatureToggle: (feature: string) => void = () => {};

  let buttonRefs: HTMLButtonElement[] = [];
  let sliderStyle = "";
  let currentPage = 0;
  const pageSize = 100;

  let listItemsEl: HTMLDivElement;

  function scrollTo(page: number) {
    currentPage = page;
    listItemsEl.scrollTo(0, 0);
  }

  const kindOptions: Array<"All" | ItemKind> = [
    "All",
    "Function",
    "Struct",
    "Enum",
    "Trait",
    "Module",
    "Macro",
  ];

  $: l10n = $l10nDerive("$$_FILE_PATH_$$");
  $: totalPages = Math.max(1, Math.ceil($filteredItemsStore.length / pageSize));
  $: if ($filteredItemsStore) currentPage = 0;
  $: clampedPage = Math.min(currentPage, totalPages - 1);
  $: pageOffset = clampedPage * pageSize;
  $: pageItems = $filteredItemsStore.slice(pageOffset, pageOffset + pageSize);

  $: {
    const idx = kindOptions.indexOf($kindFilterStore);
    if (idx >= 0 && buttonRefs[idx]) {
      const btn = buttonRefs[idx];
      sliderStyle = `transform: translateX(${btn.offsetLeft}px); width: ${btn.offsetWidth}px;`;
    } else {
      sliderStyle = "opacity: 0;";
    }
  }
</script>

<aside class="panel list">
  <div class="panel-header">
    <div>
      <h2>{l10n("index")}</h2>
      <p class="panel-subtitle">
        {l10n("results", { n: $filteredItemsStore.length })}
      </p>
    </div>
    <button class="ghost" on:click={onHome} disabled={$viewStore === "home"}>
      {l10n("home")}
    </button>
  </div>

  {#if $workspacePackagesStore.length > 0}
    <div class="crates">
      <div class="section-label">{l10n(":crate")}</div>
      <div class="crate-pills">
        <button
          class="crate-pill"
          class:active={!$selectedPackageIdStore}
          on:click={() => onPackageChange("")}>{l10n("all_crates")}</button
        >
        {#each $workspacePackagesStore as pkg}
          <button
            class="crate-pill"
            class:active={(pkg.id ?? pkg.name) === $selectedPackageIdStore}
            on:click={() => onPackageChange(pkg.id ?? pkg.name)}
          >
            {pkg.name}
          </button>
        {/each}
      </div>
    </div>
  {/if}

  {#if $featureNamesListStore.length > 0}
    <div class="features">
      <div class="section-label">
        {l10n("features_for", { name: $selectedPackageNameStore })}
      </div>
      <div class="feature-pills">
        {#each $featureNamesListStore as feat}
          <button
            class="feature-pill"
            class:on={$enabledFeaturesStore[feat] !== undefined}
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
      placeholder={l10n("search_placeholder")}
      value={$queryStore}
      on:input={(event) => onQueryChange(event.currentTarget.value)}
      aria-label={l10n("search_aria")}
    />
    <div class="filters">
      <div class="filter-slider" style={sliderStyle}></div>
      {#each kindOptions as option, i}
        <button
          bind:this={buttonRefs[i]}
          class:active={$kindFilterStore === option}
          on:click={() => onKindChange(option)}
        >
          {l10n(":kind_" + option.toLowerCase())}
          {#if option !== "All"}
            <span class="count">{$countsStore[option] ?? 0}</span>
          {/if}
        </button>
      {/each}
    </div>
  </div>

  <div bind:this={listItemsEl} class="list-items">
    {#if $filteredItemsStore.length === 0}
      <div class="empty">{l10n("no_matches")}</div>
    {:else}
      {#each pageItems as item, i (`${pageOffset + i}`)}
        <ListItem
          {item}
          index={i}
          selectedPath={$selectedPathStore}
          onSelect={onSelectItem}
        />
      {/each}
    {/if}
    {#if totalPages > 1}
      <div class="pagination">
        <button
          class="page-btn"
          disabled={clampedPage === 0}
          on:click={() => scrollTo(clampedPage - 1)}>&lsaquo;</button
        >
        {#each Array(totalPages) as _, p}
          <button
            class="page-btn"
            class:active={p === clampedPage}
            on:click={() => scrollTo(p)}>{p + 1}</button
          >
        {/each}
        <button
          class="page-btn"
          disabled={clampedPage >= totalPages - 1}
          on:click={() => scrollTo(clampedPage + 1)}>&rsaquo;</button
        >
      </div>
    {/if}
  </div>
</aside>
