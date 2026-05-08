<script>
  export let items = [];
  export let counts = {};
  export let topModules = [];
  export let prominentItems = [];
  export let onSelectItem = () => {};
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
    <h3>Top Modules</h3>
    {#if topModules.length}
      <ul>
        {#each topModules as item}
          <li>
            <button class="link" on:click={() => onSelectItem(item)}>
              {item.path}
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
              {item.path}
            </button>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="home-body">Generate docs to see highlighted items.</p>
    {/if}
  </section>
</div>
