<script lang="ts">
  import type { GeneratorInfo, ItemDoc, PackageInfo } from "../../types";
  import { locale, langs } from "../../l10n";
  import {
    packageInfoStore,
    itemsStore,
    generatorStore,
    resolveGraphStore,
  } from "../../lib/store";

  export let onOpenGraph: () => void = () => {};

  $: currentLocale = $locale;

  const locales = Object.entries(langs).map(
    ([code, data]) => [code, `${data.name} / ${data.display}`] as const,
  );
</script>

<header class="topbar">
  <div class="brand">
    <div class="brand-mark">{l10n("brand")}</div>
    <div class="brand-info">
      <p class="kicker">{l10n("subtitle")}</p>
      <p class="brand-title">
        <span class="brand-name">{$packageInfoStore?.name ?? l10n("doc_index")}</span>
        {#if $packageInfoStore}
          <span class="brand-version"
            >{$packageInfoStore.version ?? l10n("version_unknown")}</span
          >
        {/if}
      </p>
      {#if $packageInfoStore}
        <p class="brand-meta">
          <span
            >{l10n("edition")}{$packageInfoStore.edition ??
              l10n("edition_unknown")}</span
          >
          {#if $packageInfoStore.license}
            <span class="dot">{l10n("separator")}</span>
            <span>{$packageInfoStore.license}</span>
          {/if}
        </p>
      {/if}
    </div>
  </div>
  <div class="meta">
    <div class="meta-items">
      <span class="meta-label">{l10n("items_label")}</span>
      <span class="meta-count">{$itemsStore.length}</span>
    </div>
    <button class="ghost" on:click={onOpenGraph} disabled={!$resolveGraphStore}>
      {l10n("dep_graph")}
    </button>
    {#if $generatorStore?.commit_id}
      <span class="commit-badge" title={$generatorStore.commit_id}>
        {$generatorStore.commit_id.slice(0, 7)}
      </span>
    {/if}
    <select
      class="lang-switcher"
      value={currentLocale}
      on:change={(e) => locale.set(e.currentTarget.value)}
    >
      {#each locales as [code, label]}
        <option value={code}>{label}</option>
      {/each}
    </select>
  </div>
</header>
