<script lang="ts">
  import type { ResolveGraph } from "../types";

  export let resolveGraph: ResolveGraph | null = null;
  export let onClose: () => void = () => {};

  let mode: "list" | "graph" = "list";
  let selectedNodeId = "";
  let graphNodeById = new Map<string, ResolveGraph["nodes"][number]>();
  let graphIncoming: Record<string, number> | undefined;
  let graphOutgoing: Record<string, number> | undefined;
  let incomingMap = new Map<string, string[]>();
  let outgoingMap = new Map<string, string[]>();
  let graphLayoutNodes: Array<{
    id: string;
    node: ResolveGraph["nodes"][number];
    x: number;
    y: number;
    layer: number;
    incoming: number;
    outgoing: number;
  }> = [];
  let graphLayoutEdges: Array<{
    from: string;
    to: string;
    kind: string;
    path: string;
  }> = [];
  let graphCanvasWidth = 0;
  let graphCanvasHeight = 0;
  let selectedUpstreamNodeIds = new Set<string>();
  let selectedDownstreamNodeIds = new Set<string>();
  let panX = 40;
  let panY = 40;
  let dragStartX = 0;
  let dragStartY = 0;
  let dragOriginX = 0;
  let dragOriginY = 0;
  let isPanning = false;

  const NODE_WIDTH = 240;
  const NODE_HEIGHT = 86;
  const COLUMN_GAP = 280;
  const ROW_GAP = 120;
  const MARGIN_X = 80;
  const MARGIN_Y = 80;

  const formatNodeLabel = (node: ResolveGraph["nodes"][number]): string =>
    `${node.name} ${node.version}`;

  const resolveNodeLabel = (nodeId: string): string => {
    const node = graphNodeById.get(nodeId);
    return node ? formatNodeLabel(node) : nodeId;
  };

  const selectNode = (nodeId: string): void => {
    selectedNodeId = nodeId;
    mode = "graph";
  };

  const setMode = (nextMode: "list" | "graph"): void => {
    mode = nextMode;
  };

  const resetPan = (): void => {
    panX = 40;
    panY = 40;
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

  const sortNodeIds = (left: string, right: string): number =>
    resolveNodeLabel(left).localeCompare(resolveNodeLabel(right));

  const uniqueSorted = (values: string[]): string[] =>
    Array.from(new Set(values)).sort(sortNodeIds);

  const buildLayers = (
    startNodeId: string,
    direction: "upstream" | "downstream",
  ): string[][] => {
    if (!startNodeId) return [];

    const visited = new Set<string>([startNodeId]);
    let frontier = [startNodeId];
    const layers: string[][] = [];

    while (frontier.length > 0) {
      const nextLayer: string[] = [];

      for (const nodeId of frontier) {
        const neighbors = uniqueSorted(
          (direction === "downstream"
            ? outgoingMap.get(nodeId)
            : incomingMap.get(nodeId)) ?? [],
        );

        for (const neighborId of neighbors) {
          if (visited.has(neighborId)) continue;
          visited.add(neighborId);
          nextLayer.push(neighborId);
        }
      }

      if (nextLayer.length === 0) break;

      layers.push(uniqueSorted(nextLayer));
      frontier = nextLayer;
    }

    return layers;
  };

  const collectReachable = (
    startNodeId: string,
    direction: "upstream" | "downstream",
  ): Set<string> => {
    const visited = new Set<string>([startNodeId]);
    const collected = new Set<string>();
    const queue = [startNodeId];

    while (queue.length > 0) {
      const currentNodeId = queue.shift();
      if (!currentNodeId) continue;

      const neighbors =
        direction === "downstream"
          ? (outgoingMap.get(currentNodeId) ?? [])
          : (incomingMap.get(currentNodeId) ?? []);

      for (const neighborId of uniqueSorted(neighbors)) {
        if (visited.has(neighborId)) continue;
        visited.add(neighborId);
        collected.add(neighborId);
        queue.push(neighborId);
      }
    }

    return collected;
  };

  const buildLayout = () => {
    if (!resolveGraph || graphNodeById.size === 0) {
      graphLayoutNodes = [];
      graphLayoutEdges = [];
      graphCanvasWidth = 0;
      graphCanvasHeight = 0;
      return;
    }

    const depthMap = new Map<string, number>();
    const rootIds = uniqueSorted(
      (resolveGraph.roots.length
        ? resolveGraph.roots
        : resolveGraph.nodes
            .filter((node) => (incomingMap.get(node.id) ?? []).length === 0)
            .map((node) => node.id)
      ).filter((nodeId) => graphNodeById.has(nodeId)),
    );

    const fallbackRootId = rootIds[0] ?? resolveGraph.nodes[0]?.id;
    if (fallbackRootId) {
      depthMap.set(fallbackRootId, 0);
    }

    for (const rootId of rootIds) {
      depthMap.set(rootId, 0);
    }

    let changed = true;
    let guard = resolveGraph.nodes.length * 2;
    while (changed && guard-- > 0) {
      changed = false;

      for (const edge of resolveGraph.edges) {
        const sourceDepth = depthMap.get(edge.from);
        if (sourceDepth === undefined) continue;

        const nextDepth = sourceDepth + 1;
        const currentDepth = depthMap.get(edge.to);

        if (currentDepth === undefined || nextDepth < currentDepth) {
          depthMap.set(edge.to, nextDepth);
          changed = true;
        }
      }
    }

    const unresolvedIds = resolveGraph.nodes
      .map((node) => node.id)
      .filter((nodeId) => depthMap.get(nodeId) === undefined);

    for (const nodeId of unresolvedIds) {
      depthMap.set(nodeId, 0);
    }

    const layerMap = new Map<number, string[]>();
    let maxLayer = 0;

    for (const node of resolveGraph.nodes) {
      const layer = depthMap.get(node.id) ?? 0;
      maxLayer = Math.max(maxLayer, layer);
      const existing = layerMap.get(layer) ?? [];
      existing.push(node.id);
      layerMap.set(layer, existing);
    }

    const layers = Array.from({ length: maxLayer + 1 }, (_, layer) =>
      uniqueSorted(layerMap.get(layer) ?? []),
    );

    const nodePositions = new Map<
      string,
      { x: number; y: number; layer: number }
    >();
    let maxRows = 0;

    layers.forEach((layerNodes, layerIndex) => {
      maxRows = Math.max(maxRows, layerNodes.length);
      layerNodes.forEach((nodeId, rowIndex) => {
        nodePositions.set(nodeId, {
          x: MARGIN_X + layerIndex * COLUMN_GAP,
          y: MARGIN_Y + rowIndex * ROW_GAP,
          layer: layerIndex,
        });
      });
    });

    graphLayoutNodes = resolveGraph.nodes
      .map((node) => {
        const position = nodePositions.get(node.id) ?? {
          x: MARGIN_X,
          y: MARGIN_Y,
          layer: 0,
        };

        return {
          id: node.id,
          node,
          x: position.x,
          y: position.y,
          layer: position.layer,
          incoming: graphIncoming?.[node.id] ?? 0,
          outgoing: graphOutgoing?.[node.id] ?? 0,
        };
      })
      .sort((left, right) => {
        if (left.layer !== right.layer) return left.layer - right.layer;
        return sortNodeIds(left.id, right.id);
      });

    const positionById = new Map(
      graphLayoutNodes.map((entry) => [entry.id, entry]),
    );

    graphLayoutEdges = resolveGraph.edges
      .map((edge) => {
        const source = positionById.get(edge.from);
        const target = positionById.get(edge.to);
        if (!source || !target) return null;

        const sourceX = source.x + NODE_WIDTH;
        const sourceY = source.y + NODE_HEIGHT / 2;
        const targetX = target.x;
        const targetY = target.y + NODE_HEIGHT / 2;
        const controlOffset = Math.max(90, Math.abs(targetX - sourceX) / 2);
        const midX = sourceX + (targetX - sourceX) / 2;
        const path = `M ${sourceX} ${sourceY} C ${sourceX + controlOffset} ${sourceY}, ${targetX - controlOffset} ${targetY}, ${targetX} ${targetY}`;

        return {
          from: edge.from,
          to: edge.to,
          kind: edge.kind,
          path,
        };
      })
      .filter((edge): edge is NonNullable<typeof edge> => Boolean(edge));

    graphCanvasWidth = MARGIN_X * 2 + maxLayer * COLUMN_GAP + NODE_WIDTH;
    graphCanvasHeight = Math.max(
      MARGIN_Y * 2 + maxRows * ROW_GAP,
      MARGIN_Y * 2 + NODE_HEIGHT,
    );
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

  $: if (resolveGraph && mode === "graph") {
    const nodeId =
      selectedNodeId ||
      resolveGraph.roots[0] ||
      resolveGraph.nodes[0]?.id ||
      "";
    selectedUpstreamNodeIds = nodeId
      ? collectReachable(nodeId, "upstream")
      : new Set<string>();
    selectedDownstreamNodeIds = nodeId
      ? collectReachable(nodeId, "downstream")
      : new Set<string>();
  } else {
    selectedUpstreamNodeIds = new Set<string>();
    selectedDownstreamNodeIds = new Set<string>();
  }

  $: {
    const nextIncoming = new Map<string, string[]>();
    const nextOutgoing = new Map<string, string[]>();

    for (const edge of resolveGraph?.edges ?? []) {
      const incoming = nextIncoming.get(edge.to) ?? [];
      incoming.push(edge.from);
      nextIncoming.set(edge.to, incoming);

      const outgoing = nextOutgoing.get(edge.from) ?? [];
      outgoing.push(edge.to);
      nextOutgoing.set(edge.from, outgoing);
    }

    incomingMap = nextIncoming;
    outgoingMap = nextOutgoing;
  }

  $: if (resolveGraph) {
    const fallbackNodeId =
      resolveGraph.roots[0] ?? resolveGraph.nodes[0]?.id ?? "";
    if (!selectedNodeId || !graphNodeById.has(selectedNodeId)) {
      selectedNodeId = fallbackNodeId;
    }
  }

  $: if (resolveGraph && mode === "graph") {
    buildLayout();
  } else {
    graphLayoutNodes = [];
    graphLayoutEdges = [];
    graphCanvasWidth = 0;
    graphCanvasHeight = 0;
  }

  const onGraphPointerDown = (event: PointerEvent): void => {
    if (event.button !== 0) return;
    const target = event.target instanceof HTMLElement ? event.target : null;
    if (target?.closest("button")) return;

    isPanning = true;
    dragStartX = event.clientX;
    dragStartY = event.clientY;
    dragOriginX = panX;
    dragOriginY = panY;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  };

  const onGraphPointerMove = (event: PointerEvent): void => {
    if (!isPanning) return;
    panX = dragOriginX + (event.clientX - dragStartX);
    panY = dragOriginY + (event.clientY - dragStartY);
  };

  const onGraphPointerEnd = (event: PointerEvent): void => {
    if (!isPanning) return;
    isPanning = false;
    try {
      (event.currentTarget as HTMLElement).releasePointerCapture(
        event.pointerId,
      );
    } catch {
      /* ignore */
    }
  };
</script>

{#if resolveGraph}
  <div
    class="graph-overlay"
    role="dialog"
    aria-modal="true"
    aria-label="Dependency graph"
  >
    <button
      class="graph-backdrop"
      type="button"
      aria-label="Close dependency graph"
      on:click={onClose}
    ></button>
    <section class="graph-modal panel">
      <div class="graph-modal-header">
        <div>
          <p class="kicker">Dependency Graph</p>
          <h2>Resolve Graph</h2>
          <p class="home-body">
            Cargo metadata resolve graph exported from the workspace.
          </p>
        </div>
        <div class="graph-modal-actions">
          <div
            class="graph-mode-switch"
            role="tablist"
            aria-label="Graph view mode"
          >
            <button
              class:active={mode === "list"}
              type="button"
              on:click={() => setMode("list")}
            >
              列表式
            </button>
            <button
              class:active={mode === "graph"}
              type="button"
              on:click={() => {
                setMode("graph");
                resetPan();
              }}
            >
              图谱式
            </button>
          </div>
          {#if mode === "graph"}
            <button class="ghost" type="button" on:click={resetPan}
              >Reset view</button
            >
          {/if}
          <button class="ghost" type="button" on:click={onClose}>Close</button>
        </div>
      </div>

      <div class="graph-modal-stats home-stats graph-stats">
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
            {resolveGraph.nodes.filter((node) => node.workspace_member).length}
          </strong>
        </div>
      </div>

      {#if mode === "list"}
        <div class="graph-modal-body">
          <section class="graph-section">
            <h3>Roots</h3>
            {#if resolveGraph.roots.length}
              <div class="graph-pills">
                {#each resolveGraph.roots as rootId}
                  <button
                    class="graph-pill graph-pill-button"
                    type="button"
                    on:click={() => selectNode(rootId)}
                    title={rootId}
                  >
                    {resolveNodeLabel(rootId)}
                  </button>
                {/each}
              </div>
            {:else}
              <p class="home-body">No resolve roots available.</p>
            {/if}
          </section>

          <section class="graph-section graph-section-wide">
            <h3>Packages</h3>
            {#if resolveGraph.nodes.length}
              <div class="graph-list graph-list-modal">
                {#each resolveGraph.nodes as node}
                  <button
                    class:selected={selectedNodeId === node.id}
                    class="graph-node graph-node-button"
                    type="button"
                    on:click={() => selectNode(node.id)}
                  >
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
                          .slice(0, 6) as edge}
                          <div class="graph-edge-row">
                            <span>{edgeKindLabel(edge.kind)}</span>
                            <span>{resolveNodeLabel(edge.to)}</span>
                          </div>
                        {/each}
                      </div>
                    {/if}
                  </button>
                {/each}
              </div>
            {:else}
              <p class="home-body">No resolve graph nodes were exported.</p>
            {/if}
          </section>
        </div>
      {:else}
        <div
          class={`graph-viewport ${isPanning ? "graph-viewport--dragging" : ""}`}
          role="application"
          aria-label="Dependency graph canvas"
          on:pointerdown={onGraphPointerDown}
          on:pointermove={onGraphPointerMove}
          on:pointerup={onGraphPointerEnd}
          on:pointercancel={onGraphPointerEnd}
        >
          <div
            class="graph-surface"
            style={`width: ${graphCanvasWidth}px; height: ${graphCanvasHeight}px; transform: translate(${panX}px, ${panY}px);`}
          >
            <svg
              class="graph-links"
              width={graphCanvasWidth}
              height={graphCanvasHeight}
              aria-hidden="true"
            >
              {#each graphLayoutEdges as edge}
                <path
                  class={`graph-edge ${selectedUpstreamNodeIds.has(edge.from) ? "graph-edge--upstream" : selectedDownstreamNodeIds.has(edge.from) || edge.from === selectedNodeId ? "graph-edge--downstream" : "graph-edge--neutral"}`}
                  d={edge.path}
                ></path>
              {/each}
            </svg>

            {#each graphLayoutNodes as layoutNode}
              <button
                class={`graph-node-card ${selectedNodeId === layoutNode.id ? "graph-node-card--selected" : ""} ${selectedUpstreamNodeIds.has(layoutNode.id) ? "graph-node-card--upstream" : ""} ${selectedDownstreamNodeIds.has(layoutNode.id) ? "graph-node-card--downstream" : ""}`}
                type="button"
                style={`left: ${layoutNode.x}px; top: ${layoutNode.y}px; width: ${NODE_WIDTH}px; min-height: ${NODE_HEIGHT}px;`}
                on:click={() => selectNode(layoutNode.id)}
              >
                <div class="graph-node-card-head">
                  <div>
                    <strong>{layoutNode.node.name}</strong>
                    <div class="graph-node-card-subtitle">
                      {layoutNode.node.version}
                    </div>
                  </div>
                  {#if layoutNode.node.workspace_member}
                    <span class="graph-node-tag">workspace</span>
                  {/if}
                </div>
                <div class="graph-node-card-meta">
                  <span>layer {layoutNode.layer}</span>
                  <span>in {layoutNode.incoming}</span>
                  <span>out {layoutNode.outgoing}</span>
                </div>
                {#if layoutNode.node.source}
                  <div class="graph-node-card-source">
                    {layoutNode.node.source}
                  </div>
                {/if}
              </button>
            {/each}

            {#if selectedNodeId}
              <div
                class="graph-hint"
                style={`left: ${MARGIN_X}px; top: ${graphCanvasHeight - 44}px;`}
              >
                Selected: {resolveNodeLabel(selectedNodeId)}
              </div>
            {/if}
          </div>
        </div>
      {/if}
    </section>
  </div>
{/if}
