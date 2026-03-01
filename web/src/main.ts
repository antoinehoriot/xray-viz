/**
 * Xray UI Shell — entry point.
 * Fetches graph from /api/graph, renders with Sigma.js v3 + graphology.
 */

import Sigma from "sigma";
import type { NodeDisplayData, EdgeDisplayData } from "sigma/types";
import { EdgeArrowProgram, drawDiscNodeLabel } from "sigma/rendering";
import type { NodeHoverDrawingFunction } from "sigma/rendering";
import { Engine } from "./engine";
import type { XrayGraph, FunctionInfo, CycleInfo } from "./engine";
import {
  selectNode,
  clearSelection,
  updateStats,
  setBlastRadiusHandler,
  selectFunctionNode,
  updateCycleCount,
  setCycleClickHandler,
  showCyclePanel,
  hideCyclePanel,
  updateOrphanCount,
  setOrphanClickHandler,
  showOrphanPanel,
  hideOrphanPanel,
  showDirectoryPanel,
  hideDirectoryPanel,
  displayNodeTags,
  showAnnotationEditor,
  hideAnnotationEditor,
} from "./panels";

const DEV_MODE = true;

const DIM_COLOR = "#1e293b";
const HOVER_BG = "#1a1d27";

interface HighlightState {
  selected: string | null;
  deps: Set<string>;
  dependents: Set<string>;
  searchMatches: Set<string> | null;
  cycleNodes: Set<string> | null;
  cycleEdges: Set<string> | null;
  orphanNodes: Set<string> | null;
  directoryNodes: Set<string> | null;
}

let renderer: Sigma | null = null;
let engine: Engine | null = null;
let currentMode: "file" | "function" = "file";
let graphStats = { fileCount: 0, edgeCount: 0 };

const highlight: HighlightState = {
  selected: null,
  deps: new Set(),
  dependents: new Set(),
  searchMatches: null,
  cycleNodes: null,
  cycleEdges: null,
  orphanNodes: null,
  directoryNodes: null,
};

// ── Pro license ───────────────────────────────────────────────────────────────

function getLicenseKey(): string | null {
  return localStorage.getItem("xray_pro_key");
}

function setLicenseKey(key: string): void {
  localStorage.setItem("xray_pro_key", key);
}

function showProGateModal(): void {
  const overlay = document.getElementById("pro-gate-overlay");
  if (overlay) overlay.style.display = "flex";
  const input = document.getElementById("pro-key-input") as HTMLInputElement | null;
  if (input) input.value = "";
}

function hideProGateModal(): void {
  const overlay = document.getElementById("pro-gate-overlay");
  if (overlay) overlay.style.display = "none";
}

function setupProGate(): void {
  const submitBtn = document.getElementById("pro-key-submit");
  const closeBtn = document.getElementById("pro-gate-close");
  const overlay = document.getElementById("pro-gate-overlay");

  submitBtn?.addEventListener("click", () => {
    const input = document.getElementById("pro-key-input") as HTMLInputElement | null;
    const key = input?.value.trim() ?? "";
    if (!key) return;
    setLicenseKey(key);
    hideProGateModal();
    // Switch to Fn mode now that we have a key
    activateFnMode();
  });

  closeBtn?.addEventListener("click", hideProGateModal);

  overlay?.addEventListener("click", (e) => {
    if (e.target === overlay) hideProGateModal();
  });

  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      hideProGateModal();
      hideAnnotationEditor();
    }
  });
}

// ── Highlight helpers ─────────────────────────────────────────────────────────

function isHighlightActive(): boolean {
  return (
    highlight.selected !== null ||
    highlight.searchMatches !== null ||
    highlight.cycleNodes !== null ||
    highlight.orphanNodes !== null ||
    highlight.directoryNodes !== null
  );
}

function getNodeColor(node: string, originalColor: string): string {
  if (highlight.selected !== null) {
    if (node === highlight.selected) return "#fbbf24";
    if (highlight.deps.has(node)) return "#22c55e";
    if (highlight.dependents.has(node)) return "#ef4444";
    return DIM_COLOR;
  }
  if (highlight.searchMatches !== null) {
    if (highlight.searchMatches.has(node)) return originalColor;
    return DIM_COLOR;
  }
  if (highlight.cycleNodes !== null) {
    if (highlight.cycleNodes.has(node)) return "#ef4444";
    return DIM_COLOR;
  }
  if (highlight.orphanNodes !== null) {
    if (highlight.orphanNodes.has(node)) return "#f59e0b";
    return DIM_COLOR;
  }
  if (highlight.directoryNodes !== null) {
    if (highlight.directoryNodes.has(node)) return "#06b6d4";
    return DIM_COLOR;
  }
  return originalColor;
}

