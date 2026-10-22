<script lang="ts">
  import type { SourceLocation } from "../../types";
  import { renderRustSource } from "../../lib/highlight";
  import CopyButton from "./CopyButton.svelte";

  export let source_code: string;
  export let source: SourceLocation | undefined;
</script>

<div class="source-item">
  <div class="source-header">
    <span>
      <span>{source?.file ?? l10n("unknown_file")}</span>
      <span>
        {#if source}
          {source.line_start}
          {#if source.line_end !== source.line_start}
            - {source.line_end}
          {/if}
        {/if}
      </span>
    </span>
    <CopyButton text={source_code} />
  </div>
  <pre class="source-code"><code class="hljs language-rust"
      >{@html renderRustSource(source_code)}</code
    ></pre>
</div>

<style>
  .source-item {
    & > .source-header {
      display: flex;
      gap: 16px;
      align-items: center;
      justify-content: space-between;
      font-size: 0.8rem;
      color: var(--muted-2);

      & > :first-child {
        display: flex;
        flex: 1;
        gap: 8px;
        align-items: center;
        justify-content: space-between;
      }
    }

    & > .source-code {
      margin: 0;
      overflow: auto;
      line-height: 1.55;
      background: #fff;
      border: 1px solid var(--line);

      & code {
        padding: 1em;
        font-family: var(--mono);
        background-image: linear-gradient(
            90deg,
            transparent calc(80ch),
            #2f9a 80ch,
            transparent calc(80ch + 1px)
          ),
          linear-gradient(
            90deg,
            transparent calc(100ch),
            #29fa 100ch,
            transparent calc(100ch + 1px)
          ),
          linear-gradient(
            90deg,
            transparent calc(120ch),
            #f29a 120ch,
            transparent calc(120ch + 1px)
          );
        background-position: 1em 0;
        background-repeat: no-repeat;
      }
    }
  }
</style>
