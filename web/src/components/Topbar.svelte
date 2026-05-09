<script lang="ts">
  import type { ItemDoc, PackageInfo } from "../types";

  export let packageInfo: PackageInfo | null = null;
  export let items: ItemDoc[] = [];
  export let onOpenGraph: () => void = () => {};
  export let graphAvailable = false;
</script>

<header class="topbar">
  <div class="brand">
    <div class="brand-mark">mdoc</div>
    <div class="brand-info">
      <p class="kicker">Rust module surface</p>
      <p class="brand-title">
        <span class="brand-name">{packageInfo?.name ?? "Documentation Index"}</span>
        {#if packageInfo}
          <span class="brand-version">{packageInfo.version ?? "0.0.0"}</span>
        {/if}
      </p>
      {#if packageInfo}
        <p class="brand-meta">
          <span>Edition {packageInfo.edition ?? "unknown"}</span>
          {#if packageInfo.license}
            <span class="dot">•</span>
            <span>{packageInfo.license}</span>
          {/if}
        </p>
      {/if}
    </div>
  </div>
  <div class="meta">
    <div class="meta-items">
      <span class="meta-label">Items</span>
      <span class="meta-count">{items.length}</span>
    </div>
    <button class="ghost" on:click={onOpenGraph} disabled={!graphAvailable}>
      Dependency graph
    </button>
  </div>
</header>
