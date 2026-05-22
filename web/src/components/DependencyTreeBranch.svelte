<script lang="ts">
  import type { ResolveGraph } from "../types";
  import { l10nDerive } from "../l10n";

  export let resolveGraph: ResolveGraph;
  export let nodeId: string;
  export let direction: "upstream" | "downstream";
  export let selectedNodeId = "";
  export let onSelectNode: (nodeId: string) => void = () => {};
  export let edgeLabel = "";
  export let pathIds: string[] = [];

  $: l10n = $l10nDerive("$$_FILE_PATH_$$");

  const nodeById = new Map(resolveGraph.nodes.map((node) => [node.id, node]));

  const childrenFor = (
    currentNodeId: string,
  ): Array<{
    nodeId: string;
    edgeKind: string;
  }> => {
    if (direction === "downstream") {
      return resolveGraph.edges
        .filter((edge) => edge.from === currentNodeId)
        .map((edge) => ({ nodeId: edge.to, edgeKind: edge.kind }));
    }

    return resolveGraph.edges
      .filter((edge) => edge.to === currentNodeId)
      .map((edge) => ({ nodeId: edge.from, edgeKind: edge.kind }));
  };

  $: node = nodeById.get(nodeId) ?? null;
  $: children = childrenFor(nodeId);
  $: isCycle = pathIds.includes(nodeId);
  $: branchClass =
    direction === "downstream"
      ? "graph-tree-node--downstream"
      : "graph-tree-node--upstream";
  $: edgeClass =
    direction === "downstream"
      ? "graph-tree-edge--downstream"
      : "graph-tree-edge--upstream";
</script>

{#if node && !isCycle}
  <ul class={`graph-tree graph-tree--${direction}`}>
    <li>
      <div class="graph-tree-item">
        <button
          class={`graph-tree-node ${branchClass} ${selectedNodeId === nodeId ? "graph-tree-node--selected" : ""}`}
          type="button"
          on:click={() => onSelectNode(nodeId)}
        >
          <div class="graph-tree-node-head">
            <div>
              <strong>{node.name} {node.version}</strong>
              <div class="graph-tree-node-subtitle">{node.version}</div>
            </div>
            {#if node.workspace_member}
              <span class="graph-tree-tag">{l10n(":workspace_tag")}</span>
            {/if}
          </div>

          <div class="graph-tree-node-meta">
            <span
              >{direction === "downstream"
                ? l10n("depends_on")
                : l10n("depended_by")}</span
            >
            <span>{edgeLabel || l10n("edge_unknown")}</span>
            {#if node.source}
              <span>{node.source}</span>
            {/if}
          </div>
        </button>

        {#if children.length}
          <ul class={`graph-tree-children ${edgeClass}`}>
            {#each children as child}
              <li>
                <svelte:self
                  {resolveGraph}
                  nodeId={child.nodeId}
                  {direction}
                  {selectedNodeId}
                  {onSelectNode}
                  edgeLabel={child.edgeKind}
                  pathIds={[...pathIds, nodeId]}
                />
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    </li>
  </ul>
{/if}