function isNodeDimmed(node: string): boolean {
  if (highlight.selected !== null) {
    return node !== highlight.selected && !highlight.deps.has(node) && !highlight.dependents.has(node);
  }
  if (highlight.searchMatches !== null) {
    return !highlight.searchMatches.has(node);
  }
  if (highlight.cycleNodes !== null) {
    return !highlight.cycleNodes.has(node);
  }
  if (highlight.orphanNodes !== null) {
    return !highlight.orphanNodes.has(node);
  }
  if (highlight.directoryNodes !== null) {
    return !highlight.directoryNodes.has(node);
  }
  return false;
}

function resetHighlight(): void {
  highlight.selected = null;
  highlight.deps = new Set();
  highlight.dependents = new Set();
  highlight.searchMatches = null;
  highlight.cycleNodes = null;
  highlight.cycleEdges = null;
  highlight.orphanNodes = null;
  highlight.directoryNodes = null;
}

// ── Custom hover renderer ─────────────────────────────────────────────────────

const darkNodeHover: NodeHoverDrawingFunction = (context, data, settings) => {
  const size = settings.labelSize;
  const font = settings.labelFont;
  const weight = settings.labelWeight;
  context.font = `${weight} ${size}px ${font}`;

  context.fillStyle = HOVER_BG;
  context.shadowOffsetX = 0;
  context.shadowOffsetY = 0;
  context.shadowBlur = 8;
  context.shadowColor = "#000";

  const PADDING = 2;
  if (typeof data.label === "string") {
    const textWidth = context.measureText(data.label).width;
    const boxWidth = Math.round(textWidth + 5);
    const boxHeight = Math.round(size + 2 * PADDING);
    const radius = Math.max(data.size, size / 2) + PADDING;
    const angleRadian = Math.asin(boxHeight / 2 / radius);
    const xDeltaCoord = Math.sqrt(Math.abs(Math.pow(radius, 2) - Math.pow(boxHeight / 2, 2)));
    context.beginPath();
    context.moveTo(data.x + xDeltaCoord, data.y + boxHeight / 2);
    context.lineTo(data.x + radius + boxWidth, data.y + boxHeight / 2);
    context.lineTo(data.x + radius + boxWidth, data.y - boxHeight / 2);
    context.lineTo(data.x + xDeltaCoord, data.y - boxHeight / 2);
    context.arc(data.x, data.y, radius, angleRadian, -angleRadian);
    context.closePath();
    context.fill();
  } else {
    context.beginPath();
    context.arc(data.x, data.y, data.size + PADDING, 0, Math.PI * 2);
    context.closePath();
    context.fill();
  }

  context.shadowOffsetX = 0;
  context.shadowOffsetY = 0;
  context.shadowBlur = 0;

  drawDiscNodeLabel(context, data, settings);
};

// ── Mock function data (fallback when backend not ready) ──────────────────────

function generateMockFunctions(fileId: string, path: string, language: string): FunctionInfo[] {
  const namesByLang: Record<string, Array<{ name: string; kind: "Function" | "Class" }>> = {
    typescript: [
      { name: "init", kind: "Function" },
      { name: "render", kind: "Function" },
      { name: "update", kind: "Function" },
      { name: "cleanup", kind: "Function" },
    ],
    python: [
      { name: "__init__", kind: "Function" },
      { name: "process", kind: "Function" },
      { name: "validate", kind: "Function" },
      { name: "run", kind: "Function" },
    ],
    rust: [
      { name: "new", kind: "Function" },
      { name: "parse", kind: "Function" },
      { name: "render", kind: "Function" },
      { name: "validate", kind: "Function" },
    ],
    go: [
      { name: "New", kind: "Function" },
      { name: "Handle", kind: "Function" },
      { name: "Close", kind: "Function" },
    ],
    java: [
      { name: "Main", kind: "Class" },
      { name: "getInstance", kind: "Function" },
      { name: "execute", kind: "Function" },
    ],
  };

  const entries = namesByLang[language] ?? namesByLang["typescript"]!;
  return entries.map((entry, i) => ({
    id: `${fileId}::${entry.name}`,
    file_path: path,
    name: entry.name,
    signature: `${entry.name}()`,
    kind: entry.kind,
    line_start: 1 + i * 20,
    line_end: 15 + i * 20,
    calls: i > 0 ? [`${fileId}::${entries[i - 1]!.name}`] : [],
  }));
}

