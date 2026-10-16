<script lang="ts">
  import type { CfgExpr, ImplItemDoc } from "../../types";
  import { l10nDerive } from "../../l10n";
  import SourceItem from "./SourceItem.svelte";
  import DocBody from "./DocBody.svelte";

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
  <DocBody source={item.docs} className="impl-item-doc" />
  <SourceItem source_code={item.source_code} source={item.source} />
</div>
