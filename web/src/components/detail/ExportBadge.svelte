<script lang="ts">
  export let as: "div" | "span" = "span";
</script>

<svelte:element this={as} class="export-badge">
  <span class="export-label">{l10n("export_label")}</span>
  <code>#[macro_export]</code>
</svelte:element>

<style>
  .export-badge {
    display: inline-flex;
    gap: 8px;
    align-items: center;
    padding: 6px 10px;
    font-size: 0.85rem;
    color: var(--kind-macro);
    background: rgb(from var(--kind-macro) r g b / var(--kind-bg-opacity));
    border: 1px solid currentColor;
    border-radius: 0;

    .export-label {
      font-size: 0.7rem;
      color: var(--muted);
      text-transform: uppercase;
      letter-spacing: 0.08em;
    }

    code {
      font-size: 0.8rem;
      color: var(--ink);
      background: transparent;
    }
  }
</style>
