<script lang="ts">
  import type { Counts, ItemDoc, ItemKind } from "../types";

  export let filteredItems: ItemDoc[] = [];
  export let counts: Counts = {};
  export let kindOptions: Array<"all" | ItemKind> = [];
  export let selectedPath = "";
  export let kindFilter: "all" | ItemKind = "all";
  export let query = "";
  export let view: "home" | "item" = "home";

  export let onHome: () => void = () => {};
  export let onSelectItem: (item: ItemDoc) => void = () => {};
  export let onQueryChange: (value: string) => void = () => {};
  export let onKindChange: (value: "all" | ItemKind) => void = () => {};
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

  <div class="search">
    <input
      type="search"
      placeholder="Search paths or docs"
      value={query}
      on:input={(event) => onQueryChange(event.currentTarget.value)}
      aria-label="Search"
    />
    <div class="filters">
      {#each kindOptions as option}
        <button
          class:active={kindFilter === option}
          on:click={() => onKindChange(option)}
        >
          {option}
          {#if option !== "all"}
            <span class="count">{counts[option] ?? 0}</span>
          {/if}
        </button>
      {/each}
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
          <span class="path">{item.path}</span>
          <span class="kind">{item.kind}</span>
        </button>
      {/each}
    {/if}
  </div>
</aside>