// ── Function expansion ────────────────────────────────────────────────────────

async function expandFileNode(fileId: string): Promise<void> {
  if (!engine) return;
  const nodeData = engine.getNode(fileId);
  if (!nodeData) return;

  const licenseKey = DEV_MODE ? null : getLicenseKey();
  const url = `/api/functions?file=${encodeURIComponent(nodeData.path)}${licenseKey ? `&license=${encodeURIComponent(licenseKey)}` : ""}`;

  let functions: FunctionInfo[];

  try {
    const resp = await fetch(url);

    if (!DEV_MODE && resp.status === 402) {
      showProGateModal();
      return;
    }

    if (resp.ok) {
      const body = (await resp.json()) as {
        file: string;
        functions: { name: string; line_start: number; line_end: number; calls: { callee: string; resolved_node_id: string | null; line: number }[]; is_exported: boolean; is_async: boolean }[];
        classes: { name: string; line_start: number; line_end: number }[];
      };
      const fnNodes: FunctionInfo[] = body.functions.map((f) => ({
        id: `${fileId}::${f.name}`,
        file_path: body.file,
        name: f.name,
        signature: `${f.name}()`,
        kind: "Function",
        line_start: f.line_start,
        line_end: f.line_end,
        calls: f.calls.map((c) => c.callee),
      }));
      const classNodes: FunctionInfo[] = body.classes.map((c) => ({
        id: `${fileId}::${c.name}`,
        file_path: body.file,
        name: c.name,
        signature: c.name,
        kind: "Class",
        line_start: c.line_start,
        line_end: c.line_end,
        calls: [],
      }));
      functions = [...fnNodes, ...classNodes];
    } else {
      // Backend not ready — use mock data
      functions = generateMockFunctions(fileId, nodeData.path, nodeData.language);
    }
  } catch {
    // Network error — use mock data
    functions = generateMockFunctions(fileId, nodeData.path, nodeData.language);
  }

  engine.addFunctionNodes(fileId, functions);
  updateStats(graphStats.fileCount, graphStats.edgeCount, functions.length);
  renderer?.refresh();
}

// ── Mode management ───────────────────────────────────────────────────────────

function activateFnMode(): void {
  document.querySelectorAll(".mode-btn").forEach((b) => b.classList.remove("active"));
  document.querySelector('.mode-btn[data-mode="function"]')?.classList.add("active");
  currentMode = "function";
}

function activateFileMode(): void {
  if (!engine) return;

  // Collapse all expanded function nodes
  engine.collapseAll();

  // Clear selection if it was on a function node
  if (highlight.selected && engine.isFunctionNode(highlight.selected)) {
    resetHighlight();
    clearSelection();
  }

  currentMode = "file";
  updateStats(graphStats.fileCount, graphStats.edgeCount);
  renderer?.refresh();
}

// ── Renderer creation ─────────────────────────────────────────────────────────

