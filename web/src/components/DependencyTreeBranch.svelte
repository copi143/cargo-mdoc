<script lang="ts">
  import type { ResolveGraph } from "../types";

  export let resolveGraph: ResolveGraph;
  export let nodeId: string;
  export let direction: "upstream" | "downstream";
  export let selectedNodeId = "";
  export let onSelectNode: (nodeId: string) => void = () => {};
  export let edgeLabel = "";
  export let pathIds: string[] = [];

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
              <span class="graph-tree-tag">workspace</span>
            {/if}
          </div>

          <div class="graph-tree-node-meta">
            <span
              >{direction === "downstream" ? "depends on" : "depended by"}</span
            >
            <span>{edgeLabel || edgeKindLabel("unknown")}</span>
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
