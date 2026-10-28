<script lang="ts">
  import type { MacroCallDoc } from "../../types";
  import { renderRustSource } from "../../lib/highlight";
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
  {#if call.expanded_code}
    <details class="macro-expansion">
      <summary class="macro-expansion-header">
        <span class="macro-expansion-label">{l10n("expansion")}</span>
        <span class="macro-expansion-toggle">▼</span>
      </summary>
      <div class="macro-expansion-body">
        <pre class="macro-expansion-code"><code class="hljs language-rust"
            >{@html renderRustSource(call.expanded_code)}</code
          ></pre>
        {#if call.expanded_items && call.expanded_items.length > 0}
          <div class="macro-expansion-items">
            <h4>{l10n("generated_items")}</h4>
            {#each call.expanded_items as item (item.name)}
              <div class="macro-expansion-item">
                <KindBadge kind={item.kind} className="macro-expansion-kind" />
                <code class="macro-expansion-name">{item.name}</code>
                {#if item.signature}
                  <code class="macro-expansion-signature">{item.signature}</code>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </details>
  {/if}
  {#if call.expansion_error}
    <p class="macro-expansion-error">{l10n("expansion_error")}: {call.expansion_error}</p>
  {/if}
</div>