function createRenderer(graphData: XrayGraph): void {
  // Destroy previous renderer if any
  if (renderer) {
    renderer.kill();
    renderer = null;
  }

  engine = new Engine(graphData);
  engine.layoutForce();

  graphStats = {
    fileCount: graphData.stats.file_count,
    edgeCount: graphData.stats.edge_count,
  };

  const container = document.getElementById("canvas-container") as HTMLElement;

  renderer = new Sigma(engine.graph, container, {
    labelColor: { color: "#e2e8f0" },
    defaultEdgeType: "arrow",
    edgeProgramClasses: { arrow: EdgeArrowProgram },
    defaultDrawNodeHover: darkNodeHover,
    nodeReducer: (node: string, data: Partial<NodeDisplayData>): Partial<NodeDisplayData> => {
      if (!engine || !isHighlightActive()) return data;
      const originalColor = engine.getOriginalColor(node);
      const annotation = engine.getAnnotation(node);
      // Dim deprecated nodes even when not in highlight mode
      const isDeprecated = annotation?.status === "deprecated";
      const color = getNodeColor(node, originalColor);
      if (isNodeDimmed(node)) {
        return { ...data, color: isDeprecated ? "#3d4255" : color, label: null };
      }
      return { ...data, color: isDeprecated ? "#4a5068" : color };
    },
    edgeReducer: (edge: string, data: Partial<EdgeDisplayData>): Partial<EdgeDisplayData> => {
      if (!engine || !isHighlightActive()) return data;
      if (highlight.cycleEdges !== null) {
        if (highlight.cycleEdges.has(edge)) {
          return { ...data, color: "#ef4444", size: 2 };
        }
        return { ...data, hidden: true };
      }
      if (highlight.selected !== null) {
        const src = engine.graph.source(edge);
        const tgt = engine.graph.target(edge);
        const srcHighlighted = src === highlight.selected || highlight.deps.has(src) || highlight.dependents.has(src);
        const tgtHighlighted = tgt === highlight.selected || highlight.deps.has(tgt) || highlight.dependents.has(tgt);
        if (!srcHighlighted || !tgtHighlighted) {
          return { ...data, hidden: true };
        }
      }
      return data;
    },
  });

  renderer.on("clickNode", async ({ node }: { node: string }) => {
    if (!engine) return;

    if (currentMode === "function" && !engine.isFunctionNode(node)) {
      // File node clicked in Fn mode → expand or collapse
      resetHighlight();
      if (engine.hasExpandedFunctions(node)) {
        engine.removeFunctionNodes(node);
        clearSelection();
        updateStats(graphStats.fileCount, graphStats.edgeCount);
      } else {
        highlight.selected = node;
        await expandFileNode(node);
      }
      renderer?.refresh();
      return;
    }

    resetHighlight();
    highlight.selected = node;

    if (engine.isFunctionNode(node)) {
      const fnData = engine.getFunctionNode(node);
      if (fnData) selectFunctionNode(fnData);
    } else {
      const nodeData = engine.getNode(node);
      if (nodeData) {
        selectNode(nodeData);
        // Display annotations/tags for this node
        const annotation = engine.getAnnotation(node);
        const allTags = [...(nodeData.metadata.tags ?? []), ...(annotation?.tags ?? [])];
        if (allTags.length > 0) {
          displayNodeTags(allTags);
        }
      }
    }

    renderer?.refresh();
  });

  renderer.on("clickStage", () => {
    resetHighlight();
    clearSelection();
    hideCyclePanel();
    hideOrphanPanel();
    renderer?.refresh();
  });

  updateStats(graphData.stats.file_count, graphData.stats.edge_count);

  setBlastRadiusHandler((nodeId: string) => {
    if (!engine) return;
    highlight.deps = new Set(engine.blastRadius(nodeId));
    highlight.dependents = new Set(engine.reverseDeps(nodeId));
    highlight.selected = nodeId;
    renderer?.refresh();
  });
}

// ── Cycle detection ────────────────────────────────────────────────────────────

function setupCycles(): void {
  if (!engine || !renderer) return;

  const cycles: CycleInfo[] = engine.detectCycles();
  updateCycleCount(cycles.length);

  if (cycles.length > 0) {
    setCycleClickHandler(() => {
      if (highlight.cycleNodes !== null) {
        // Toggle off
        resetHighlight();
        hideCyclePanel();
        renderer?.refresh();
        return;
      }
      // Highlight all cycle nodes/edges and show panel
      highlight.cycleNodes = engine!.getCycleNodes();
      highlight.cycleEdges = engine!.getCycleEdges();
      showCyclePanel(
        cycles,
        (id) => engine!.getNode(id)?.path ?? id,
        (index) => {
          const cycle = cycles[index];
          if (cycle) {
            highlight.cycleNodes = new Set(cycle.nodes);
            highlight.cycleEdges = new Set(cycle.edges);
            renderer?.refresh();
          }
        },
      );
      renderer?.refresh();
    });
  }
}

// ── Orphan detection ───────────────────────────────────────────────────────────

function setupOrphans(): void {
  if (!engine || !renderer) return;

  const orphanIds = engine.detectOrphans();
  updateOrphanCount(orphanIds.length);

  if (orphanIds.length > 0) {
    setOrphanClickHandler(() => {
      if (highlight.orphanNodes !== null) {
        // Toggle off
        resetHighlight();
        hideOrphanPanel();
        renderer?.refresh();
        return;
      }
      // Highlight all orphan nodes and show panel
      highlight.orphanNodes = new Set(orphanIds);
      showOrphanPanel(
        orphanIds,
        (id) => engine!.getNode(id)?.path ?? id,
      );
      renderer?.refresh();
    });
  }
}

// ── Directory grouping ────────────────────────────────────────────────────────

