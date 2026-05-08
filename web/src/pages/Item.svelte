<script>
  export let selectedItem = null;
  export let formatCfg = () => "";
</script>

{#if selectedItem}
  <div class="detail-header">
    <div>
      <p class="kicker">{selectedItem.kind}</p>
      <h2>{selectedItem.path}</h2>
    </div>
    <div class="cfg">
      <span class="cfg-label">cfg</span>
      <code>{formatCfg(selectedItem.cfg)}</code>
    </div>
  </div>
  <div class="detail-body">
    <h3>Docs</h3>
    <pre class="doc">{selectedItem.docs || "No docs provided."}</pre>
  </div>
{:else}
  <div class="empty detail-empty">
    Select an item from the index to inspect its docs and cfg gates.
  </div>
{/if}
