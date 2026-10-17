<script lang="ts">
  import type { ItemKind } from "../../types";
  import { l10nDerive } from "../../l10n";

  export let kind: ItemKind;
  export let as: "p" | "span" = "span";
  export let className: string | undefined;

  $: l10n = $l10nDerive("$$_FILE_PATH_$$");
  $: label = l10n(":kind_" + kind.toLowerCase());
</script>

<svelte:element
  this={as}
  class={`kind-badge kind-${kind.toLowerCase()} ${className}`}
>
  {label}
</svelte:element>

<style>
  .kind-badge {
    display: inline-flex;
    gap: 6px;
    align-items: center;
    padding: 4px 8px;
    font-size: 0.75rem;
    background: rgb(from var(--kind-default) r g b / var(--kind-bg-opacity));
    border: 1px solid currentColor;

    &.kind-function {
      background: rgb(from var(--kind-function) r g b / var(--kind-bg-opacity));
    }

    &.kind-struct {
      background: rgb(from var(--kind-struct) r g b / var(--kind-bg-opacity));
    }

    &.kind-enum {
      background: rgb(from var(--kind-enum) r g b / var(--kind-bg-opacity));
    }

    &.kind-trait {
      background: rgb(from var(--kind-trait) r g b / var(--kind-bg-opacity));
    }

    &.kind-module {
      background: rgb(from var(--kind-module) r g b / var(--kind-bg-opacity));
    }
  }
</style>