function setupDirectoryPanel(): void {
  const btn = document.getElementById("dir-group-btn");
  if (!btn) return;

  let panelOpen = false;

  btn.addEventListener("click", () => {
    if (!engine) return;

    if (panelOpen) {
      hideDirectoryPanel();
      resetHighlight();
      renderer?.refresh();
      panelOpen = false;
      btn.classList.remove("active");
      return;
    }

    panelOpen = true;
    btn.classList.add("active");

    const groups = engine.getDirectoryGroups();
    const edgeCounts = engine.getInterModuleEdges();

    showDirectoryPanel(groups, edgeCounts, (dir, nodeIds) => {
      resetHighlight();
      highlight.directoryNodes = new Set(nodeIds);
      renderer?.refresh();
      void dir;
    });
  });
}

// ── Annotations ───────────────────────────────────────────────────────────────

function setupAnnotations(): void {
  const btn = document.getElementById("annotate-btn");
  if (!btn) return;

  btn.addEventListener("click", () => {
    if (!engine || !highlight.selected) return;
    const nodeId = highlight.selected;
    const annotation = engine.getAnnotation(nodeId);
    const currentTags = annotation?.tags ?? [];

    showAnnotationEditor(nodeId, currentTags, (newTags) => {
      if (!engine) return;
      engine.setAnnotation(nodeId, { tags: newTags });
      // Refresh displayed tags
      const nodeData = engine.getNode(nodeId);
      const allTags = [...(nodeData?.metadata.tags ?? []), ...newTags];
      displayNodeTags(allTags);
      renderer?.refresh();
    });
  });
}

// ── Snapshot export ───────────────────────────────────────────────────────────

