<script lang="ts">
  export let page: number = 0;
  export let totalPages: number = 1;
  export let onPageChange: (page: number) => void = () => {};
</script>

{#if totalPages > 1}
  <div class="pagination">
    <button
      class="page-btn"
      disabled={page === 0}
      on:click={() => onPageChange(page - 1)}>&lsaquo;</button
    >
    {#each Array(totalPages) as _, p}
      <button
        class="page-btn"
        class:active={p === page}
        on:click={() => onPageChange(p)}>{p + 1}</button
      >
    {/each}
    <button
      class="page-btn"
      disabled={page >= totalPages - 1}
      on:click={() => onPageChange(page + 1)}>&rsaquo;</button
    >
  </div>
{/if}

<style>
  .pagination {
    position: sticky;
    bottom: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    justify-content: center;
    padding: 12px 0;
    background: var(--paper);
    border-top: 1px solid var(--line);
  }

  .page-btn {
    display: grid;
    place-items: center;
    min-width: 32px;
    height: 32px;
    padding: 0 6px;
    font-size: 0.8rem;
    color: var(--muted);
    cursor: pointer;
    background: transparent;
    border: 1px solid var(--line);
    border-radius: 0;
  }

  .page-btn:hover:not(:disabled) {
    color: var(--ink);
    border-color: var(--muted-2);
  }

  .page-btn:disabled {
    cursor: default;
    opacity: 0.35;
  }

  .page-btn.active {
    font-weight: 600;
    color: var(--accent);
    border-color: var(--accent);
  }
</style>
