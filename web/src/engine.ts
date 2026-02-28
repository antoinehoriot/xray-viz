/**
 * XrayEngine WASM wrapper.
 * M2: import wasm-bindgen generated glue and wrap XrayEngine.
 */

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

/**
 * Thin wrapper around the XrayEngine WASM module.
 * M2: load wasm-bindgen glue and proxy calls to XrayEngine.
 */
export class Engine {
  // M2: private engine: import("../xray_wasm").XrayEngine;

  constructor(_graphJson: string) {
    // M2: this.engine = new WasmXrayEngine(graphJson);
  }

  getNodePositions(): Float32Array {
    // M2: return new Float32Array(this.engine.get_node_positions());
    return new Float32Array();
  }

  search(_query: string): number[] {
    // M2: return this.engine.search(query);
    return [];
  }

  blastRadius(_nodeIdx: number): number[] {
    return [];
  }

  reverseDeps(_nodeIdx: number): number[] {
    return [];
  }
}
