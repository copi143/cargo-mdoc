<script lang="ts">
  import type { ItemDoc } from "../../types";
  import { formatSource } from "../../lib/path";

  export let items: ItemDoc[] = [];
  export let onSelectItem: (item: ItemDoc) => void = () => {};
  export let emptyMessage = "";
</script>

{#if items.length}
  <ul>
    {#each items as item}
      <li>
        <button class="link" on:click={() => onSelectItem(item)}>
          <span>{item.path}</span>
          <small>{formatSource(item, l10n)}</small>
        </button>
      </li>
    {/each}
  </ul>
{:else if emptyMessage}
  <p class="home-body">{emptyMessage}</p>
{/if}
