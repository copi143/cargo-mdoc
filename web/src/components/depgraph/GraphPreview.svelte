<script lang="ts">
  import type { ResolveGraph } from "../../types";

  export let resolveGraph: ResolveGraph | null = null;

  let graphNodeById = new Map<string, ResolveGraph["nodes"][number]>();
  let graphIncoming: Record<string, number> | undefined;
  let graphOutgoing: Record<string, number> | undefined;

  $: graphNodeById = new Map(resolveGraph?.nodes.map((node) => [node.id, node]) ?? []);

  $: graphIncoming = resolveGraph?.edges.reduce<Record<string, number>>((acc, edge) => {
    acc[edge.to] = (acc[edge.to] ?? 0) + 1;
    return acc;
  }, {});

  $: graphOutgoing = resolveGraph?.edges.reduce<Record<string, number>>((acc, edge) => {
    acc[edge.from] = (acc[edge.from] ?? 0) + 1;
    return acc;
  }, {});

  const resolveNodeLabel = (nodeId: string): string => {
    const node = graphNodeById.get(nodeId);
    return node ? `${node.name} ${node.version}` : nodeId;
  };
</script>

<section class="home-card home-graph">
  <div class="home-graph-header">
    <div>
      <h3>{l10n("resolve_graph")}</h3>
      <p class="home-body">
        {l10n(":graph_desc")}
      </p>
    </div>
    {#if resolveGraph?.generated_by}
      <span class="graph-badge">{resolveGraph.generated_by}</span>
    {/if}
  </div>

  {#if resolveGraph}
    <div class="home-stats graph-stats">
      <div>
        <span>{l10n(":roots")}</span>
        <strong>{resolveGraph.roots.length}</strong>
      </div>
      <div>
        <span>{l10n(":nodes")}</span>
        <strong>{resolveGraph.nodes.length}</strong>
      </div>
      <div>
        <span>{l10n(":edges")}</span>
        <strong>{resolveGraph.edges.length}</strong>
      </div>
      <div>
        <span>{l10n(":workspace")}</span>
        <strong>
          {resolveGraph.nodes.filter((node) => node.workspace_member).length}
        </strong>
      </div>
    </div>

    <div class="graph-section">
      <h4>{l10n(":roots_section")}</h4>
      {#if resolveGraph.roots.length}
        <div class="graph-pills">
          {#each resolveGraph.roots as rootId}
            <span class="graph-pill" title={rootId}>{resolveNodeLabel(rootId)}</span>
          {/each}
        </div>
      {:else}
        <p class="home-body">{l10n(":no_roots_graph")}</p>
      {/if}
    </div>

    <div class="graph-section">
      <h4>{l10n(":packages_section")}</h4>
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
                  <span class="graph-tag">{l10n(":workspace_tag")}</span>
                {/if}
              </div>
              <div class="graph-node-meta">
                <span
                  >{l10n(":incoming", {
                    n: graphIncoming?.[node.id] ?? 0,
                  })}</span
                >
                <span
                  >{l10n(":outgoing", {
                    n: graphOutgoing?.[node.id] ?? 0,
                  })}</span
                >
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
                      <span>{l10n("edge_" + edge.kind)}</span>
                      <span>{resolveNodeLabel(edge.to)}</span>
                    </div>
                  {/each}
                </div>
              {/if}
            </article>
          {/each}
        </div>
      {:else}
        <p class="home-body">{l10n(":no_packages_graph")}</p>
      {/if}
    </div>
  {:else}
    <p class="home-body">
      {l10n("no_graph_data")}
    </p>
  {/if}
</section>
