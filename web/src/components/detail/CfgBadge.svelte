<script lang="ts">
  import type { CfgExpr } from "../../types";

  export let cfg: CfgExpr | null | undefined;
  export let formatCfg: (cfg?: CfgExpr | null) => string = () => "";
  export let as: "div" | "span" = "span";
  export let showWhenTrue: boolean = false;

  $: hidden = !cfg || (cfg.kind === "True" && !showWhenTrue);
</script>

{#if !hidden}
  <svelte:element this={as} class="cfg">
    <span class="cfg-label">{l10n("cfg_label")}</span>
    <code>{formatCfg(cfg)}</code>
  </svelte:element>
{/if}

<style>
  .cfg {
    display: inline-flex;
    gap: 8px;
    align-items: center;
    padding: 6px 10px;
    font-size: 0.85rem;
    background: var(--accent-soft);
    border: 1px solid rgba(31, 111, 235, 0.4);
    border-radius: 0;

    .cfg-label {
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
