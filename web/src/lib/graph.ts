import dagre from "dagre";
import type { ResolveGraph } from "../types";

export const NODE_WIDTH = 240;
export const NODE_HEIGHT = 86;
export const MARGIN_X = 80;
export const DRAG_THRESHOLD = 5;
export const MIN_SCALE = -25;
export const MAX_SCALE = +25;

export interface LayoutNode {
  id: string;
  node: ResolveGraph["nodes"][number];
  x: number;
  y: number;
  layer: number;
  incoming: number;
  outgoing: number;
}

export interface LayoutEdge {
  from: string;
  to: string;
  kind: string;
  path: string;
}

export interface GraphLayoutResult {
  nodes: LayoutNode[];
  edges: LayoutEdge[];
  width: number;
  height: number;
}

export const formatNodeLabel = (node: ResolveGraph["nodes"][number]): string =>
  `${node.name} ${node.version}`;

export const resolveNodeLabel = (
  nodeId: string,
  nodeById: Map<string, ResolveGraph["nodes"][number]>,
): string => {
  const node = nodeById.get(nodeId);
  return node ? formatNodeLabel(node) : nodeId;
};

export const sortNodeIds = (
  left: string,
  right: string,
  nodeById: Map<string, ResolveGraph["nodes"][number]>,
): number =>
  resolveNodeLabel(left, nodeById).localeCompare(resolveNodeLabel(right, nodeById));

export const uniqueSorted = (
  values: string[],
  nodeById: Map<string, ResolveGraph["nodes"][number]>,
): string[] =>
  Array.from(new Set(values)).sort((a, b) => sortNodeIds(a, b, nodeById));

export const buildLayers = (
  startNodeId: string,
  direction: "upstream" | "downstream",
  outgoingMap: Map<string, string[]>,
  incomingMap: Map<string, string[]>,
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
        new Map(),
      );

      for (const neighborId of neighbors) {
        if (visited.has(neighborId)) continue;
        visited.add(neighborId);
        nextLayer.push(neighborId);
      }
    }

    if (nextLayer.length === 0) break;

    layers.push(uniqueSorted(nextLayer, new Map()));
    frontier = nextLayer;
  }

  return layers;
};

export const collectReachable = (
  startNodeId: string,
  direction: "upstream" | "downstream",
  outgoingMap: Map<string, string[]>,
  incomingMap: Map<string, string[]>,
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

    for (const neighborId of uniqueSorted(neighbors, new Map())) {
      if (visited.has(neighborId)) continue;
      visited.add(neighborId);
      collected.add(neighborId);
      queue.push(neighborId);
    }
  }

  return collected;
};

export const collectFocusSet = (
  startNodeId: string,
  outgoingMap: Map<string, string[]>,
  incomingMap: Map<string, string[]>,
): Set<string> => {
  const focus = new Set<string>([startNodeId]);
  const upLayers = buildLayers(startNodeId, "upstream", outgoingMap, incomingMap);
  const downLayers = buildLayers(startNodeId, "downstream", outgoingMap, incomingMap);
  const MAX_FOCUS_DEPTH = 2;
  for (let i = 0; i < Math.min(upLayers.length, MAX_FOCUS_DEPTH); i++) {
    for (const id of upLayers[i]) focus.add(id);
  }
  for (let i = 0; i < Math.min(downLayers.length, MAX_FOCUS_DEPTH); i++) {
    for (const id of downLayers[i]) focus.add(id);
  }
  return focus;
};

export const buildLayout = (
  resolveGraph: ResolveGraph | null,
  selectedNodeId: string | null,
  nodeById: Map<string, ResolveGraph["nodes"][number]>,
  incomingMap: Map<string, string[]>,
  outgoingMap: Map<string, string[]>,
): GraphLayoutResult => {
  const empty: GraphLayoutResult = { nodes: [], edges: [], width: 0, height: 0 };

  if (
    !resolveGraph ||
    nodeById.size === 0 ||
    resolveGraph.nodes.length === 0
  ) {
    return empty;
  }

  const focusSet =
    selectedNodeId && nodeById.has(selectedNodeId)
      ? collectFocusSet(selectedNodeId, outgoingMap, incomingMap)
      : new Set<string>();

  const g = new dagre.graphlib.Graph({ compound: false });
  g.setGraph({
    rankdir: "LR",
    nodesep: 40,
    ranksep: 120,
    marginx: 40,
    marginy: 40,
  });
  g.setDefaultEdgeLabel(() => ({}));

  for (const node of resolveGraph.nodes) {
    g.setNode(node.id, { width: NODE_WIDTH, height: NODE_HEIGHT });
  }

  for (const edge of resolveGraph.edges) {
    const bothInFocus =
      focusSet.size > 0 && focusSet.has(edge.from) && focusSet.has(edge.to);
    g.setEdge(edge.from, edge.to, { weight: bothInFocus ? 2 : 1 });
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
        incoming: incomingMap.get(node.id)?.length ?? 0,
        outgoing: outgoingMap.get(node.id)?.length ?? 0,
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

  const sortedNodes = nodeEntries.sort((a, b) => {
    if (a.layer !== b.layer) return a.layer - b.layer;
    return sortNodeIds(a.id, b.id, nodeById);
  });

  const sortedEdges = resolveGraph.edges
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

  if (sortedNodes.length === 0) {
    return empty;
  }

  const maxX = Math.max(...sortedNodes.map((n) => n.x + NODE_WIDTH), 0);
  const maxY = Math.max(...sortedNodes.map((n) => n.y + NODE_HEIGHT), 0);

  return {
    nodes: sortedNodes,
    edges: sortedEdges,
    width: maxX + MARGIN_X,
    height: Math.max(maxY + MARGIN_X, 400),
  };
};
