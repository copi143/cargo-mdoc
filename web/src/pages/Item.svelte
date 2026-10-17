<script lang="ts">
  import type { ItemDoc } from "../types";
  import { formatCfg } from "../lib/cfg";
  import { splitDisplayPath } from "../lib/path";
  import { l10nDerive } from "../l10n";
  import ImplBlock from "../components/detail/ImplBlock.svelte";
  import SourceItem from "../components/detail/SourceItem.svelte";
  import DocBody from "../components/detail/DocBody.svelte";
  import CfgBadge from "../components/detail/CfgBadge.svelte";
  import { selectedItem as selectedItemStore } from "../lib/derived";

  $: l10n = $l10nDerive("$$_FILE_PATH_$$");
  $: selectedItem = $selectedItemStore;

  $: impls = selectedItem?.impls ?? [];
</script>

{#if selectedItem}
  {@const displayPath = splitDisplayPath(selectedItem.path)}
  <div class="detail-header">
    <p class={`kicker kind-badge kind-${selectedItem.kind.toLowerCase()}`}>
      {l10n(":kind_" + selectedItem.kind.toLowerCase())}
    </p>
    <h2 class="detail-title">
      {#if displayPath.namespace}
        <span class="detail-title-namespace">{displayPath.namespace}</span>
      {/if}
      <span class="detail-title-name">{displayPath.name}</span>
    </h2>
    <CfgBadge cfg={selectedItem.cfg} {formatCfg} as="div" showWhenTrue={true} />
  </div>
  <div class="detail-body">
    <DocBody source={selectedItem?.docs} />
    <SourceItem source_code={selectedItem?.source_code ?? ""} source={selectedItem?.source} />
    {#if impls.length > 0}
      <section class="impls-section">
        <h3>{l10n("impl_blocks", { n: impls.length })}</h3>
        {#each impls as impl (impl.id)}
          <ImplBlock {impl} {formatCfg} />
        {/each}
      </section>
    {/if}
  </div>
{:else}
  <div class="empty detail-empty">
    {l10n("empty_select")}
  </div>
{/if}
