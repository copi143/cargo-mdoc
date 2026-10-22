<script lang="ts">
  import type { MacroCallDoc } from "../../types";
  import SourceItem from "./SourceItem.svelte";
  import KindBadge from "./KindBadge.svelte";

  export let call: MacroCallDoc;
</script>

<div class="macro-call">
  <div class="macro-call-header">
    <KindBadge kind="Macro" className="macro-call-kind" />
    <code class="macro-call-path">{call.path}</code>
    <span class="macro-call-module">{call.module_path}</span>
  </div>
  <SourceItem source_code={call.source_code} source={call.source} />
</div>
