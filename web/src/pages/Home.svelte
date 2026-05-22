<script lang="ts">
  import type { Counts, ItemDoc, PackageInfo, ResolveGraph } from "../types";
  import { renderMarkdown } from "../lib/markdown";
  import { l10nDerive } from "../l10n";

  export let packageInfo: PackageInfo | null = null;
  export let items: ItemDoc[] = [];
  export let counts: Counts = {};
  export let topModules: ItemDoc[] = [];
  export let prominentItems: ItemDoc[] = [];
  export let resolveGraph: ResolveGraph | null = null;
  export let onSelectItem: (item: ItemDoc) => void = () => {};

  $: l10n = $l10nDerive("$$_FILE_PATH_$$");

  let graphNodeById = new Map<string, ResolveGraph["nodes"][number]>();
  let graphIncoming: Record<string, number> | undefined;
  let graphOutgoing: Record<string, number> | undefined;

  $: renderedReadme = renderMarkdown(
    packageInfo?.readme || l10n("readme_not_loaded_err"),
  );

  const formatSource = (item: ItemDoc): string => {
    if (!item.source) return l10n("source_unavailable");

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
      <p class="kicker">{l10n("overview")}</p>
      <h2>{l10n("doc_home")}</h2>
    </div>
  </div>
  <div class="home-layout">
    <section class="home-card home-readme">
      <h3>{l10n("readme")}</h3>
      {#if packageInfo?.readme}
        <div class="markdown-body home-markdown">{@html renderedReadme}</div>
        <div class="home-meta">
          <span>{l10n("path")}</span>
          <strong>{packageInfo.readme_path ?? l10n("readme_default")}</strong>
        </div>
      {:else}
        <p class="home-body">{l10n("readme_not_loaded")}</p>
      {/if}
    </section>

    <div class="home-side">
      <section class="home-card home-graph">
        <div class="home-graph-header">
          <div>
            <h3>{l10n("resolve_graph")}</h3>
            <p class="home-body">
              {l10n("resolve_graph_desc")}
            </p>
          </div>
          {#if resolveGraph?.generated_by}
            <span class="graph-badge">{resolveGraph.generated_by}</span>
          {/if}
        </div>

        {#if resolveGraph}
          <div class="home-stats graph-stats">
            <div>
              <span>{l10n("roots")}</span>
              <strong>{resolveGraph.roots.length}</strong>
            </div>
            <div>
              <span>{l10n("nodes")}</span>
              <strong>{resolveGraph.nodes.length}</strong>
            </div>
            <div>
              <span>{l10n("edges")}</span>
              <strong>{resolveGraph.edges.length}</strong>
            </div>
            <div>
              <span>{l10n("workspace")}</span>
              <strong>
                {resolveGraph.nodes.filter((node) => node.workspace_member)
                  .length}
              </strong>
            </div>
          </div>

          <div class="graph-section">
            <h4>{l10n("roots_section")}</h4>
            {#if resolveGraph.roots.length}
              <div class="graph-pills">
                {#each resolveGraph.roots as rootId}
                  <span class="graph-pill" title={rootId}
                    >{resolveNodeLabel(rootId)}</span
                  >
                {/each}
              </div>
            {:else}
              <p class="home-body">{l10n("no_roots")}</p>
            {/if}
          </div>

          <div class="graph-section">
            <h4>{l10n("packages_section")}</h4>
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
                        <span class="graph-tag">{l10n("workspace_tag")}</span>
                      {/if}
                    </div>
                    <div class="graph-node-meta">
                      <span
                        >{l10n("incoming", {
                          n: graphIncoming?.[node.id] ?? 0,
                        })}</span
                      >
                      <span
                        >{l10n("outgoing", {
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
              <p class="home-body">{l10n("no_graph_nodes")}</p>
            {/if}
          </div>
        {:else}
          <p class="home-body">
            {l10n("no_graph_data")}
          </p>
        {/if}
      </section>

      <section class="home-card">
        <h3>{l10n("getting_started")}</h3>
        <p class="home-body">
          {l10n("getting_started_desc")}
        </p>
        <div class="home-meta">
          <span>{l10n("total_items")}</span>
          <strong>{items.length}</strong>
        </div>
      </section>
      <section class="home-card">
        <h3>{l10n("top_modules")}</h3>
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
          <p class="home-body">{l10n("no_top_modules")}</p>
        {/if}
      </section>
      <section class="home-card">
        <h3>{l10n("api_surface")}</h3>
        <div class="home-stats">
          <div>
            <span>{l10n("functions")}</span>
            <strong>{counts.Function ?? 0}</strong>
          </div>
          <div>
            <span>{l10n("structs")}</span>
            <strong>{counts.Struct ?? 0}</strong>
          </div>
          <div>
            <span>{l10n("enums")}</span>
            <strong>{counts.Enum ?? 0}</strong>
          </div>
          <div>
            <span>{l10n("traits")}</span>
            <strong>{counts.Trait ?? 0}</strong>
          </div>
        </div>
      </section>
      <section class="home-card">
        <h3>{l10n("highlights")}</h3>
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
          <p class="home-body">{l10n("no_highlights")}</p>
        {/if}
      </section>
    </div>
  </div>
</div>
