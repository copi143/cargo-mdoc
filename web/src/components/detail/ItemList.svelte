<script lang="ts">
  import type { ItemDoc } from "../../types";
  import { l10nDerive } from "../../l10n";

  export let items: ItemDoc[] = [];
  export let onSelectItem: (item: ItemDoc) => void = () => {};
  export let emptyMessage = "";

  $: l10n = $l10nDerive("$$_FILE_PATH_$$");

  const formatSource = (item: ItemDoc): string => {
    if (!item.source) return l10n(":source_unavailable");
    const { file, line_start, line_end } = item.source;
    const range =
      line_start === line_end ? `${line_start}` : `${line_start}-${line_end}`;
    return `${file}:${range}`;
  };
</script>

{#if items.length}
  <ul>
    {#each items as item}
      <li>
        <button class="link" on:click={() => onSelectItem(item)}>
          <span>{item.path}</span>
          <small>{formatSource(item)}</small>
        </button>
      </li>
    {/each}
  </ul>
{:else if emptyMessage}
  <p class="home-body">{emptyMessage}</p>
{/if}
