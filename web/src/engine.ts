/**
 * Xray graph engine — pure TypeScript/graphology implementation.
 * Handles graph construction, layout, blast radius, and search.
 */

import Graph from "graphology";
import { circular } from "graphology-layout";
import forceAtlas2 from "graphology-layout-forceatlas2";

export interface XrayGraph {
  version: number;
  root: string;
  scanned_at: string;
  languages: string[];
  nodes: XrayNode[];
  edges: XrayEdge[];
  stats: GraphStats;
}

export interface XrayNode {
  id: string;
  kind: "File" | "Module" | "Function" | "Class" | "Interface";
  label: string;
  path: string;
  language: string;
  line_start: number | null;
  line_end: number | null;
  loc: number | null;
  metadata: NodeMetadata;
}

export interface NodeMetadata {
  exports: string[];
  is_entry_point: boolean;
  tags: string[];
}

export interface XrayEdge {
  id: string;
  source: string;
  target: string;
  kind: "Import" | "DynamicImport" | "Call" | "Inheritance" | "ReExport";
  symbol: string | null;
  resolved: boolean;
}

export interface GraphStats {
  file_count: number;
  function_count: number;
  edge_count: number;
  parse_duration_ms: number;
  cache_hits: number;
}

const LANG_COLORS: Record<string, string> = {
  typescript: "#3b82f6",
  python: "#f97316",
  rust: "#22c55e",
  go: "#06b6d4",
  java: "#ef4444",
};

const EDGE_COLORS: Record<string, string> = {
  Import: "#64748b",
  DynamicImport: "#64748b",
  Call: "#3b82f6",
  Inheritance: "#a855f7",
  ReExport: "#94a3b8",
};

function nodeSize(loc: number | null): number {
  if (!loc || loc <= 0) return 8;
  return Math.min(Math.max(8 + (Math.log(loc + 1) / Math.log(2001)) * 24, 8), 32);
}

export class Engine {
  readonly graph: Graph;
  private readonly graphData: XrayGraph;

  constructor(graphData: XrayGraph) {
    this.graphData = graphData;
    this.graph = new Graph({ type: "directed", multi: false });

    for (const node of graphData.nodes) {
      const color = LANG_COLORS[node.language] ?? "#94a3b8";
      this.graph.addNode(node.id, {
        label: node.label,
        path: node.path,
        language: node.language,
        x: Math.random(),
        y: Math.random(),
        size: nodeSize(node.loc),
        color,
        originalColor: color,
      });
    }

    for (const edge of graphData.edges) {
      if (!this.graph.hasNode(edge.source) || !this.graph.hasNode(edge.target)) {
        continue;
      }
      try {
        this.graph.addEdge(edge.source, edge.target, {
          kind: edge.kind,
          color: EDGE_COLORS[edge.kind] ?? "#475569",
          size: 1,
        });
      } catch {
        // Skip duplicate edges
      }
    }
  }

  layoutForce(): void {
    if (this.graph.order === 0) return;
    circular.assign(this.graph);
    forceAtlas2.assign(this.graph, { iterations: 100 });
  }

  layoutHierarchical(): void {
    if (this.graph.order === 0) return;

    const dirMap = new Map<string, string[]>();
    this.graph.nodes().forEach((id) => {
      const path = this.graph.getNodeAttribute(id, "path") as string;
      const dir = path.includes("/") ? path.substring(0, path.lastIndexOf("/")) : ".";
      if (!dirMap.has(dir)) dirMap.set(dir, []);
      dirMap.get(dir)!.push(id);
    });

    const dirs = [...dirMap.keys()].sort();
    const rowHeight = 200;
    const colWidth = 100;

    dirs.forEach((dir, rowIdx) => {
      const nodes = dirMap.get(dir)!;
      nodes.forEach((id, colIdx) => {
        this.graph.setNodeAttribute(id, "x", (colIdx - nodes.length / 2) * colWidth);
        this.graph.setNodeAttribute(id, "y", rowIdx * rowHeight);
      });
    });
  }

  blastRadius(nodeId: string): string[] {
    if (!this.graph.hasNode(nodeId)) return [];
    const visited = new Set<string>();
    const queue: string[] = [nodeId];
    while (queue.length > 0) {
      const curr = queue.shift()!;
      if (visited.has(curr)) continue;
      visited.add(curr);
      for (const neighbor of this.graph.outNeighbors(curr)) {
        if (!visited.has(neighbor)) queue.push(neighbor);
      }
    }
    visited.delete(nodeId);
    return [...visited];
  }

  reverseDeps(nodeId: string): string[] {
    if (!this.graph.hasNode(nodeId)) return [];
    const visited = new Set<string>();
    const queue: string[] = [nodeId];
    while (queue.length > 0) {
      const curr = queue.shift()!;
      if (visited.has(curr)) continue;
      visited.add(curr);
      for (const neighbor of this.graph.inNeighbors(curr)) {
        if (!visited.has(neighbor)) queue.push(neighbor);
      }
    }
    visited.delete(nodeId);
    return [...visited];
  }

  search(query: string): string[] {
    if (!query) return [];
    const q = query.toLowerCase();
    return this.graph.nodes().filter((id) => {
      const label = (this.graph.getNodeAttribute(id, "label") as string ?? "").toLowerCase();
      const path = (this.graph.getNodeAttribute(id, "path") as string ?? "").toLowerCase();
      return label.includes(q) || path.includes(q);
    });
  }

  getNode(id: string): XrayNode | null {
    return this.graphData.nodes.find((n) => n.id === id) ?? null;
  }

  getOriginalColor(id: string): string {
    if (!this.graph.hasNode(id)) return "#94a3b8";
    return this.graph.getNodeAttribute(id, "originalColor") as string;
  }
}
