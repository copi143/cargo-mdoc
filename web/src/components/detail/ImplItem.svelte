<script lang="ts">
  import type { CfgExpr, ImplItemDoc } from "../../types";
  import { renderMarkdown } from "../../lib/markdown";
  import { renderRustSource } from "../../lib/highlight";
  import { l10nDerive } from "../../l10n";

  export let item: ImplItemDoc;
  export let formatCfg: (cfg?: CfgExpr | null) => string = () => "";

  $: l10n = $l10nDerive("$$_FILE_PATH_$$");
</script>

<div class="impl-item">
  <div class="impl-item-header">
    <span class="impl-item-kind kind-badge kind-{item.kind.toLowerCase()}">
      {l10n(":kind_" + item.kind.toLowerCase())}
    </span>
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
  <pre class="source-code"><code class="hljs language-rust">{@html renderRustSource(item.source_code)}</code></pre>
</div>
