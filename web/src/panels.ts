/**
 * Detail panel and blast radius UI interactions.
 * M2 implementation: wire up node selection, blast radius highlighting.
 */

import type { XrayNode } from "./engine";

const nodePathEl = document.getElementById("node-path") as HTMLElement;
const nodeLangEl = document.getElementById("node-lang") as HTMLElement;
const blastBtn = document.getElementById("blast-radius-btn") as HTMLButtonElement;
const statFiles = document.getElementById("stat-files") as HTMLElement;
const statEdges = document.getElementById("stat-edges") as HTMLElement;

let selectedNode: XrayNode | null = null;

export function selectNode(node: XrayNode): void {
  selectedNode = node;
  nodePathEl.textContent = node.path;
  nodeLangEl.textContent = node.language;
  blastBtn.disabled = false;
}

export function clearSelection(): void {
  selectedNode = null;
  nodePathEl.textContent = "Select a node";
  nodeLangEl.textContent = "";
  blastBtn.disabled = true;
}

export function updateStats(fileCount: number, edgeCount: number): void {
  statFiles.textContent = `${fileCount} files`;
  statEdges.textContent = `${edgeCount} edges`;
}

let blastRadiusCallback: ((nodeId: string) => void) | null = null;

export function setBlastRadiusHandler(handler: (nodeId: string) => void): void {
  blastRadiusCallback = handler;
}

blastBtn.addEventListener("click", () => {
  if (!selectedNode || !blastRadiusCallback) return;
  blastRadiusCallback(selectedNode.id);
});
