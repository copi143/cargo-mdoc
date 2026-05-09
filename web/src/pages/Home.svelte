<script lang="ts">
  import type { Counts, ItemDoc, PackageInfo, ResolveGraph } from "../types";
  import { renderMarkdown } from "../lib/markdown";

  export let packageInfo: PackageInfo | null = null;
  export let items: ItemDoc[] = [];
  export let counts: Counts = {};
  export let topModules: ItemDoc[] = [];
  export let prominentItems: ItemDoc[] = [];
  export let resolveGraph: ResolveGraph | null = null;
  export let onSelectItem: (item: ItemDoc) => void = () => {};

  let graphNodeById = new Map<string, ResolveGraph["nodes"][number]>();
  let graphIncoming: Record<string, number> | undefined;
  let graphOutgoing: Record<string, number> | undefined;

  $: renderedReadme = renderMarkdown(
    packageInfo?.readme || "README not loaded.",
  );

  const formatSource = (item: ItemDoc): string => {
    if (!item.source) return "Source unavailable";

    const { file, line_start, line_end } = item.source;
    const range =
      line_start === line_end ? `${line_start}` : `${line_start}-${line_end}`;

    return `${file}:${range}`;
  };

  const formatNodeLabel = (node: ResolveGraph["nodes"][number]): string =>
    `${node.name} ${node.version}`;

  const resolveNodeLabel = (nodeId: string): string => {
    const node = graphNodeById.get(nodeId);
    return node ? formatNodeLabel(node) : nodeId;
  };

  const edgeKindLabel = (kind: string): string => {
    switch (kind) {
      case "normal":
        return "normal";
      case "dev":
        return "dev";
      case "build":
        return "build";
      default:
        return "unknown";
    }
  };

  $: graphNodeById = new Map(
    resolveGraph?.nodes.map((node) => [node.id, node]) ?? [],
  );

  $: graphIncoming = resolveGraph?.edges.reduce<Record<string, number>>(
    (acc, edge) => {
      acc[edge.to] = (acc[edge.to] ?? 0) + 1;
      return acc;
    },
    {},
  );

  $: graphOutgoing = resolveGraph?.edges.reduce<Record<string, number>>(
    (acc, edge) => {
      acc[edge.from] = (acc[edge.from] ?? 0) + 1;
      return acc;
    },
    {},
  );
</script>

<div class="home-page">
  <div class="detail-header">
    <div>
      <p class="kicker">Overview</p>
      <h2>Documentation Home</h2>
    </div>
  </div>
  <div class="home-layout">
    <section class="home-card home-readme">
      <h3>README</h3>
      {#if packageInfo?.readme}
        <div class="markdown-body home-markdown">{@html renderedReadme}</div>
        <div class="home-meta">
          <span>Path</span>
          <strong>{packageInfo.readme_path ?? "README.md"}</strong>
        </div>
      {:else}
        <p class="home-body">README file not loaded into the export.</p>
      {/if}
    </section>

    <div class="home-side">
      <section class="home-card home-graph">
        <div class="home-graph-header">
          <div>
            <h3>Resolve Graph</h3>
            <p class="home-body">
              Cargo metadata resolve graph exported from the workspace.
            </p>
          </div>
          {#if resolveGraph?.generated_by}
            <span class="graph-badge">{resolveGraph.generated_by}</span>
          {/if}
        </div>

        {#if resolveGraph}
          <div class="home-stats graph-stats">
            <div>
              <span>Roots</span>
              <strong>{resolveGraph.roots.length}</strong>
            </div>
            <div>
              <span>Nodes</span>
              <strong>{resolveGraph.nodes.length}</strong>
            </div>
            <div>
              <span>Edges</span>
              <strong>{resolveGraph.edges.length}</strong>
            </div>
            <div>
              <span>Workspace</span>
              <strong>
                {resolveGraph.nodes.filter((node) => node.workspace_member)
                  .length}
              </strong>
            </div>
          </div>

          <div class="graph-section">
            <h4>Roots</h4>
            {#if resolveGraph.roots.length}
              <div class="graph-pills">
                {#each resolveGraph.roots as rootId}
                  <span class="graph-pill" title={rootId}
                    >{resolveNodeLabel(rootId)}</span
                  >
                {/each}
              </div>
            {:else}
              <p class="home-body">No resolve roots available.</p>
            {/if}
          </div>

          <div class="graph-section">
            <h4>Packages</h4>
            {#if resolveGraph.nodes.length}
              <div class="graph-list">
                {#each resolveGraph.nodes.slice(0, 8) as node}
                  <article class="graph-node">
                    <div class="graph-node-head">
                      <div>
                        <strong>{node.name}</strong>
                        <div class="graph-node-subtitle">{node.version}</div>
                      </div>
                      {#if node.workspace_member}
                        <span class="graph-tag">workspace</span>
                      {/if}
                    </div>
                    <div class="graph-node-meta">
                      <span>in {graphIncoming?.[node.id] ?? 0}</span>
                      <span>out {graphOutgoing?.[node.id] ?? 0}</span>
                      {#if node.source}
                        <span>{node.source}</span>
                      {/if}
                    </div>
                    {#if resolveGraph.edges.some((edge) => edge.from === node.id)}
                      <div class="graph-node-edges">
                        {#each resolveGraph.edges
                          .filter((edge) => edge.from === node.id)
                          .slice(0, 3) as edge}
                          <div class="graph-edge-row">
                            <span>{edgeKindLabel(edge.kind)}</span>
                            <span>{resolveNodeLabel(edge.to)}</span>
                          </div>
                        {/each}
                      </div>
                    {/if}
                  </article>
                {/each}
              </div>
            {:else}
              <p class="home-body">No resolve graph nodes were exported.</p>
            {/if}
          </div>
        {:else}
          <p class="home-body">
            Resolve graph data is not available in this export.
          </p>
        {/if}
      </section>

      <section class="home-card">
        <h3>Getting Started</h3>
        <p class="home-body">
          Use the index to explore public items, or jump into a module to see
          its docs and cfg gates.
        </p>
        <div class="home-meta">
          <span>Total items</span>
          <strong>{items.length}</strong>
        </div>
      </section>
      <section class="home-card">
        <h3>Top Modules</h3>
        {#if topModules.length}
          <ul>
            {#each topModules as item}
              <li>
                <button class="link" on:click={() => onSelectItem(item)}>
                  <span>{item.path}</span>
                  <small>{formatSource(item)}</small>
                </button>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="home-body">No top-level modules detected yet.</p>
        {/if}
      </section>
      <section class="home-card">
        <h3>API Surface</h3>
        <div class="home-stats">
          <div>
            <span>Functions</span>
            <strong>{counts.Function ?? 0}</strong>
          </div>
          <div>
            <span>Structs</span>
            <strong>{counts.Struct ?? 0}</strong>
          </div>
          <div>
            <span>Enums</span>
            <strong>{counts.Enum ?? 0}</strong>
          </div>
          <div>
            <span>Traits</span>
            <strong>{counts.Trait ?? 0}</strong>
          </div>
        </div>
      </section>
      <section class="home-card">
        <h3>Highlights</h3>
        {#if prominentItems.length}
          <ul>
            {#each prominentItems as item}
              <li>
                <button class="link" on:click={() => onSelectItem(item)}>
                  <span>{item.path}</span>
                  <small>{formatSource(item)}</small>
                </button>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="home-body">Generate docs to see highlighted items.</p>
        {/if}
      </section>
    </div>
  </div>
</div>
