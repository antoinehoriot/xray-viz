/**
 * Xray UI Shell — entry point.
 * Fetches graph from /api/graph, renders with Sigma.js v3 + graphology.
 */

import Sigma from "sigma";
import type { NodeDisplayData, EdgeDisplayData } from "sigma/types";
import { EdgeArrowProgram, drawDiscNodeLabel } from "sigma/rendering";
import type { NodeHoverDrawingFunction } from "sigma/rendering";
import { Engine } from "./engine";
import type { XrayGraph } from "./engine";
import { selectNode, clearSelection, updateStats, setBlastRadiusHandler } from "./panels";

const DIM_COLOR = "#1e293b";
const HOVER_BG = "#1a1d27";

interface HighlightState {
  selected: string | null;
  deps: Set<string>;
  dependents: Set<string>;
  searchMatches: Set<string> | null;
}

let renderer: Sigma | null = null;
let engine: Engine | null = null;

const highlight: HighlightState = {
  selected: null,
  deps: new Set(),
  dependents: new Set(),
  searchMatches: null,
};

function isHighlightActive(): boolean {
  return highlight.selected !== null || highlight.searchMatches !== null;
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
  return originalColor;
}

function isNodeDimmed(node: string): boolean {
  if (highlight.selected !== null) {
    return node !== highlight.selected && !highlight.deps.has(node) && !highlight.dependents.has(node);
  }
  if (highlight.searchMatches !== null) {
    return !highlight.searchMatches.has(node);
  }
  return false;
}

function resetHighlight(): void {
  highlight.selected = null;
  highlight.deps = new Set();
  highlight.dependents = new Set();
  highlight.searchMatches = null;
}

// Custom hover renderer: uses dark surface background instead of white
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

async function init(): Promise<void> {
  const loading = document.getElementById("loading") as HTMLElement;

  try {
    const resp = await fetch("/api/graph");
    if (!resp.ok) {
      loading.textContent = `No graph loaded. Run \`xray .\` to scan your codebase. (HTTP ${resp.status})`;
      setupModeToggle();
      setupLayoutToggle();
      return;
    }
    const graphData: XrayGraph = await resp.json() as XrayGraph;

    engine = new Engine(graphData);
    engine.layoutForce();

    loading.style.display = "none";

    const container = document.getElementById("canvas-container") as HTMLElement;

    renderer = new Sigma(engine.graph, container, {
      labelColor: { color: "#e2e8f0" },
      defaultEdgeType: "arrow",
      edgeProgramClasses: { arrow: EdgeArrowProgram },
      defaultDrawNodeHover: darkNodeHover,
      nodeReducer: (node: string, data: Partial<NodeDisplayData>): Partial<NodeDisplayData> => {
        if (!engine || !isHighlightActive()) return data;
        const originalColor = engine.getOriginalColor(node);
        const color = getNodeColor(node, originalColor);
        if (isNodeDimmed(node)) {
          return { ...data, color, label: null };
        }
        return { ...data, color };
      },
      edgeReducer: (edge: string, data: Partial<EdgeDisplayData>): Partial<EdgeDisplayData> => {
        if (!engine || !isHighlightActive()) return data;
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

    renderer.on("clickNode", ({ node }: { node: string }) => {
      resetHighlight();
      highlight.selected = node;
      const nodeData = engine!.getNode(node);
      if (nodeData) selectNode(nodeData);
      renderer?.refresh();
    });

    renderer.on("clickStage", () => {
      resetHighlight();
      clearSelection();
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

    setupSearch();
    setupModeToggle();
    setupLayoutToggle();
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
      document.querySelectorAll(".mode-btn").forEach((b) => b.classList.remove("active"));
      (e.target as HTMLElement).classList.add("active");
      // Mode toggle is a UI-only state for now (file vs function level)
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
