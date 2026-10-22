<script lang="ts">
  import type { ResolveGraph } from "../../types";
  import {
    buildLayout,
    collectReachable,
    formatNodeLabel,
    resolveNodeLabel,
    NODE_WIDTH,
    NODE_HEIGHT,
    MARGIN_X,
    DRAG_THRESHOLD,
    MIN_SCALE,
    MAX_SCALE,
  } from "../../lib/graph";

  export let resolveGraph: ResolveGraph | null = null;
  export let onClose: () => void = () => {};


  let mode: "list" | "graph" = "list";
  let selectedNodeId = "";
  let layout: ReturnType<typeof buildLayout> | null = null;
  let incomingMap: Map<string, string[]> = new Map();
  let outgoingMap: Map<string, string[]> = new Map();

  let panX = 40;
  let panY = 40;
  let scale_step = 0;
  let dragStartX = 0;
  let dragStartY = 0;
  let dragOriginX = 0;
  let dragOriginY = 0;
  let isPanning = false;
  let didDrag = false;

  $: scale = Math.pow(1.1, scale_step);

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
    scale_step = 0;
  };

  const zoomIn = () => {
    if (scale_step >= MAX_SCALE) return;
    scale_step += 1;
    const cx = layoutCanvasWidth / 2;
    const cy = layoutCanvasHeight / 2;
    const newScale = Math.pow(1.1, scale_step);
    panX = cx - (cx - panX) * (newScale / scale);
    panY = cy - (cy - panY) * (newScale / scale);
  };

  const zoomOut = () => {
    if (scale_step <= MIN_SCALE) return;
    scale_step -= 1;
    const cx = layoutCanvasWidth / 2;
    const cy = layoutCanvasHeight / 2;
    const newScale = Math.pow(1.1, scale_step);
    panX = cx - (cx - panX) * (newScale / scale);
    panY = cy - (cy - panY) * (newScale / scale);
  };

  $: nodeById = new Map(
    resolveGraph?.nodes.map((node) => [node.id, node]) ?? [],
  );

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
    if (!selectedNodeId || !nodeById.has(selectedNodeId)) {
      selectedNodeId = fallbackNodeId;
    }
  }

  $: layout = resolveGraph && mode === "graph" ? buildLayout(resolveGraph, selectedNodeId, nodeById, incomingMap, outgoingMap) : null;

  $: selectedUpstreamNodeIds = layout && selectedNodeId
    ? collectReachable(selectedNodeId, "upstream", outgoingMap, incomingMap)
    : new Set<string>();
  $: selectedDownstreamNodeIds = layout && selectedNodeId
    ? collectReachable(selectedNodeId, "downstream", outgoingMap, incomingMap)
    : new Set<string>();

  $: layoutCanvasWidth = layout?.width ?? 0;
  $: layoutCanvasHeight = layout?.height ?? 0;

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
    const delta = Math.sign(-event.deltaY);
    if (delta === 0) return;
    if (delta > 0 && scale_step >= MAX_SCALE) return;
    if (delta < 0 && scale_step <= MIN_SCALE) return;
    scale_step += delta;
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const mx = event.clientX - rect.left;
    const my = event.clientY - rect.top;
    const newScale = Math.pow(1.1, scale_step);
    panX = mx - (mx - panX) * (newScale / scale);
    panY = my - (my - panY) * (newScale / scale);
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
            {l10n(":graph_desc")}
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

      {#if mode === "list"}
        <div class="graph-modal-body">
          <section class="graph-section">
            <h3>{l10n(":roots_section")}</h3>
            {#if resolveGraph.roots.length}
              <div class="graph-pills">
                {#each resolveGraph.roots as rootId}
                  <button
                    class="graph-pill graph-pill-button"
                    type="button"
                    on:click={() => selectNode(rootId)}
                    title={rootId}
                  >
                    {resolveNodeLabel(rootId, nodeById)}
                  </button>
                {/each}
              </div>
            {:else}
              <p class="home-body">{l10n(":no_roots_graph")}</p>
            {/if}
          </section>

          <section class="graph-section graph-section-wide">
            <h3>{l10n(":packages_section")}</h3>
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
                        <span class="graph-tag">{l10n(":workspace_tag")}</span>
                      {/if}
                    </div>
                    <div class="graph-node-meta">
                      <span
                        >{l10n(":incoming", {
                          n: (incomingMap.get(node.id) ?? []).length,
                        })}</span
                      >
                      <span
                        >{l10n(":outgoing", {
                          n: (outgoingMap.get(node.id) ?? []).length,
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
                            <span>{resolveNodeLabel(edge.to, nodeById)}</span>
                          </div>
                        {/each}
                      </div>
                    {/if}
                  </button>
                {/each}
              </div>
            {:else}
              <p class="home-body">{l10n(":no_packages_graph")}</p>
            {/if}
          </section>
        </div>
      {:else}
        {#if layout}
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
              style={`width: ${layout.width}px; height: ${layout.height}px; transform: translate(${panX}px, ${panY}px) scale(${scale}); transform-origin: 0 0;`}
            >
              <svg
                class="graph-links"
                width={layout.width}
                height={layout.height}
                aria-hidden="true"
              >
                {#each layout.edges as edge}
                  <path
                    class={`graph-edge ${selectedUpstreamNodeIds.has(edge.from) ? "graph-edge--upstream" : selectedDownstreamNodeIds.has(edge.from) || edge.from === selectedNodeId ? "graph-edge--downstream" : "graph-edge--neutral"}`}
                    d={edge.path}
                  ></path>
                {/each}
              </svg>

              {#each layout.nodes as layoutNode}
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
                      <span class="graph-node-tag">{l10n(":workspace_tag")}</span>
                    {/if}
                  </div>
                  <div class="graph-node-card-meta">
                    <span>{l10n("layer_prefix", { n: layoutNode.layer })}</span>
                    <span>{l10n(":incoming", { n: layoutNode.incoming })}</span>
                    <span>{l10n(":outgoing", { n: layoutNode.outgoing })}</span>
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
                  style={`left: ${MARGIN_X}px; top: ${layout.height - 44}px;`}
                >
                  {l10n("selected_label")}{resolveNodeLabel(selectedNodeId, nodeById)}
                </div>
              {/if}
            </div>
          </div>
        {/if}
      {/if}
    </section>
  </div>
{/if}
