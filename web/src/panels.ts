/**
 * Detail panel and blast radius UI interactions.
 * M3 implementation: wire up node selection, blast radius, function node detail.
 */

import type { XrayNode, FunctionInfo } from "./engine";

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
