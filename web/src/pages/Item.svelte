<script lang="ts">
  import type { ItemDoc } from "../types";
  import { formatCfg } from "../lib/cfg";
  import { splitDisplayPath } from "../lib/path";
  import ImplBlock from "../components/detail/ImplBlock.svelte";
  import SourceItem from "../components/detail/SourceItem.svelte";
  import DocBody from "../components/detail/DocBody.svelte";
  import CfgBadge from "../components/detail/CfgBadge.svelte";
  import KindBadge from "../components/detail/KindBadge.svelte";
  import { selectedItem as selectedItemStore } from "../lib/derived";

  $: selectedItem = $selectedItemStore;

  $: impls = selectedItem?.impls ?? [];
</script>

{#if selectedItem}
  {@const displayPath = splitDisplayPath(selectedItem.path)}
  <div class="detail-header">
    <KindBadge kind={selectedItem.kind} as="p" className="kicker" />
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
