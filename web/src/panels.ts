/**
 * Detail panel and blast radius UI interactions.
 * M3 implementation: wire up node selection, blast radius, function node detail.
 */

import type { XrayNode, FunctionInfo, CycleInfo } from "./engine";

const nodePathEl = document.getElementById("node-path") as HTMLElement;
const nodeLangEl = document.getElementById("node-lang") as HTMLElement;
const nodeFnSig = document.getElementById("node-fn-sig") as HTMLElement | null;
const blastBtn = document.getElementById("blast-radius-btn") as HTMLButtonElement;
const statFiles = document.getElementById("stat-files") as HTMLElement;
const statEdges = document.getElementById("stat-edges") as HTMLElement;
const statFunctions = document.getElementById("stat-functions") as HTMLElement | null;

let selectedNode: XrayNode | null = null;

export function selectNode(node: XrayNode): void {
  selectedNode = node;
  nodePathEl.textContent = node.path;
  nodeLangEl.textContent = node.language;
  if (nodeFnSig) nodeFnSig.style.display = "none";
  blastBtn.disabled = false;
}

export function selectFunctionNode(fn: FunctionInfo): void {
  selectedNode = null;
  nodePathEl.textContent = `${fn.name}`;
  nodeLangEl.textContent = fn.kind === "Class" ? "class" : "fn";
  if (nodeFnSig) {
    nodeFnSig.textContent = fn.signature;
    nodeFnSig.style.display = "";
  }
  blastBtn.disabled = true;
}

export function clearSelection(): void {
  selectedNode = null;
  nodePathEl.textContent = "Select a node";
  nodeLangEl.textContent = "";
  if (nodeFnSig) nodeFnSig.style.display = "none";
  blastBtn.disabled = true;
}

export function updateStats(fileCount: number, edgeCount: number, functionCount?: number): void {
  statFiles.textContent = `${fileCount} files`;
  statEdges.textContent = `${edgeCount} edges`;
  if (statFunctions) {
    if (functionCount !== undefined) {
      statFunctions.textContent = `${functionCount} fns`;
      statFunctions.style.display = "";
    } else {
      statFunctions.style.display = "none";
    }
  }
}

let blastRadiusCallback: ((nodeId: string) => void) | null = null;

export function setBlastRadiusHandler(handler: (nodeId: string) => void): void {
  blastRadiusCallback = handler;
}

blastBtn.addEventListener("click", () => {
  if (!selectedNode || !blastRadiusCallback) return;
  blastRadiusCallback(selectedNode.id);
});

// ── Cycle panel ───────────────────────────────────────────────────────────────

export function updateCycleCount(count: number): void {
  let el = document.getElementById("stat-cycles");
  if (!el) {
    el = document.createElement("span");
    el.id = "stat-cycles";
    document.getElementById("graph-stats")?.appendChild(el);
  }
  el.textContent = `${count} cycles`;
  el.style.color = count > 0 ? "#ef4444" : "#22c55e";
  el.style.cursor = count > 0 ? "pointer" : "default";
}

export function setCycleClickHandler(handler: () => void): void {
  const el = document.getElementById("stat-cycles");
  if (el) el.addEventListener("click", handler);
}

export function showCyclePanel(
  cycles: CycleInfo[],
  getNodePath: (id: string) => string,
  onCycleSelect: (index: number) => void,
): void {
  hideCyclePanel();

  const panel = document.createElement("div");
  panel.id = "cycle-panel";
  panel.style.cssText = [
    "position: fixed",
    "bottom: 60px",
    "right: 20px",
    "max-height: 400px",
    "width: 420px",
    "overflow-y: auto",
    "background: #1a1d27",
    "border: 1px solid #2a2d3e",
    "border-radius: 8px",
    "z-index: 1000",
    "font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
    "font-size: 0.82rem",
    "color: #e2e8f0",
    "box-shadow: 0 8px 32px rgba(0,0,0,0.5)",
  ].join(";");

  // Header
  const header = document.createElement("div");
  header.style.cssText = [
    "display: flex",
    "align-items: center",
    "justify-content: space-between",
    "padding: 12px 16px",
    "border-bottom: 1px solid #2a2d3e",
    "font-weight: 600",
    "font-size: 0.85rem",
    "position: sticky",
    "top: 0",
    "background: #1a1d27",
  ].join(";");

  const title = document.createElement("span");
  title.textContent = `Cycles (${cycles.length} found)`;
  title.style.color = "#ef4444";

  const closeBtn = document.createElement("button");
  closeBtn.textContent = "✕";
  closeBtn.style.cssText = [
    "background: none",
    "border: none",
    "color: #64748b",
    "cursor: pointer",
    "font-size: 1rem",
    "padding: 0",
    "line-height: 1",
  ].join(";");
  closeBtn.addEventListener("click", hideCyclePanel);

  header.appendChild(title);
  header.appendChild(closeBtn);
  panel.appendChild(header);

  // Cycle list
  const list = document.createElement("ul");
  list.style.cssText = "list-style: none; padding: 8px 0; margin: 0;";

  cycles.forEach((cycle, i) => {
    const item = document.createElement("li");
    item.style.cssText = [
      "padding: 10px 16px",
      "cursor: pointer",
      "border-bottom: 1px solid #14172000",
    ].join(";");
    item.addEventListener("mouseenter", () => {
      item.style.background = "#0f1117";
    });
    item.addEventListener("mouseleave", () => {
      item.style.background = "";
    });

    const label = document.createElement("div");
    label.style.cssText = "color: #64748b; font-size: 0.75rem; margin-bottom: 3px;";
    label.textContent = `Cycle ${i + 1} — ${cycle.nodes.length} nodes`;

    const paths = document.createElement("div");
    paths.style.cssText = "font-family: monospace; color: #e2e8f0; word-break: break-all;";
    const nodeLabels = cycle.nodes.map((id) => {
      const path = getNodePath(id);
      return path.split("/").pop() ?? path;
    });
    paths.textContent = nodeLabels.join(" → ") + " → …";

    item.appendChild(label);
    item.appendChild(paths);
    item.addEventListener("click", () => onCycleSelect(i));
    list.appendChild(item);
  });

  panel.appendChild(list);
  document.body.appendChild(panel);
}

export function hideCyclePanel(): void {
  document.getElementById("cycle-panel")?.remove();
}
