<script lang="ts">
  import type { ItemDoc } from "../types";
  import { renderMarkdown } from "../lib/markdown";
  import { renderRustSource } from "../lib/highlight";
  import { formatCfg } from "../lib/cfg";
  import { splitDisplayPath } from "../lib/path";
  import { l10nDerive } from "../l10n";
  import ImplBlock from "../components/detail/ImplBlock.svelte";
  import CopyButton from "../components/detail/CopyButton.svelte";
  import { selectedItem as selectedItemStore } from "../lib/derived";

  $: l10n = $l10nDerive("$$_FILE_PATH_$$");
  $: selectedItem = $selectedItemStore;

  $: renderedDocs = renderMarkdown(selectedItem?.docs || l10n("nodoc"));
  $: renderedSource = renderRustSource(selectedItem?.source_code || "");
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
    <div class="cfg">
      <span class="cfg-label">{l10n("cfg_label")}</span>
      <code>{formatCfg(selectedItem.cfg)}</code>
    </div>
  </div>
  <div class="detail-body">
    <div class="markdown-body doc">{@html renderedDocs}</div>
    <div class="source-item">
      <div class="source-header">
        <span>
          <span>{selectedItem.source?.file ?? l10n("unknown_file")}</span>
          <span>
            {#if selectedItem.source}
              {selectedItem.source.line_start}
              {#if selectedItem.source.line_end !== selectedItem.source.line_start}
                - {selectedItem.source.line_end}
              {/if}
            {/if}
          </span>
        </span>
        <CopyButton text={selectedItem?.source_code ?? ""} />
      </div>
      <pre class="source-code"><code class="hljs language-rust"
          >{@html renderedSource}</code
        ></pre>
    </div>
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
