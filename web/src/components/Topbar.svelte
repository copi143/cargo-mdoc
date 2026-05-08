<script>
  export let packageInfo;
  export let items = [];
  export let counts = {};
</script>

<header class="topbar">
  <div class="brand">
    <div class="brand-mark">mdoc</div>
    <div>
      <p class="kicker">Rust module surface</p>
      <h1>{packageInfo?.name ?? "Documentation Index"}</h1>
      <p class="subtitle">
        {#if packageInfo}
          <span>{packageInfo.version ?? "0.0.0"}</span>
          <span class="dot">•</span>
          <span>Edition {packageInfo.edition ?? "unknown"}</span>
          {#if packageInfo.license}
            <span class="dot">•</span>
            <span>{packageInfo.license}</span>
          {/if}
        {:else}
          Load the latest mdoc export to explore the API surface.
        {/if}
      </p>
    </div>
  </div>
  <div class="meta">
    <div class="meta-card">
      <div class="meta-label">Items</div>
      <div class="meta-value">{items.length}</div>
    </div>
    <div class="meta-card">
      <div class="meta-label">Kinds</div>
      <div class="meta-value">
        {Object.keys(counts).length || 0}
      </div>
    </div>
    <div class="meta-card">
      <div class="meta-label">Source</div>
      <div class="meta-value">mdoc-ra.json</div>
    </div>
  </div>
</header>
