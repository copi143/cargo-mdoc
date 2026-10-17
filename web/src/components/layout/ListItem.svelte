<script lang="ts">
  import type { ItemDoc } from "../../types";
  import { formatSource } from "../../lib/path";
  import { l10nDerive } from "../../l10n";
  import { onMount } from "svelte";

  export let item: ItemDoc;
  export let selectedPath: string = "";
  export let index: number = 0;
  export let onSelect: (item: ItemDoc) => void = () => {};

  function abbreviateSegment(segment: string): string {
    return segment.length < 4
      ? segment.toLowerCase()
      : segment
          .replace(/([A-Z]+)([A-Z][a-z])/g, "$1 $2")
          .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
          .split(/[-_\s]+/)
          .filter((word) => word.length > 0)
          .map((word) => word[0].toLowerCase())
          .join("");
  }

  function abbreviations(
    identifier: string,
    separator: string = "/",
  ): string[] {
    let parts = identifier.split(separator);
    let rets = [identifier];
    for (let i = 0; i < parts.length - 1; i++) {
      const abbr = abbreviateSegment(parts[i]);
      if (abbr == parts[i]) continue;
      parts[i] = abbr;
      rets.push(parts.join(separator));
    }
    return rets;
  }

  $: l10n = $l10nDerive("$$_FILE_PATH_$$");
  $: pathCandidates = abbreviations(item.path, "::");
  $: sourceCandidates = abbreviations(formatSource(item, l10n), "/");
  $: pathDisplayed = pathCandidates[0];
  $: sourceDisplayed = sourceCandidates[0];

  let mainEl: HTMLSpanElement;
  let pathEl: HTMLSpanElement;
  let sourceEl: HTMLSpanElement;

  function fit(el: HTMLSpanElement | null, candidates: string[]): string {
    if (!el || candidates.length === 0) return "";
    for (const cand of candidates) {
      el.textContent = cand;
      if (el.clientWidth < mainEl.clientWidth) return cand;
    }
    el.textContent = candidates[candidates.length - 1];
    return candidates[candidates.length - 1];
  }

  onMount(() => {
    const ro = new ResizeObserver(() => {
      if (pathEl) pathDisplayed = fit(pathEl, pathCandidates);
      if (sourceEl) sourceDisplayed = fit(sourceEl, sourceCandidates);
    });
    ro.observe(pathEl!.parentElement!);
  });
</script>

<button
  class="list-item"
  class:selected={item.path === selectedPath}
  style={`--i: ${index}`}
  onclick={() => onSelect(item)}
>
  <span bind:this={mainEl} class="row-main">
    <span bind:this={pathEl} class="path">{pathDisplayed}</span>
    <span bind:this={sourceEl} class="source-line">{sourceDisplayed}</span>
  </span>
  <span class={`kind kind-${item.kind.toLowerCase()}`}>
    {l10n(":kind_" + item.kind.toLowerCase())}
  </span>
</button>

<style>
  .list-item {
    display: flex;
    gap: 12px;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px;
    text-align: left;
    cursor: pointer;
    background: #fff;
    border: 1px solid transparent;
    border-radius: 0;
    opacity: 0;
    transition:
      transform 0.15s ease,
      border-color 0.2s ease,
      box-shadow 0.2s ease;
    animation: listIn 0.4s ease forwards;
    animation-delay: calc(var(--i) * 0.02s);

    &:hover {
      border-color: var(--line);
      box-shadow: var(--shadow-soft);
      transform: translateY(-1px);
    }

    &.selected {
      border-color: var(--accent);
      box-shadow: 0 12px 24px rgba(31, 111, 235, 0.14);
    }

    .path {
      font-size: 0.95rem;
      color: var(--ink);
      word-break: break-word;
    }

    .kind {
      font-size: 0.75rem;
      text-transform: uppercase;
      letter-spacing: 0.08em;
      text-wrap: nowrap;
    }

    & > .row-main {
      display: flex;
      flex: 1;
      flex-direction: column;
      gap: 2px;
      overflow: clip;

      & > span {
        margin-right: auto;
      }
    }
  }
</style>
