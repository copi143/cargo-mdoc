<script lang="ts">

  export let text = "";


  let copied = false;

  const copySource = async () => {
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
      copied = true;
      setTimeout(() => {
        copied = false;
      }, 2000);
    } catch {
      console.error("Failed to copy text to clipboard");
    }
  };
</script>

<button class="copy-button" class:copied onclick={copySource}>
  <span class="default-text">{l10n("copy")}</span>
  <span class="copied-text">{l10n("copied")}</span>
</button>

<style>
  .copy-button {
    display: grid;
    @mixin button;
    color: var(--muted-2);
    background: none;
    user-select: none;

    &:hover {
      color: #fff;
      background: var(--accent);
    }

    & > * {
      grid-area: 1 / 1;
      transition: opacity 0.15s ease;
    }

    & > .default-text {
      opacity: 1;
      pointer-events: auto;
    }

    & > .copied-text {
      opacity: 0;
      pointer-events: none;
    }

    &.copied > .default-text {
      opacity: 0;
      pointer-events: none;
    }

    &.copied > .copied-text {
      opacity: 1;
      pointer-events: auto;
    }
  }
</style>
