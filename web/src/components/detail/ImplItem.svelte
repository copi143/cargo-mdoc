<script lang="ts">
  import type { CfgExpr, ImplItemDoc } from "../../types";
  import SourceItem from "./SourceItem.svelte";
  import DocBody from "./DocBody.svelte";
  import CfgBadge from "./CfgBadge.svelte";
  import KindBadge from "./KindBadge.svelte";

  export let item: ImplItemDoc;
  export let formatCfg: (cfg?: CfgExpr | null) => string = () => "";
</script>

<div class="impl-item">
  <div class="impl-item-header">
    <KindBadge kind={item.kind} className="impl-item-kind" />
    <code class="impl-item-name">{item.name}</code>
    <CfgBadge cfg={item.cfg} {formatCfg} />
  </div>
  <DocBody source={item.docs} className="impl-item-doc" />
  <SourceItem source_code={item.source_code} source={item.source} />
</div>
