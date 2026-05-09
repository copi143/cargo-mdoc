<script lang="ts">
  import type { CfgExpr, ItemDoc } from "../types";
  import { renderMarkdown } from "../lib/markdown";
  import { renderRustSource } from "../lib/highlight";

  export let selectedItem: ItemDoc | null = null;
  export let formatCfg: (cfg?: CfgExpr | null) => string = () => "";

  $: renderedDocs = renderMarkdown(selectedItem?.docs || "No docs provided.");
  $: renderedSource = renderRustSource(selectedItem?.source_code || "");

  const splitDisplayPath = (
    path: string,
  ): { namespace: string; name: string } => {
    const separatorIndex = path.lastIndexOf("::");

    if (separatorIndex === -1) {
      return { namespace: "", name: path };
    }

    return {
      namespace: path.slice(0, separatorIndex + 2),
      name: path.slice(separatorIndex + 2),
    };
  };
</script>

{#if selectedItem}
  {@const displayPath = splitDisplayPath(selectedItem.path)}
  <div class="detail-header">
    <div>
      <p class={`kicker kind-badge kind-${selectedItem.kind.toLowerCase()}`}>
        {selectedItem.kind}
      </p>
      <h2 class="detail-title">
        {#if displayPath.namespace}
          <span class="detail-title-namespace">{displayPath.namespace}</span>
        {/if}
        <span class="detail-title-name">{displayPath.name}</span>
      </h2>
    </div>
    <div class="cfg">
      <span class="cfg-label">cfg</span>
      <code>{formatCfg(selectedItem.cfg)}</code>
    </div>
  </div>
  <div class="detail-body">
    <div class="markdown-body doc">{@html renderedDocs}</div>
    <h3>Source</h3>
    <div class="source-card">
      <div class="source-header">
        <span>{selectedItem.source?.file ?? "Unknown file"}</span>
        <span>
          {#if selectedItem.source}
            {selectedItem.source.line_start}
            {#if selectedItem.source.line_end !== selectedItem.source.line_start}
              - {selectedItem.source.line_end}
            {/if}
          {/if}
        </span>
      </div>
      <pre class="source-code"><code class="hljs language-rust"
          >{@html renderedSource}</code
        ></pre>
    </div>
  </div>
{:else}
  <div class="empty detail-empty">
    Select an item from the index to inspect its docs, cfg gates, and source
    location.
  </div>
{/if}
