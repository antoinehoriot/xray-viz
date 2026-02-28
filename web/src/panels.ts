/**
 * Detail panel and blast radius UI interactions.
 * M3 implementation: wire up node selection, blast radius, function node detail.
 */

import type { XrayNode, FunctionInfo, CycleInfo, DirectoryGroup } from "./engine";

const nodePathEl = document.getElementById("node-path") as HTMLElement;
const nodeLangEl = document.getElementById("node-lang") as HTMLElement;
const nodeFnSig = document.getElementById("node-fn-sig") as HTMLElement | null;
const blastBtn = document.getElementById("blast-radius-btn") as HTMLButtonElement;
const annotateBtn = document.getElementById("annotate-btn") as HTMLButtonElement | null;
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
  if (annotateBtn) annotateBtn.disabled = false;
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
  if (annotateBtn) annotateBtn.disabled = true;
}

export function clearSelection(): void {
  selectedNode = null;
  nodePathEl.textContent = "Select a node";
  nodeLangEl.textContent = "";
  if (nodeFnSig) nodeFnSig.style.display = "none";
  blastBtn.disabled = true;
  if (annotateBtn) annotateBtn.disabled = true;
  clearNodeTags();
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

// ── Orphan panel ──────────────────────────────────────────────────────────────

export function updateOrphanCount(count: number): void {
  let el = document.getElementById("stat-orphans");
  if (!el) {
    el = document.createElement("span");
    el.id = "stat-orphans";
    document.getElementById("graph-stats")?.appendChild(el);
  }
  el.textContent = `${count} orphans`;
  el.style.color = count > 0 ? "#f59e0b" : "#64748b";
  el.style.cursor = count > 0 ? "pointer" : "default";
}

export function setOrphanClickHandler(handler: () => void): void {
  const el = document.getElementById("stat-orphans");
  if (el) el.addEventListener("click", handler);
}

export function showOrphanPanel(
  orphanIds: string[],
  getNodePath: (id: string) => string,
): void {
  hideOrphanPanel();

  const panel = document.createElement("div");
  panel.id = "orphan-panel";
  panel.style.cssText = [
    "position: fixed",
    "bottom: 60px",
    "right: 460px",
    "max-height: 400px",
    "width: 380px",
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
  title.textContent = `Orphan Files (${orphanIds.length})`;
  title.style.color = "#f59e0b";

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
  closeBtn.addEventListener("click", hideOrphanPanel);

  header.appendChild(title);
  header.appendChild(closeBtn);
  panel.appendChild(header);

  const desc = document.createElement("div");
  desc.style.cssText = "padding: 8px 16px; color: #64748b; font-size: 0.75rem; border-bottom: 1px solid #2a2d3e;";
  desc.textContent = "Files not imported by any other module";
  panel.appendChild(desc);

  const list = document.createElement("ul");
  list.style.cssText = "list-style: none; padding: 8px 0; margin: 0;";

  orphanIds.forEach((id) => {
    const item = document.createElement("li");
    item.style.cssText = [
      "padding: 8px 16px",
      "cursor: default",
      "font-family: monospace",
      "font-size: 0.78rem",
      "color: #f59e0b",
      "word-break: break-all",
    ].join(";");
    item.addEventListener("mouseenter", () => { item.style.background = "#0f1117"; });
    item.addEventListener("mouseleave", () => { item.style.background = ""; });
    item.textContent = getNodePath(id);
    list.appendChild(item);
  });

  panel.appendChild(list);
  document.body.appendChild(panel);
}

export function hideOrphanPanel(): void {
  document.getElementById("orphan-panel")?.remove();
}

// ── Directory panel ───────────────────────────────────────────────────────────

export function showDirectoryPanel(
  groups: DirectoryGroup[],
  edgeCounts: Array<{ from: string; to: string; count: number }>,
  onGroupClick: (dir: string, nodeIds: string[]) => void,
): void {
  hideDirectoryPanel();

  const panel = document.createElement("div");
  panel.id = "directory-panel";
  panel.style.cssText = [
    "position: fixed",
    "top: 50px",
    "right: 20px",
    "max-height: 500px",
    "width: 360px",
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
  title.textContent = `Directory Groups (${groups.length})`;
  title.style.color = "#06b6d4";

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
  closeBtn.addEventListener("click", hideDirectoryPanel);

  header.appendChild(title);
  header.appendChild(closeBtn);
  panel.appendChild(header);

  // Build inter-module edge count per source directory
  const interEdgeMap = new Map<string, number>();
  for (const e of edgeCounts) {
    interEdgeMap.set(e.from, (interEdgeMap.get(e.from) ?? 0) + e.count);
  }

  const list = document.createElement("ul");
  list.style.cssText = "list-style: none; padding: 8px 0; margin: 0;";

  const sortedGroups = [...groups].sort((a, b) => b.nodeIds.length - a.nodeIds.length);
  let activeItem: HTMLElement | null = null;

  for (const group of sortedGroups) {
    const item = document.createElement("li");
    item.style.cssText = [
      "padding: 10px 16px",
      "cursor: pointer",
      "border-bottom: 1px solid #2a2d3e22",
      "border-left: 2px solid transparent",
    ].join(";");
    item.addEventListener("mouseenter", () => { if (item !== activeItem) item.style.background = "#0f1117"; });
    item.addEventListener("mouseleave", () => { if (item !== activeItem) item.style.background = ""; });

    const dirName = document.createElement("div");
    dirName.style.cssText = "font-family: monospace; color: #06b6d4; font-size: 0.8rem; margin-bottom: 3px;";
    dirName.textContent = group.directory;

    const meta = document.createElement("div");
    meta.style.cssText = "color: #64748b; font-size: 0.72rem;";
    const interEdges = interEdgeMap.get(group.directory) ?? 0;
    meta.textContent = `${group.nodeIds.length} files · ${interEdges} cross-module edges`;

    item.appendChild(dirName);
    item.appendChild(meta);
    item.addEventListener("click", () => {
      if (activeItem && activeItem !== item) {
        activeItem.style.background = "";
        activeItem.style.borderLeft = "2px solid transparent";
      }
      activeItem = item;
      item.style.background = "#0f1117";
      item.style.borderLeft = "2px solid #06b6d4";
      onGroupClick(group.directory, group.nodeIds);
    });
    list.appendChild(item);
  }

  panel.appendChild(list);
  document.body.appendChild(panel);
}

export function hideDirectoryPanel(): void {
  document.getElementById("directory-panel")?.remove();
}

// ── Annotation UI ─────────────────────────────────────────────────────────────

function clearNodeTags(): void {
  document.getElementById("node-tags-row")?.remove();
}

export function displayNodeTags(tags: string[]): void {
  clearNodeTags();
  if (tags.length === 0) return;

  const nodeDetail = document.getElementById("node-detail");
  if (!nodeDetail) return;

  const row = document.createElement("div");
  row.id = "node-tags-row";
  row.style.cssText = "display: flex; align-items: center; gap: 4px; flex-wrap: wrap;";

  for (const tag of tags) {
    const chip = document.createElement("span");
    chip.className = "tag-chip";
    chip.textContent = tag;
    row.appendChild(chip);
  }

  nodeDetail.appendChild(row);
}

export function showAnnotationEditor(
  nodeId: string,
  currentTags: string[],
  onSave: (tags: string[]) => void,
): void {
  hideAnnotationEditor();

  const editor = document.createElement("div");
  editor.id = "annotation-editor";
  editor.className = "annotation-editor";

  // Header
  const header = document.createElement("div");
  header.style.cssText = "display: flex; align-items: center; justify-content: space-between; margin-bottom: 12px;";

  const title = document.createElement("div");
  title.style.cssText = "font-size: 0.85rem; font-weight: 600; color: #e2e8f0;";
  title.textContent = "Annotations";

  const closeBtn = document.createElement("button");
  closeBtn.textContent = "✕";
  closeBtn.style.cssText = "background: none; border: none; color: #64748b; cursor: pointer; font-size: 1rem; padding: 0;";
  closeBtn.addEventListener("click", hideAnnotationEditor);

  header.appendChild(title);
  header.appendChild(closeBtn);
  editor.appendChild(header);

  // Tag chips area
  const tags = [...currentTags];

  const chipsRow = document.createElement("div");
  chipsRow.style.cssText = "display: flex; flex-wrap: wrap; gap: 4px; min-height: 28px; margin-bottom: 10px;";

  const renderChips = (): void => {
    while (chipsRow.firstChild) {
      chipsRow.removeChild(chipsRow.firstChild);
    }
    for (const tag of tags) {
      const chip = document.createElement("span");
      chip.className = "tag-chip";
      const label = document.createElement("span");
      label.textContent = tag;
      const removeBtn = document.createElement("span");
      removeBtn.className = "remove";
      removeBtn.textContent = "×";
      removeBtn.addEventListener("click", () => {
        const idx = tags.indexOf(tag);
        if (idx !== -1) tags.splice(idx, 1);
        renderChips();
      });
      chip.appendChild(label);
      chip.appendChild(removeBtn);
      chipsRow.appendChild(chip);
    }
  };

  renderChips();
  editor.appendChild(chipsRow);

  // Add tag input
  const inputRow = document.createElement("div");
  inputRow.style.cssText = "display: flex; gap: 6px;";

  const input = document.createElement("input");
  input.type = "text";
  input.placeholder = "Add tag...";
  input.style.cssText = [
    "flex: 1",
    "padding: 5px 8px",
    "background: #0f1117",
    "border: 1px solid #2a2d3e",
    "border-radius: 4px",
    "color: #e2e8f0",
    "font-size: 0.78rem",
    "outline: none",
  ].join(";");

  const addBtn = document.createElement("button");
  addBtn.textContent = "Add";
  addBtn.style.cssText = [
    "padding: 5px 10px",
    "background: #6366f1",
    "border: none",
    "border-radius: 4px",
    "color: white",
    "cursor: pointer",
    "font-size: 0.78rem",
  ].join(";");

  const doAdd = (): void => {
    const val = input.value.trim();
    if (val && !tags.includes(val)) {
      tags.push(val);
      renderChips();
    }
    input.value = "";
  };

  addBtn.addEventListener("click", doAdd);
  input.addEventListener("keydown", (e) => { if (e.key === "Enter") doAdd(); });

  inputRow.appendChild(input);
  inputRow.appendChild(addBtn);
  editor.appendChild(inputRow);

  // Save button
  const saveBtn = document.createElement("button");
  saveBtn.textContent = "Save";
  saveBtn.style.cssText = [
    "width: 100%",
    "margin-top: 10px",
    "padding: 7px",
    "background: #6366f1",
    "border: none",
    "border-radius: 5px",
    "color: white",
    "cursor: pointer",
    "font-size: 0.82rem",
    "font-weight: 500",
  ].join(";");
  saveBtn.addEventListener("click", () => {
    onSave([...tags]);
    hideAnnotationEditor();
  });
  editor.appendChild(saveBtn);

  // Position near the annotate button
  const annotateEl = document.getElementById("annotate-btn");
  if (annotateEl) {
    const rect = annotateEl.getBoundingClientRect();
    editor.style.bottom = `${window.innerHeight - rect.top + 8}px`;
    editor.style.left = `${rect.left}px`;
  }

  // Suppress unused nodeId lint warning
  void nodeId;

  document.body.appendChild(editor);
  input.focus();
}

export function hideAnnotationEditor(): void {
  document.getElementById("annotation-editor")?.remove();
}
