<script lang="ts">
  import type { ItemDoc, PackageInfo } from "../types";
  import { locale, langs, l10nDerive } from "../l10n";

  export let packageInfo: PackageInfo | null = null;
  export let items: ItemDoc[] = [];
  export let onOpenGraph: () => void = () => {};
  export let graphAvailable = false;

  $: l10n = $l10nDerive("$$_FILE_PATH_$$");
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
        <span class="brand-name">{packageInfo?.name ?? l10n("doc_index")}</span>
        {#if packageInfo}
          <span class="brand-version"
            >{packageInfo.version ?? l10n("version_unknown")}</span
          >
        {/if}
      </p>
      {#if packageInfo}
        <p class="brand-meta">
          <span
            >{l10n("edition")}{packageInfo.edition ??
              l10n("edition_unknown")}</span
          >
          {#if packageInfo.license}
            <span class="dot">{l10n("separator")}</span>
            <span>{packageInfo.license}</span>
          {/if}
        </p>
      {/if}
    </div>
  </div>
  <div class="meta">
    <div class="meta-items">
      <span class="meta-label">{l10n("items_label")}</span>
      <span class="meta-count">{items.length}</span>
    </div>
    <button class="ghost" on:click={onOpenGraph} disabled={!graphAvailable}>
      {l10n("dep_graph")}
    </button>
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
