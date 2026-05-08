<script lang="ts">
  import type { Counts, ItemDoc, PackageInfo } from "../types";
  import { renderMarkdown } from "../lib/markdown";

  export let packageInfo: PackageInfo | null = null;
  export let items: ItemDoc[] = [];
  export let counts: Counts = {};
  export let topModules: ItemDoc[] = [];
  export let prominentItems: ItemDoc[] = [];
  export let onSelectItem: (item: ItemDoc) => void = () => {};

  $: renderedReadme = renderMarkdown(
    packageInfo?.readme || "README not loaded.",
  );

  const formatSource = (item: ItemDoc): string => {
    if (!item.source) return "Source unavailable";

    const { file, line_start, line_end } = item.source;
    const range =
      line_start === line_end ? `${line_start}` : `${line_start}-${line_end}`;

    return `${file}:${range}`;
  };
</script>

<div class="detail-header">
  <div>
    <p class="kicker">Overview</p>
    <h2>Documentation Home</h2>
  </div>
</div>
<div class="home-grid">
  <section class="home-card">
    <h3>Getting Started</h3>
    <p class="home-body">
      Use the index to explore public items, or jump into a module to see its
      docs and cfg gates.
    </p>
    <div class="home-meta">
      <span>Total items</span>
      <strong>{items.length}</strong>
    </div>
  </section>
  <section class="home-card">
    <h3>README</h3>
    {#if packageInfo?.readme}
      <div class="markdown-body home-markdown">{@html renderedReadme}</div>
      <div class="home-meta">
        <span>Path</span>
        <strong>{packageInfo.readme_path ?? "README.md"}</strong>
      </div>
    {:else}
      <p class="home-body">README file not loaded into the export.</p>
    {/if}
  </section>
  <section class="home-card">
    <h3>Top Modules</h3>
    {#if topModules.length}
      <ul>
        {#each topModules as item}
          <li>
            <button class="link" on:click={() => onSelectItem(item)}>
              <span>{item.path}</span>
              <small>{formatSource(item)}</small>
            </button>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="home-body">No top-level modules detected yet.</p>
    {/if}
  </section>
  <section class="home-card">
    <h3>API Surface</h3>
    <div class="home-stats">
      <div>
        <span>Functions</span>
        <strong>{counts.Function ?? 0}</strong>
      </div>
      <div>
        <span>Structs</span>
        <strong>{counts.Struct ?? 0}</strong>
      </div>
      <div>
        <span>Enums</span>
        <strong>{counts.Enum ?? 0}</strong>
      </div>
      <div>
        <span>Traits</span>
        <strong>{counts.Trait ?? 0}</strong>
      </div>
    </div>
  </section>
  <section class="home-card">
    <h3>Highlights</h3>
    {#if prominentItems.length}
      <ul>
        {#each prominentItems as item}
          <li>
            <button class="link" on:click={() => onSelectItem(item)}>
              <span>{item.path}</span>
              <small>{formatSource(item)}</small>
            </button>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="home-body">Generate docs to see highlighted items.</p>
    {/if}
  </section>
</div>
