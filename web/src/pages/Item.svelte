<script lang="ts">
  import type { CfgExpr, ImplDoc, ItemDoc } from "../types";
  import { renderMarkdown } from "../lib/markdown";
  import { renderRustSource } from "../lib/highlight";
  import { l10nDerive } from "../l10n";

  export let selectedItem: ItemDoc | null = null;
  export let formatCfg: (cfg?: CfgExpr | null) => string = () => "";

  $: l10n = $l10nDerive("$$_FILE_PATH_$$");

  $: renderedDocs = renderMarkdown(selectedItem?.docs || l10n("nodoc"));
  $: renderedSource = renderRustSource(selectedItem?.source_code || "");
  $: impls = selectedItem?.impls ?? [];

  const implLabel = (impl: ImplDoc): string => {
    if (impl.trait_name) {
      return `impl ${impl.trait_name} for ${impl.target_path}`;
    }
    return `impl ${impl.target_path}`;
  };

  let copied = false;

  const copySource = async () => {
    const text = selectedItem?.source_code;
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
      copied = true;
      setTimeout(() => {
        copied = false;
      }, 2000);
    } catch {
      // clipboard not available
    }
  };

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
    <div class="left">
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
    <div class="right">
      <div class="cfg">
        <span class="cfg-label">{l10n("cfg_label")}</span>
        <code>{formatCfg(selectedItem.cfg)}</code>
      </div>
    </div>
  </div>
  <div class="detail-body">
    <div class="markdown-body doc">{@html renderedDocs}</div>
    {#if impls.length > 0}
      <section class="impls-section">
        <h3>{l10n("impl_blocks", { n: impls.length })}</h3>
        {#each impls as impl (impl.id)}
          <details class="impl-block">
            <summary class="impl-header">
              <span class="impl-label">{implLabel(impl)}</span>
              {#if impl.cfg.kind !== "True"}
                <span class="cfg">
                  <span class="cfg-label">{l10n("cfg_label")}</span>
                  <code>{formatCfg(impl.cfg)}</code>
                </span>
              {/if}
              <span class="impl-toggle">▼</span>
            </summary>
            <div class="impl-body">
              <div class="impl-meta">
                <span class="source-meta">
                  {impl.source.file}:{impl.source
                    .line_start}{#if impl.source.line_end !== impl.source.line_start}-{impl
                      .source.line_end}{/if}
                </span>
              </div>
              <pre class="source-code">
                <code class="hljs language-rust">
                  {@html renderRustSource(impl.source_code)}
                </code>
              </pre>
              {#if impl.items.length > 0}
                <div class="impl-items">
                  <h4>{l10n("methods_and_items")}</h4>
                  {#each impl.items as item}
                    <div class="impl-item">
                      <div class="impl-item-header">
                        <span
                          class="impl-item-kind kind-badge kind-{item.kind.toLowerCase()}"
                          >{item.kind}</span
                        >
                        <code class="impl-item-name">{item.name}</code>
                        {#if item.cfg.kind !== "True"}
                          <span class="cfg">
                            <span class="cfg-label">{l10n("cfg_label")}</span>
                            <code>{formatCfg(item.cfg)}</code>
                          </span>
                        {/if}
                      </div>
                      <div class="markdown-body doc impl-item-doc">
                        {@html renderMarkdown(item.docs || l10n("nodoc"))}
                      </div>
                      <pre class="source-code">
                        <code class="hljs language-rust">
                          {@html renderRustSource(item.source_code)}
                        </code>
                      </pre>
                    </div>
                  {/each}
                </div>
              {/if}
            </div>
          </details>
        {/each}
      </section>
    {/if}
    <h3>{l10n("source")}</h3>
    <div class="source-card">
      <div class="source-item">
        <div class="source-header">
          <span class="source-header-left">
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
          <button class="copy-btn" onclick={copySource}>
            {copied ? l10n("copied") : l10n("copy")}
          </button>
        </div>
        <pre class="source-code"><code class="hljs language-rust"
            >{@html renderedSource}</code
          ></pre>
      </div>
    </div>
  </div>
{:else}
  <div class="empty detail-empty">
    {l10n("empty_select")}
  </div>
{/if}
