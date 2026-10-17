<script lang="ts">
  import type { CfgExpr, ImplDoc } from "../../types";
  import { l10nDerive } from "../../l10n";
  import ImplItem from "./ImplItem.svelte";
  import SourceItem from "./SourceItem.svelte";
  import CfgBadge from "./CfgBadge.svelte";

  export let impl: ImplDoc;
  export let formatCfg: (cfg?: CfgExpr | null) => string = () => "";

  $: l10n = $l10nDerive("$$_FILE_PATH_$$");

  const implLabel = (impl: ImplDoc): string => {
    if (impl.trait_name) {
      return `impl ${impl.trait_name} for ${impl.target_path}`;
    }
    return `impl ${impl.target_path}`;
  };
</script>

<details class="impl-block">
  <summary class="impl-header">
    <span class="impl-label">{implLabel(impl)}</span>
    <CfgBadge cfg={impl.cfg} {formatCfg} />
    <span class="impl-toggle">▼</span>
  </summary>
  <div class="impl-body">
    <SourceItem source_code={impl.source_code} source={impl.source} />
    {#if impl.items.length > 0}
      <div class="impl-items">
        <h4>{l10n("methods_and_items")}</h4>
        {#each impl.items as item}
          <ImplItem {item} {formatCfg} />
        {/each}
      </div>
    {/if}
  </div>
</details>
