<script lang="ts">
  import type { ResolveGraph } from "../types";
  import dagre from "dagre";
  import { l10nDerive } from "../l10n";

  export let resolveGraph: ResolveGraph | null = null;
  export let onClose: () => void = () => {};

  $: l10n = $l10nDerive("$$_FILE_PATH_$$");

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
  let scale = 1;
  let dragStartX = 0;
  let dragStartY = 0;
  let dragOriginX = 0;
  let dragOriginY = 0;
  let isPanning = false;
  let didDrag = false;

  const DRAG_THRESHOLD = 5;
  const MIN_SCALE = 0.1;
  const MAX_SCALE = 5;
  const ZOOM_STEP = 0.1;

  const NODE_WIDTH = 240;
  const NODE_HEIGHT = 86;
  const MARGIN_X = 80;

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
    scale = 1;
  };

  const zoomIn = () => {
    const newScale = Math.min(MAX_SCALE, scale + ZOOM_STEP);
    const cx = graphCanvasWidth / 2;
    const cy = graphCanvasHeight / 2;
    panX = cx - (cx - panX) * (newScale / scale);
    panY = cy - (cy - panY) * (newScale / scale);
    scale = newScale;
  };

  const zoomOut = () => {
    const newScale = Math.max(MIN_SCALE, scale - ZOOM_STEP);
    const cx = graphCanvasWidth / 2;
    const cy = graphCanvasHeight / 2;
    panX = cx - (cx - panX) * (newScale / scale);
    panY = cy - (cy - panY) * (newScale / scale);
    scale = newScale;
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

  const collectFocusSet = (startNodeId: string): Set<string> => {
    const focus = new Set<string>([startNodeId]);
    const upLayers = buildLayers(startNodeId, "upstream");
    const downLayers = buildLayers(startNodeId, "downstream");
    const MAX_FOCUS_DEPTH = 2;
    for (let i = 0; i < Math.min(upLayers.length, MAX_FOCUS_DEPTH); i++) {
      for (const id of upLayers[i]) focus.add(id);
    }
    for (let i = 0; i < Math.min(downLayers.length, MAX_FOCUS_DEPTH); i++) {
      for (const id of downLayers[i]) focus.add(id);
    }
    return focus;
  };

  const buildLayout = () => {
    if (
      !resolveGraph ||
      graphNodeById.size === 0 ||
      resolveGraph.nodes.length === 0
    ) {
      graphLayoutNodes = [];
      graphLayoutEdges = [];
      graphCanvasWidth = 0;
      graphCanvasHeight = 0;
      return;
    }

    const focusSet =
      selectedNodeId && graphNodeById.has(selectedNodeId)
        ? collectFocusSet(selectedNodeId)
        : new Set<string>();

    const g = new dagre.graphlib.Graph({ compound: focusSet.size > 0 });
    g.setGraph({
      rankdir: "LR",
      nodesep: 60,
      ranksep: 280,
      marginx: 80,
      marginy: 80,
    });
    g.setDefaultEdgeLabel(() => ({}));

    if (focusSet.size > 0) {
      g.setNode("focus-cluster", { label: "" });
      g.setNode("other-cluster", { label: "" });
    }

    for (const node of resolveGraph.nodes) {
      g.setNode(node.id, { width: NODE_WIDTH, height: NODE_HEIGHT });
      if (focusSet.size > 0) {
        g.setParent(
          node.id,
          focusSet.has(node.id) ? "focus-cluster" : "other-cluster",
        );
      }
    }

    for (const edge of resolveGraph.edges) {
      const bothInFocus =
        focusSet.size > 0 && focusSet.has(edge.from) && focusSet.has(edge.to);
      g.setEdge(edge.from, edge.to, { weight: bothInFocus ? 10 : 1 });
    }

    dagre.layout(g);

    const nodeEntries = resolveGraph.nodes
      .map((node) => {
        const pos = g.node(node.id);
        if (!pos) return null;
        return {
          id: node.id,
          node,
          x: pos.x - NODE_WIDTH / 2,
          y: pos.y - NODE_HEIGHT / 2,
          layer: 0,
          incoming: graphIncoming?.[node.id] ?? 0,
          outgoing: graphOutgoing?.[node.id] ?? 0,
        };
      })
      .filter((n): n is NonNullable<typeof n> => n !== null);

    const uniqueXs = [...new Set(nodeEntries.map((n) => Math.round(n.x)))].sort(
      (a, b) => a - b,
    );
    const xToLayer = new Map(uniqueXs.map((x, i) => [x, i]));
    for (const n of nodeEntries) {
      n.layer = xToLayer.get(Math.round(n.x)) ?? 0;
    }

    graphLayoutNodes = nodeEntries.sort((a, b) => {
      if (a.layer !== b.layer) return a.layer - b.layer;
      return sortNodeIds(a.id, b.id);
    });

    graphLayoutEdges = resolveGraph.edges
      .map((edge) => {
        const edgeData = g.edge(edge.from, edge.to);
        if (!edgeData?.points || edgeData.points.length < 2) return null;

        const pts = edgeData.points;
        let path = `M ${pts[0].x} ${pts[0].y}`;
        for (let i = 1; i < pts.length; i++) {
          path += ` L ${pts[i].x} ${pts[i].y}`;
        }

        return { from: edge.from, to: edge.to, kind: edge.kind, path };
      })
      .filter((edge): edge is NonNullable<typeof edge> => Boolean(edge));

    if (nodeEntries.length === 0) {
      graphCanvasWidth = 0;
      graphCanvasHeight = 0;
      return;
    }

    const maxX = Math.max(...nodeEntries.map((n) => n.x + NODE_WIDTH), 0);
    const maxY = Math.max(...nodeEntries.map((n) => n.y + NODE_HEIGHT), 0);
    graphCanvasWidth = maxX + MARGIN_X;
    graphCanvasHeight = Math.max(maxY + MARGIN_X, 400);
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
    selectedNodeId;
    buildLayout();
  } else {
    graphLayoutNodes = [];
    graphLayoutEdges = [];
    graphCanvasWidth = 0;
    graphCanvasHeight = 0;
  }

  const onGraphPointerDown = (event: PointerEvent): void => {
    if (event.button !== 0) return;

    isPanning = true;
    didDrag = false;
    dragStartX = event.clientX;
    dragStartY = event.clientY;
    dragOriginX = panX;
    dragOriginY = panY;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  };

  const onGraphPointerMove = (event: PointerEvent): void => {
    if (!isPanning) return;
    const dx = event.clientX - dragStartX;
    const dy = event.clientY - dragStartY;
    if (
      !didDrag &&
      (Math.abs(dx) > DRAG_THRESHOLD || Math.abs(dy) > DRAG_THRESHOLD)
    ) {
      didDrag = true;
    }
    if (didDrag) {
      panX = dragOriginX + dx;
      panY = dragOriginY + dy;
    }
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
    if (!didDrag) {
      const el = document.elementFromPoint(event.clientX, event.clientY);
      const card = el?.closest("[data-node-id]");
      if (card) {
        const nodeId = card.getAttribute("data-node-id");
        if (nodeId) selectNode(nodeId);
      }
    }
  };

  const onGraphWheel = (event: WheelEvent): void => {
    event.preventDefault();
    const delta = event.deltaY > 0 ? -ZOOM_STEP : ZOOM_STEP;
    const newScale = Math.max(MIN_SCALE, Math.min(MAX_SCALE, scale + delta));

    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const mx = event.clientX - rect.left;
    const my = event.clientY - rect.top;

    panX = mx - (mx - panX) * (newScale / scale);
    panY = my - (my - panY) * (newScale / scale);
    scale = newScale;
  };
</script>

{#if resolveGraph}
  <div
    class="graph-overlay"
    role="dialog"
    aria-modal="true"
    aria-label={l10n("dep_graph_aria")}
  >
    <button
      class="graph-backdrop"
      type="button"
      aria-label={l10n("close_graph_aria")}
      on:click={onClose}
    ></button>
    <section class="graph-modal panel">
      <div class="graph-modal-header">
        <div>
          <p class="kicker">{l10n("dep_graph_header")}</p>
          <h2>{l10n("resolve_graph_header")}</h2>
          <p class="home-body">
            {l10n("graph_desc")}
          </p>
        </div>
        <div class="graph-modal-actions">
          <div
            class="graph-mode-switch"
            role="tablist"
            aria-label={l10n("graph_view_mode")}
          >
            <button
              class:active={mode === "list"}
              type="button"
              on:click={() => setMode("list")}
            >
              {l10n("list_mode")}
            </button>
            <button
              class:active={mode === "graph"}
              type="button"
              on:click={() => {
                setMode("graph");
                resetPan();
              }}
            >
              {l10n("graph_mode")}
            </button>
          </div>
          {#if mode === "graph"}
            <button class="ghost" type="button" on:click={zoomIn}
              >{l10n("zoom_in")}</button
            >
            <button class="ghost" type="button" on:click={zoomOut}
              >{l10n("zoom_out")}</button
            >
            <span class="graph-zoom-level">{Math.round(scale * 100)}%</span>
            <button class="ghost" type="button" on:click={resetPan}
              >{l10n("reset_view")}</button
            >
          {/if}
          <button class="ghost" type="button" on:click={onClose}
            >{l10n("close")}</button
          >
        </div>
      </div>

      <div class="graph-modal-stats home-stats graph-stats">
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
            {resolveGraph.nodes.filter((node) => node.workspace_member).length}
          </strong>
        </div>
      </div>

      {#if mode === "list"}
        <div class="graph-modal-body">
          <section class="graph-section">
            <h3>{l10n("roots_section")}</h3>
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
              <p class="home-body">{l10n("no_roots_graph")}</p>
            {/if}
          </section>

          <section class="graph-section graph-section-wide">
            <h3>{l10n("packages_section")}</h3>
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
                          .slice(0, 6) as edge}
                          <div class="graph-edge-row">
                            <span>{l10n("edge_" + edge.kind)}</span>
                            <span>{resolveNodeLabel(edge.to)}</span>
                          </div>
                        {/each}
                      </div>
                    {/if}
                  </button>
                {/each}
              </div>
            {:else}
              <p class="home-body">{l10n("no_packages_graph")}</p>
            {/if}
          </section>
        </div>
      {:else}
        <div
          class={`graph-viewport ${isPanning ? "graph-viewport--dragging" : ""}`}
          role="application"
          aria-label={l10n("dep_graph_aria")}
          on:pointerdown={onGraphPointerDown}
          on:pointermove={onGraphPointerMove}
          on:pointerup={onGraphPointerEnd}
          on:pointercancel={onGraphPointerEnd}
          on:wheel|nonpassive={onGraphWheel}
        >
          <div
            class="graph-surface"
            style={`width: ${graphCanvasWidth}px; height: ${graphCanvasHeight}px; transform: translate(${panX}px, ${panY}px) scale(${scale}); transform-origin: 0 0;`}
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
                data-node-id={layoutNode.id}
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
                    <span class="graph-node-tag">{l10n("workspace_tag")}</span>
                  {/if}
                </div>
                <div class="graph-node-card-meta">
                  <span>{l10n("layer_prefix", { n: layoutNode.layer })}</span>
                  <span>{l10n("incoming", { n: layoutNode.incoming })}</span>
                  <span>{l10n("outgoing", { n: layoutNode.outgoing })}</span>
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
                {l10n("selected_label")}{resolveNodeLabel(selectedNodeId)}
              </div>
            {/if}
          </div>
        </div>
      {/if}
    </section>
  </div>
{/if}
