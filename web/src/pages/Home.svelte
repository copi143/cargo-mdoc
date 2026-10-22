<script lang="ts">
  import type { ItemDoc, PackageInfo, ResolveGraph } from "../types";
  import { renderMarkdown } from "../lib/markdown";
  import GraphPreview from "../components/depgraph/GraphPreview.svelte";
  import ItemList from "../components/detail/ItemList.svelte";
  import { packageInfoStore, itemsStore, resolveGraphStore } from "../lib/store";
  import { selectedPackage } from "../lib/derived";
  import {
    counts as countsStore,
    topModules as topModulesStore,
    prominentItems as prominentItemsStore,
  } from "../lib/derived";

  export let onSelectItem: (item: ItemDoc) => void = () => {};

  $: selectedPkg = $selectedPackage;

  $: renderedReadme = renderMarkdown($packageInfoStore?.readme || l10n("readme_not_loaded_err"));

  $: externalDocs = ($selectedPkg?.documents ?? []).filter(
    (doc) => doc.kind !== "readme" && doc.content,
  );
</script>

<div class="home-page">
  <div class="detail-header">
    <div>
      <p class="kicker">{l10n("overview")}</p>
      <h2>{l10n("doc_home")}</h2>
    </div>
  </div>
  <div class="home-layout">
    <section class="home-card home-readme">
      <h3>{l10n("readme")}</h3>
      {#if $packageInfoStore?.readme}
        <main class="markdown-body home-markdown">{@html renderedReadme}</main>
        <div class="home-meta">
          <span>{l10n("path")}</span>
          <strong>{$packageInfoStore.readme_path ?? l10n("readme_default")}</strong>
        </div>
      {:else}
        <p class="home-body">{l10n("readme_not_loaded")}</p>
      {/if}
    </section>

    <div class="home-side">
      <GraphPreview {resolveGraphStore} />

      <section class="home-card">
        <h3>{l10n("getting_started")}</h3>
        <p class="home-body">
          {l10n("getting_started_desc")}
        </p>
        <div class="home-meta">
          <span>{l10n("total_items")}</span>
          <strong>{$itemsStore.length}</strong>
        </div>
      </section>

      {#if externalDocs.length > 0}
        <section class="home-card home-external-docs">
          <h3>{l10n("external_docs")}</h3>
          {#each externalDocs as doc (doc.path)}
            <div class="external-doc">
              <h4>{doc.name}</h4>
              <div class="home-meta">
                <span>{l10n("doc_kind_" + doc.kind)}</span>
                <strong>{doc.path}</strong>
              </div>
              <main class="markdown-body home-markdown">
                {@html renderMarkdown(doc.content)}
              </main>
            </div>
          {/each}
        </section>
      {/if}

      <section class="home-card">
        <h3>{l10n("top_modules")}</h3>
        <ItemList items={$topModulesStore} {onSelectItem} emptyMessage={l10n("no_top_modules")} />
      </section>

      <section class="home-card">
        <h3>{l10n("api_surface")}</h3>
        <div class="home-stats">
          <div>
            <span>{l10n("functions")}</span>
            <strong>{$countsStore.Function ?? 0}</strong>
          </div>
          <div>
            <span>{l10n("structs")}</span>
            <strong>{$countsStore.Struct ?? 0}</strong>
          </div>
          <div>
            <span>{l10n("enums")}</span>
            <strong>{$countsStore.Enum ?? 0}</strong>
          </div>
          <div>
            <span>{l10n("traits")}</span>
            <strong>{$countsStore.Trait ?? 0}</strong>
          </div>
          <div>
            <span>{l10n("macros")}</span>
            <strong>{$countsStore.Macro ?? 0}</strong>
          </div>
        </div>
      </section>

      <section class="home-card">
        <h3>{l10n("highlights")}</h3>
        <ItemList
          items={$prominentItemsStore}
          {onSelectItem}
          emptyMessage={l10n("no_highlights")}
        />
      </section>
    </div>
  </div>
</div>
