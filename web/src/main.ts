/**
 * Xray UI Shell — entry point.
 * M2 implementation: initialize Sigma.js, fetch graph from /api/graph,
 * load XrayEngine WASM, run layout, render.
 */

import type { XrayGraph } from "./engine";

// Graph and engine state (initialized in init())
let graphData: XrayGraph | null = null;

async function init(): Promise<void> {
  const loading = document.getElementById("loading") as HTMLElement;

  try {
    // M2: fetch graph from embedded server
    // const resp = await fetch("/api/graph");
    // graphData = await resp.json();

    // Placeholder: show empty state
    loading.textContent = "No graph loaded. Run `xray .` to scan your codebase.";

    setupSearch();
    setupModeToggle();
    setupLayoutToggle();
  } catch (err) {
    loading.textContent = `Error: ${String(err)}`;
  }
}

function setupSearch(): void {
  const input = document.getElementById("search") as HTMLInputElement;
  input.addEventListener("input", () => {
    const _query = input.value.trim();
    // M2: engine.search(query) -> highlight matching nodes in Sigma
  });
}

function setupModeToggle(): void {
  document.querySelectorAll(".mode-btn").forEach((btn) => {
    btn.addEventListener("click", (e) => {
      document.querySelectorAll(".mode-btn").forEach((b) => b.classList.remove("active"));
      (e.target as HTMLElement).classList.add("active");
      const _mode = (e.target as HTMLElement).dataset["mode"];
      // M2: switch graph between file-level and function-level
    });
  });
}

function setupLayoutToggle(): void {
  document.querySelectorAll(".layout-btn").forEach((btn) => {
    btn.addEventListener("click", (e) => {
      document.querySelectorAll(".layout-btn").forEach((b) => b.classList.remove("active"));
      (e.target as HTMLElement).classList.add("active");
      const _layout = (e.target as HTMLElement).dataset["layout"];
      // M2: switch between hierarchical and force-directed layout
    });
  });
}

document.addEventListener("DOMContentLoaded", () => { void init(); });
