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

export interface FunctionInfo {
  id: string;
  file_path: string;
  name: string;
  signature: string;
  kind: "Function" | "Class";
  line_start: number;
  line_end: number;
  calls: string[]; // IDs of functions this calls
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

function lightenColor(hex: string, factor: number): string {
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  const nr = Math.round(r + (255 - r) * factor);
  const ng = Math.round(g + (255 - g) * factor);
  const nb = Math.round(b + (255 - b) * factor);
  return `#${nr.toString(16).padStart(2, "0")}${ng.toString(16).padStart(2, "0")}${nb.toString(16).padStart(2, "0")}`;
}

export class Engine {
  readonly graph: Graph;
  private readonly graphData: XrayGraph;
  private readonly functionNodes: Map<string, FunctionInfo> = new Map();
  private readonly expandedFiles: Map<string, string[]> = new Map();

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
        isFunctionNode: false,
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

  addFunctionNodes(fileId: string, functions: FunctionInfo[]): void {
    if (!this.graph.hasNode(fileId) || functions.length === 0) return;

    const parentX = this.graph.getNodeAttribute(fileId, "x") as number;
    const parentY = this.graph.getNodeAttribute(fileId, "y") as number;
    const parentColor = this.graph.getNodeAttribute(fileId, "originalColor") as string;
    const language = this.graph.getNodeAttribute(fileId, "language") as string;
    const count = functions.length;
    const radius = 80;
    const addedIds: string[] = [];

    functions.forEach((fn, i) => {
      if (this.graph.hasNode(fn.id)) return;

      const angle = (2 * Math.PI * i) / count - Math.PI / 2;
      const x = parentX + radius * Math.cos(angle);
      const y = parentY + radius * Math.sin(angle);
      const color = lightenColor(parentColor, 0.5);
      const size = fn.kind === "Class" ? 10 : 7;

      this.graph.addNode(fn.id, {
        label: fn.name,
        path: fn.file_path,
        language,
        x,
        y,
        size,
        color,
        originalColor: color,
        isFunctionNode: true,
        parentFileId: fileId,
      });

      try {
        this.graph.addEdge(fileId, fn.id, {
          kind: "Contains",
          color: "#334155",
          size: 0.5,
        });
      } catch {
        // skip dup
      }

      this.functionNodes.set(fn.id, fn);
      addedIds.push(fn.id);
    });

    // Add call edges between function nodes after all are added
    for (const fn of functions) {
      for (const calleeId of fn.calls) {
        if (this.graph.hasNode(fn.id) && this.graph.hasNode(calleeId)) {
          try {
            this.graph.addEdge(fn.id, calleeId, {
              kind: "Call",
              color: EDGE_COLORS["Call"],
              size: 1,
            });
          } catch {
            // skip dup
          }
        }
      }
    }

    this.expandedFiles.set(fileId, addedIds);
  }

  removeFunctionNodes(fileId: string): void {
    const nodeIds = this.expandedFiles.get(fileId) ?? [];
    for (const id of nodeIds) {
      if (this.graph.hasNode(id)) {
        this.graph.dropNode(id);
      }
      this.functionNodes.delete(id);
    }
    this.expandedFiles.delete(fileId);
  }

  collapseAll(): void {
    for (const fileId of [...this.expandedFiles.keys()]) {
      this.removeFunctionNodes(fileId);
    }
  }

  hasExpandedFunctions(fileId: string): boolean {
    return this.expandedFiles.has(fileId);
  }

  isFunctionNode(id: string): boolean {
    return this.functionNodes.has(id);
  }

  getFunctionNode(id: string): FunctionInfo | null {
    return this.functionNodes.get(id) ?? null;
  }

  getStats(): GraphStats {
    return this.graphData.stats;
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