function generateSnapshotHTML(graphData: XrayGraph, eng: Engine): string {
  const snapshot = eng.getSnapshotData();
  const snapshotJson = JSON.stringify(snapshot);
  const graphJson = JSON.stringify({
    root: graphData.root,
    scanned_at: graphData.scanned_at,
    languages: graphData.languages,
    stats: graphData.stats,
  });

  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Xray Snapshot — ${graphData.root}</title>
  <script src="https://unpkg.com/graphology@0.25.4/dist/graphology.umd.js"><\/script>
  <script src="https://unpkg.com/graphology-layout@0.6.1/dist/graphology-layout.min.js"><\/script>
  <script src="https://unpkg.com/sigma@3.0.0/dist/sigma.min.js"><\/script>
  <style>
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body { background: #0f1117; color: #e2e8f0; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; height: 100vh; display: flex; flex-direction: column; }
    header { padding: 10px 16px; background: #1a1d27; border-bottom: 1px solid #2a2d3e; display: flex; align-items: center; gap: 12px; font-size: 0.85rem; }
    #logo { font-weight: 700; color: #6366f1; font-size: 1rem; }
    .meta { color: #64748b; font-size: 0.78rem; }
    #container { flex: 1; }
  </style>
</head>
<body>
  <header>
    <span id="logo">Xray</span>
    <span class="meta">Snapshot · ${new Date().toISOString().split("T")[0]}</span>
    <span class="meta" id="meta-root"></span>
  </header>
  <div id="container"></div>
  <script>
    const SNAPSHOT = ${snapshotJson};
    const META = ${graphJson};
    document.getElementById('meta-root').textContent = META.root + ' · ' + META.stats.file_count + ' files · ' + META.stats.edge_count + ' edges';

    const Graph = graphology.Graph;
    const graph = new Graph({ type: 'directed', multi: false });
    for (const n of SNAPSHOT.nodes) {
      graph.addNode(n.id, { label: n.label, x: n.x, y: n.y, color: n.color, size: n.size });
    }
    for (const e of SNAPSHOT.edges) {
      try { graph.addEdge(e.source, e.target, { color: e.color, size: 1 }); } catch (_) {}
    }

    const { Sigma } = sigma;
    new Sigma(graph, document.getElementById('container'), {
      labelColor: { color: '#e2e8f0' },
      defaultEdgeType: 'arrow',
    });
  <\/script>
</body>
</html>`;
}

function triggerDownload(content: string, filename: string, mime: string): void {
  const blob = new Blob([content], { type: mime });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

function setupExport(): void {
  const exportBtn = document.getElementById("export-btn");
  const dropdown = document.getElementById("export-dropdown");

  exportBtn?.addEventListener("click", (e) => {
    e.stopPropagation();
    if (dropdown) {
      dropdown.style.display = dropdown.style.display === "none" ? "flex" : "none";
    }
  });

  document.addEventListener("click", () => {
    if (dropdown) dropdown.style.display = "none";
  });

  document.getElementById("export-json")?.addEventListener("click", () => {
    if (!engine) return;
    const graphData = engine.getGraphData();
    triggerDownload(JSON.stringify(graphData, null, 2), "xray-graph.json", "application/json");
    if (dropdown) dropdown.style.display = "none";
  });

  document.getElementById("export-html")?.addEventListener("click", () => {
    if (!engine) return;
    const graphData = engine.getGraphData();
    const html = generateSnapshotHTML(graphData, engine);
    triggerDownload(html, "xray-snapshot.html", "text/html");
    if (dropdown) dropdown.style.display = "none";
  });
}

// ── Hot reload via SSE ────────────────────────────────────────────────────────

async function reloadGraph(): Promise<void> {
  try {
    const resp = await fetch("/api/graph");
    if (!resp.ok) return;
    const graphData: XrayGraph = await resp.json() as XrayGraph;
    resetHighlight();
    clearSelection();
    createRenderer(graphData);
    currentMode = "file";
    document.querySelectorAll(".mode-btn").forEach((b) => b.classList.remove("active"));
    document.querySelector('.mode-btn[data-mode="file"]')?.classList.add("active");
    console.log("[xray] graph reloaded");
  } catch (err) {
    console.warn("[xray] failed to reload graph", err);
  }
}

function setupSSE(): void {
  const evtSource = new EventSource("/api/events");
  evtSource.addEventListener("graph_updated", () => {
    void reloadGraph();
  });
  evtSource.onerror = () => {
    console.warn("[xray] SSE connection lost, will retry...");
  };
}

// ── Init ──────────────────────────────────────────────────────────────────────

async function init(): Promise<void> {
  const loading = document.getElementById("loading") as HTMLElement;

  try {
    const resp = await fetch("/api/graph");
    if (!resp.ok) {
      loading.textContent = `No graph loaded. Run \`xray .\` to scan your codebase. (HTTP ${resp.status})`;
      setupModeToggle();
      setupLayoutToggle();
      setupProGate();
      setupExport();
      return;
    }
    const graphData: XrayGraph = await resp.json() as XrayGraph;

    loading.style.display = "none";
    createRenderer(graphData);

    // Load annotations from backend (silently ignored if unavailable)
    if (engine) await engine.loadAnnotations();

    setupSearch();
    setupModeToggle();
    setupLayoutToggle();
    setupProGate();
    setupCycles();
    setupOrphans();
    setupDirectoryPanel();
    setupAnnotations();
    setupExport();
    setupSSE();
  } catch (err) {
    loading.textContent = `Error loading graph: ${String(err)}`;
  }
}

function setupSearch(): void {
  const input = document.getElementById("search") as HTMLInputElement;
  input.addEventListener("input", () => {
    const query = input.value.trim();
    if (!engine || !renderer) return;
    if (!query) {
      highlight.searchMatches = null;
    } else {
      highlight.searchMatches = new Set(engine.search(query));
    }
    renderer.refresh();
  });
}

function setupModeToggle(): void {
  document.querySelectorAll(".mode-btn").forEach((btn) => {
    btn.addEventListener("click", (e) => {
      const target = e.target as HTMLElement;
      const mode = target.dataset["mode"] as "file" | "function";
      if (mode === currentMode) return;

      if (mode === "function") {
        if (!DEV_MODE && !getLicenseKey()) {
          showProGateModal();
          return;
        }
        document.querySelectorAll(".mode-btn").forEach((b) => b.classList.remove("active"));
        target.classList.add("active");
        currentMode = "function";
      } else {
        document.querySelectorAll(".mode-btn").forEach((b) => b.classList.remove("active"));
        target.classList.add("active");
        activateFileMode();
      }
    });
  });
}

function setupLayoutToggle(): void {
  document.querySelectorAll(".layout-btn").forEach((btn) => {
    btn.addEventListener("click", (e) => {
      document.querySelectorAll(".layout-btn").forEach((b) => b.classList.remove("active"));
      (e.target as HTMLElement).classList.add("active");
      const layout = (e.target as HTMLElement).dataset["layout"];
      if (!engine || !renderer) return;
      if (layout === "hierarchical") {
        engine.layoutHierarchical();
      } else {
        engine.layoutForce();
      }
      renderer.refresh();
    });
  });
}

document.addEventListener("DOMContentLoaded", () => { void init(); });
