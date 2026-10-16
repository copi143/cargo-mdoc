<script lang="ts">
  import { renderMarkdown } from "../../lib/markdown";
  import { l10nDerive } from "../../l10n";
  import CopyButton from "./CopyButton.svelte";

  export let source: string | null = null;
  export let className: string = "";

  $: l10n = $l10nDerive("$$_FILE_PATH_$$");
  $: rendered = renderMarkdown(source ?? "");
  $: hasDocs = source != null && source.trim().length > 0;

  let showCopy = false;
</script>

<div class="doc-wrapper" role="region" aria-label={l10n("copy_docs")} class:show-copy={showCopy} onmousemove={() => (showCopy = true)} onmouseleave={() => (showCopy = false)}>
  <CopyButton text={source ?? ""} />
  <div class="markdown-body doc {className}">
    {#if hasDocs}
      {@html rendered}
    {:else}
      <p class="doc-empty">{l10n("nodoc")}</p>
    {/if}
  </div>
</div>

<style>
  .doc-wrapper {
    position: relative;
  }

  :global(.copy-button) {
    position: absolute;
    top: 6px;
    right: 6px;
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.15s ease, transform 0.15s ease;
    transform: translateY(-4px);
    z-index: 5;
  }

  .doc-wrapper.show-copy :global(.copy-button) {
    opacity: 1;
    pointer-events: auto;
    transform: translateY(0);
  }

  .doc-empty {
    color: var(--muted-1);
    font-style: italic;
    margin: 0;
  }
</style>
