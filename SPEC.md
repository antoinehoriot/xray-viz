# Xray — Technical Specification

**Version:** 0.2.0-draft
**Date:** 2026-02-28
**Status:** Draft (revised with architecture research)

---

## Table of Contents

1. [Project Overview and Goals](#1-project-overview-and-goals)
2. [High-Level Architecture](#2-high-level-architecture)
3. [Milestone Breakdown](#3-milestone-breakdown)
4. [Data Models](#4-data-models)
5. [CLI Interface Design](#5-cli-interface-design)
6. [WASM Renderer Architecture and Browser UI](#6-wasm-renderer-architecture-and-browser-ui)
7. [AI Context Export Format](#7-ai-context-export-format)
8. [Performance Requirements and Benchmarks](#8-performance-requirements-and-benchmarks)
9. [Testing Strategy](#9-testing-strategy)
10. [Error Handling](#10-error-handling)

---

## 1. Project Overview and Goals

### Problem Statement

Developers spend 80%+ of their time reading and understanding code, with onboarding taking over a month for 70%+ of engineers. No lightweight, offline tool exists to produce an interactive, accurate architectural map of how a codebase fits together across multiple languages.

Existing tools fail in critical ways:
- **Sourcetrail**: Archived Dec 2021, no Go/Rust/TS support.
- **CodeSee**: Cloud-only, requires uploading proprietary code.
- **IDE plugins**: Language-specific, editor-coupled, produce stale static views.
- **Documentation sites**: Reflect what someone wrote, not what the code does.

### Product Goals

Xray is a Rust CLI that runs `xray .` in any repository and opens a browser with an interactive, real-time architecture map.

**Primary goals:**
1. Parse any major language codebase in seconds via tree-sitter.
2. Visualize file-level and function-level dependency graphs in a browser without a server.
3. Enable blast radius analysis (select any file/function, highlight all downstream callers and dependencies).
4. Export structured graph slices for LLM/agent context.
5. Never send code to an external server.

**Non-goals (MVP):**
- Semantic code analysis beyond structural/dependency relationships.
- IDE plugin integrations.
- Cloud sync or team sharing (post-MVP).

---

## 2. High-Level Architecture

### Component Diagram

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                              xray CLI (Rust binary)                          │
│                                                                              │
│   ┌──────────────┐   ┌──────────────────┐   ┌────────────────────────────┐  │
│   │  CLI Parser  │──▶│  Scanner Engine  │──▶│   Graph Builder            │  │
│   │  (clap)      │   │  (ignore + TS)   │   │   (petgraph)               │  │
│   └──────────────┘   └──────────────────┘   └────────────┬───────────────┘  │
│                                                           │                  │
│                              ┌────────────────────────────▼──────────────┐  │
│                              │           Serializer / Exporter            │  │
│                              │   (JSON graph, AI context Markdown/JSON)   │  │
│                              └──────────┬─────────────────────────────────┘  │
└─────────────────────────────────────────┼────────────────────────────────────┘
                                          │
                     ┌────────────────────▼────────────────────┐
                     │         Embedded HTTP Server             │
                     │         (serves WASM + HTML bundle)      │
                     └────────────────────┬────────────────────┘
                                          │ WebSocket / HTTP JSON API
                     ┌────────────────────▼────────────────────┐
                     │         Browser (WASM + Sigma.js v3)     │
                     │                                          │
                     │  ┌──────────────────────────────────────┐ │
                     │  │ UI Shell (Vanilla JS/TS + Sigma.js)  │ │
                     │  │ XrayEngine WASM: compute only        │ │
                     │  │   layout, blast radius, search       │ │
                     │  │ Sigma.js v3: WebGL rendering          │ │
                     │  └──────────────────────────────────────┘ │
                     └─────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Language | Responsibility |
|---|---|---|
| CLI Parser | Rust (clap) | Command routing, flag parsing, user I/O |
| Scanner Engine | Rust (ignore, tree-sitter) | Walk directories (built-in .gitignore support, parallel), parse source files into ASTs |
| Graph Builder | Rust (petgraph) | Resolve dependencies, build directed graph |
| Serializer | Rust (serde_json) | Emit graph as JSON, AI export formats |
| Embedded HTTP Server | Rust (axum, tower-http) | Serve WASM bundle, expose graph API |
| Graph Renderer (WASM) | Rust → WASM (wasm-bindgen) | Force-directed layout, hit testing, rendering |
| UI Shell | TypeScript (vanilla) | Controls, search, blast radius UI, AI export panel |

### Data Flow

```
Source files
     │
     ▼
tree-sitter parse → AST per file
     │
     ▼
Dependency resolver → edges (imports, calls)
     │
     ▼
petgraph StableGraph<NodeData, EdgeData, Directed>
     │
     ├──▶ JSON serialization → embedded HTTP /api/graph
     ├──▶ AI export (JSON/Markdown)
     └──▶ WASM renderer ← browser fetches
```

---

## 3. Milestone Breakdown

### Milestone 0: Project Scaffolding (Day 1–2)

**Scope:**
- Cargo workspace layout (3 crates): `crates/xray-core`, `crates/xray-cli`, `crates/xray-wasm`
  - `xray-core`: rlib — parsing, graph (StableGraph), resolution, cache (shared by CLI + WASM)
  - `xray-cli`: binary crate (native target) — CLI entry point, server, depends on xray-core
  - `xray-wasm`: cdylib (wasm32 target) — thin wasm-bindgen wrapper around xray-core
- `web/` directory: TypeScript + HTML + CSS browser UI shell + Sigma.js integration
- `grammars/` directory: tree-sitter `.scm` query files per language
- CI pipeline: lint, test, WASM build (wasm-pack)
- Dev tooling: `just` recipes, `cargo-watch`

**Acceptance criteria:**
- `cargo build` succeeds across all crates
- `cargo test` runs (empty suite passes)
- WASM target (`wasm32-unknown-unknown`) compiles via `wasm-pack build`

**Dependencies:** None

---

### Milestone 1: MVP — File-Level Graph (Week 1)

**Scope:**
- Tree-sitter integration for: JavaScript/TypeScript, Python, Rust (tree-sitter 0.26+)
- File walker: `ignore` crate (built-in .gitignore support, parallel directory walking)
- Dependency extraction: import/require/use/mod resolution at file level
- Graph structure: `petgraph::StableGraph<NodeData, EdgeData, Directed>` — stable indices across incremental updates
- String interning via `lasso` for file paths (30-50% memory savings, thread-safe)
- Incremental cache: `blake3` hashing → `.xray/cache.db` (SQLite, file_hash→parsed_deps), `.xray/graph.bin` (bincode full graph)
- CLI: `xray scan [path]` outputs JSON graph to stdout or file, with `indicatif` progress bars
- JSON graph format (see §4)

**Acceptance criteria:**
- `xray scan .` on the Xray repo itself produces a valid graph JSON
- JS/TS: correctly resolves `import`/`require` with relative and bare specifiers
- Python: resolves `import`/`from ... import` at file level
- Rust: resolves `mod`, `use`, external crate edges
- 10K-file repo scans in under 10 seconds (fresh); under 1 second on cached re-scan
- Unit tests for each language resolver

**Dependencies:** M0

---

### Milestone 2: MVP — WASM Engine + Browser UI (Week 2)

**Scope:**
- `xray-wasm` crate: XrayEngine (compute only — no rendering):
  - Hierarchical Sugiyama layout (default) — shows DAG direction, aligns with directory structure
  - Force-directed layout (toggle) — Fruchterman-Reingold + Barnes-Hut for exploration
  - Blast radius computation (transitive closure), search, LOD filtering
  - Float32Array position output for direct GPU upload (no JSON overhead)
- `web/` browser UI: Sigma.js v3 for WebGL rendering (~50KB gzip, 50K nodes @ 60fps)
- LOD (Level of Detail) strategy:
  - Zoom out: directory clusters (~100–500 visible nodes)
  - Zoom medium: files within clusters (~1K–5K visible)
  - Zoom in: functions within files (~100–1K visible)
  - Progressive: instant cluster layout, stream file data on demand
- Blast radius overlay: select node → highlight all transitive dependencies and dependents
- Embedded HTTP server: `xray .` starts server (rust-embed bundle), auto-opens browser (`webbrowser` crate)
- UI shell: search bar, node detail panel, legend, mode toggle (Sugiyama/Force)

**Acceptance criteria:**
- `xray .` opens browser tab with interactive graph within 3 seconds on a 5K-node graph
- Sigma.js renders 50K nodes at 60fps; 10K nodes at 60fps after Sugiyama layout
- Node selection correctly highlights blast radius (transitive closure)
- Works in Chrome, Firefox, Safari (latest stable)
- No external network requests from the browser UI
- Sugiyama layout on 10K nodes completes in <500ms in WASM

**Dependencies:** M1

---

### Milestone 3: Go + Java Grammar + Function-Level Graphs (Week 3)

**Scope:**
- Tree-sitter grammars: Go, Java (tree-sitter 0.26+)
- Function-level parsing: extract function/method definitions and call sites
- **Lazy function-level loading (critical):** Full function-level data at 100K files requires ~500–800MB. Data MUST be loaded on-demand per file when user expands a node in the viewport. Function data is extracted lazily from `.xray/cache.db` — never loaded into memory all at once.
- Graph extension: dual-mode (file-level / function-level) switchable in UI
- Performance tuning for 100K-file repos via rayon parallelism + incremental cache
- Pro feature gate: function-level graphs and AI export require Pro license

**Acceptance criteria:**
- Go and Java imports resolve correctly
- Function call graph renders for a mid-size repo (50K LOC) with lazy expansion
- 100K-file repo file-level scan in under 30 seconds; incremental re-scan under 3 seconds
- Function-level data streams lazily: expanding a node loads its call graph without stalling the UI
- Pro gate enforced: unlicensed users see "upgrade" prompt

**Dependencies:** M2

---

### Milestone 4: AI Context Export + Pro Billing (Post-MVP)

**Scope:**
- AI export: structured JSON and Markdown slice of selected subgraph
- CLI command: `xray export --format json|markdown --from <file|function> --depth <n>`
- License validation: offline JWT or license key check
- Pro billing integration (Stripe or Lemon Squeezy)
- Landing page and marketing site

**Acceptance criteria:**
- `xray export` produces valid JSON and Markdown slices
- Exported JSON is parseable by common LLM tools without truncation for typical module clusters
- License check works offline (no external call required)

**Dependencies:** M3

---

### Milestone 5: Team Features (Post-M4)

**Scope:**
- Living architecture docs: auto-update graph on git push via GitHub Actions integration
- Team annotations: comment nodes/edges, saved to `.xray/annotations.json`
- Onboarding dashboards: curated views for new hires
- Shared workspace URL (self-hosted, no Xray cloud)

**Acceptance criteria:**
- GitHub Action runs `xray scan` and commits updated graph artifact
- Annotations persist across `xray .` invocations
- TBD: requires user research

**Dependencies:** M4

---

## 4. Data Models

### 4.1 Graph Structure

The core graph is a directed graph: nodes are source units (files or functions), edges are dependency relationships.

```rust
/// Stable node identifier (blake3 hash of canonical path, hex, first 16 chars)
type NodeId = String;

/// String interner — all file paths and labels use lasso::Rodeo for 30-50% memory savings
/// Thread-safe variant (lasso::ThreadedRodeo) used during parallel parsing
type SymbolKey = lasso::Spur;

/// Core in-memory graph — petgraph::StableGraph maintains stable indices across incremental updates
/// (node removals during re-scan do not invalidate other indices)
type Graph = petgraph::stable_graph::StableGraph<Node, Edge, petgraph::Directed>;

/// Top-level graph container (serialized as JSON for API, bincode for cache)
struct XrayGraph {
    version: u8,           // schema version, currently 1
    root: String,          // absolute path of scanned root
    scanned_at: String,    // ISO 8601 UTC timestamp
    languages: Vec<String>,
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    stats: GraphStats,
}

struct GraphStats {
    file_count: u64,
    function_count: u64,
    edge_count: u64,
    parse_duration_ms: u64,
    cache_hits: u64,       // files served from .xray/cache.db
}
```

**Incremental Cache Schema (.xray/cache.db — SQLite):**
```sql
CREATE TABLE file_cache (
    file_hash TEXT PRIMARY KEY,  -- blake3 hex of file contents
    path TEXT NOT NULL,
    language TEXT NOT NULL,
    deps_json BLOB NOT NULL,     -- serialized Vec<ImportDecl>
    parsed_at INTEGER NOT NULL   -- unix timestamp
);
```

**Graph Cache (.xray/graph.bin — bincode):**
Serialized `XrayGraph` for instant re-serve without re-parsing unchanged files.

### 4.2 Node Types

```rust
enum NodeKind {
    File,
    Module,     // logical grouping (e.g., Rust module, Python package)
    Function,
    Class,
    Interface,
}

struct Node {
    id: NodeId,
    kind: NodeKind,
    label: String,          // short display name (filename or function name)
    path: String,           // relative path from repo root
    language: String,       // "typescript", "python", "rust", "go", "java", "c"
    line_start: Option<u32>,
    line_end: Option<u32>,
    loc: Option<u32>,       // lines of code (filled in function-level mode)
    metadata: NodeMetadata,
}

struct NodeMetadata {
    exports: Vec<String>,   // exported symbols (functions, types, constants)
    is_entry_point: bool,   // true if detected as CLI/server/test entry
    tags: Vec<String>,      // user or auto-assigned tags
}
```

### 4.3 Edge Types

```rust
enum EdgeKind {
    Import,     // static import / require / use
    DynamicImport,
    Call,       // function call (function-level graph only)
    Inheritance, // class extends / implements
    ReExport,   // re-exported symbol
}

struct Edge {
    id: String,          // "{source_id}→{target_id}:{kind}"
    source: NodeId,
    target: NodeId,
    kind: EdgeKind,
    symbol: Option<String>, // specific symbol imported/called (if resolvable)
    resolved: bool,          // false if target node could not be found in graph
}
```

### 4.4 AST Intermediate Representation

Tree-sitter produces a concrete syntax tree per file. Xray uses a thin IR to extract the information needed for graph construction without retaining the full CST:

```rust
struct FileAst {
    path: String,
    language: String,
    imports: Vec<ImportDecl>,
    exports: Vec<ExportDecl>,
    functions: Vec<FunctionDecl>,  // populated in function-level mode
    classes: Vec<ClassDecl>,
}

struct ImportDecl {
    specifier: String,          // raw import string e.g. "../utils/format"
    resolved_path: Option<String>, // absolute path after resolution
    kind: ImportKind,           // Static | Dynamic | Reexport
    symbols: Vec<String>,       // named imports, empty = default/star
    line: u32,
}

struct ExportDecl {
    symbol: String,
    kind: ExportKind,           // Named | Default | Star
    line: u32,
}

struct FunctionDecl {
    name: String,
    line_start: u32,
    line_end: u32,
    calls: Vec<CallSite>,
    is_exported: bool,
    is_async: bool,
}

struct CallSite {
    callee: String,             // raw callee name
    resolved_node_id: Option<NodeId>,
    line: u32,
}
```

### 4.5 Graph Serialization (JSON Schema)

```json
{
  "version": 1,
  "root": "/home/user/myrepo",
  "scanned_at": "2026-02-28T09:00:00Z",
  "languages": ["typescript", "rust"],
  "stats": {
    "file_count": 342,
    "function_count": 0,
    "edge_count": 891,
    "parse_duration_ms": 1240
  },
  "nodes": [
    {
      "id": "a1b2c3d4e5f6a7b8",
      "kind": "File",
      "label": "index.ts",
      "path": "src/index.ts",
      "language": "typescript",
      "line_start": null,
      "line_end": null,
      "loc": null,
      "metadata": {
        "exports": ["main", "App"],
        "is_entry_point": true,
        "tags": []
      }
    }
  ],
  "edges": [
    {
      "id": "a1b2c3d4e5f6a7b8→c8d7e6f5a4b3c2d1:Import",
      "source": "a1b2c3d4e5f6a7b8",
      "target": "c8d7e6f5a4b3c2d1",
      "kind": "Import",
      "symbol": null,
      "resolved": true
    }
  ]
}
```

---

## 5. CLI Interface Design

### Command Structure

```
xray [OPTIONS] <COMMAND>

Commands:
  scan      Parse a codebase and output a graph (no browser)
  view      Scan and open interactive browser UI (default command)
  export    Export a graph slice for AI/LLM context
  license   Manage Pro license
  help      Print help information

Options:
  -h, --help       Print help
  -V, --version    Print version
```

### `xray view` (default)

```
xray view [OPTIONS] [PATH]

Arguments:
  [PATH]  Root directory to scan [default: .]

Options:
  --port <PORT>           Port for embedded HTTP server [default: 7000]
  --no-open               Don't auto-open browser
  --level <LEVEL>         Graph detail level: file|function [default: file]
  --exclude <GLOB>...     Exclude patterns (e.g. "node_modules/**")
  --include <LANG>...     Languages to include [default: all detected]
  --depth <N>             Maximum directory depth [default: unlimited]
  --watch                 Re-scan on file changes (hot reload)
  -o, --output <FILE>     Also write graph JSON to file
  --format <FORMAT>       Output format: json|dot [default: json]
```

**Example:**
```bash
xray .                         # scan CWD, open browser on :7000
xray view ./myproject --port 8080 --level function
xray view --exclude "*.test.*" --exclude "fixtures/**"
```

### `xray scan`

```
xray scan [OPTIONS] [PATH]

Arguments:
  [PATH]  Root directory to scan [default: .]

Options:
  -o, --output <FILE>   Write graph JSON to file [default: stdout]
  --format <FORMAT>     Output format: json|dot|mermaid [default: json]
  --level <LEVEL>       Graph detail: file|function [default: file]
  --exclude <GLOB>...   Exclude patterns
  --include <LANG>...   Languages to include
  --depth <N>           Max directory depth
  --pretty              Pretty-print JSON output
  --stats               Print scan stats to stderr
```

**Example:**
```bash
xray scan . -o graph.json --pretty --stats
xray scan /path/to/repo --format dot | dot -Tsvg > arch.svg
```

### `xray export`

```
xray export [OPTIONS] [PATH]

Arguments:
  [PATH]  Root directory [default: .]

Options:
  --from <NODE>         Starting node (file path or function name)
  --depth <N>           Traversal depth from starting node [default: 2]
  --direction <DIR>     Direction: upstream|downstream|both [default: both]
  --format <FORMAT>     Output format: json|markdown [default: markdown]
  --max-nodes <N>       Cap exported nodes [default: 200]
  -o, --output <FILE>   Write to file [default: stdout]
```

**Example:**
```bash
xray export . --from src/auth/index.ts --depth 3 --format markdown
xray export . --from login_user --direction downstream --format json
```

### `xray license`

```
xray license <SUBCOMMAND>

Subcommands:
  activate   Activate a Pro license key
  status     Show current license status
  deactivate Remove stored license
```

### Output Formats

| Format | Description | Use case |
|---|---|---|
| `json` | Full XrayGraph JSON | Machine consumption, piping, storage |
| `dot` | Graphviz DOT format | SVG/PDF export via `dot` |
| `mermaid` | Mermaid diagram syntax | Markdown embedding |
| `markdown` | AI-readable narrative + graph | LLM context export |

### Exit Codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | General error (see stderr) |
| 2 | Invalid arguments |
| 3 | Parse failure on one or more files (partial success) |
| 4 | License required for requested feature |
| 5 | No files found matching given criteria |

---

## 6. WASM Renderer Architecture and Browser UI

### Bundle Composition

The Rust binary embeds the browser bundle via `rust-embed` at compile time. `rust-embed` supports hot reload in development (serves from filesystem when `debug_assertions` is set) while embedding assets at release.

```
web/
  index.html          # ~5KB, references wasm + main.js
  main.js             # UI shell, bundled (esbuild, ~80KB gzip, includes Sigma.js v3)
  xray_wasm_bg.wasm   # XrayEngine WASM (<2MB raw, <500KB gzip target)
  xray_wasm.js        # wasm-bindgen JS glue
```

**WASM build configuration (Cargo.toml for xray-wasm):**
```toml
[profile.release]
opt-level = "z"        # optimize for size
lto = true
codegen-units = 1
strip = true
panic = "abort"        # smaller panic handler
```
Post-build: `wasm-opt -Oz` via wasm-pack to reach <500KB gzip.

The embedded HTTP server (axum) serves these files from memory. The server listens on `127.0.0.1` only (not `0.0.0.0`) for security. Browser auto-opened via `webbrowser` crate.

### WASM Engine (XrayEngine — compute only)

**Crate:** `crates/xray-wasm` (compiled to `wasm32-unknown-unknown`, cdylib)

**Key architectural decision:** XrayEngine handles computation only — layout, search, blast radius, LOD filtering. Sigma.js v3 handles all rendering via WebGL. This separation yields:
- Smaller WASM binary (no web-sys Canvas rendering code)
- Better rendering performance (Sigma.js: 50K nodes @ 60fps with WebGL)
- Hot path: `get_node_positions()` returns `Float32Array` for direct GPU upload — no JSON overhead

**Responsibilities:**
- Receive graph JSON from JS via `wasm-bindgen`
- Run Sugiyama hierarchical layout (default) or Fruchterman-Reingold force-directed (toggle)
- Compute blast radius (transitive closure), reverse deps
- Search/filter node IDs
- LOD viewport filtering: return only visible nodes at current zoom level

**Public WASM API (wasm-bindgen):**

```rust
#[wasm_bindgen]
pub struct XrayEngine {
    // internal StableGraph + layout state
}

#[wasm_bindgen]
impl XrayEngine {
    pub fn new(graph_json: &str) -> XrayEngine;
    pub fn layout_hierarchical(&mut self);              // Sugiyama (DEFAULT) — ~100-500ms for 10K nodes
    pub fn layout_force_directed(&mut self) -> bool;   // tick, returns converged
    pub fn get_node_positions(&self) -> Vec<f32>;       // flat [x0,y0,x1,y1,...] — Float32Array for Sigma
    pub fn get_edges(&self) -> Vec<u32>;                // [src0,tgt0,src1,tgt1,...] for Sigma edge list
    pub fn blast_radius(&self, node_id: u32) -> Vec<u32>;   // downstream (dependencies)
    pub fn reverse_deps(&self, node_id: u32) -> Vec<u32>;   // upstream (dependents)
    pub fn search(&self, query: &str) -> Vec<u32>;          // matching node indices
    pub fn get_visible_nodes(&self, viewport: &[f32]) -> Vec<u32>; // LOD: [x, y, w, h, zoom]
}
```

**Layout algorithms:**
1. **Sugiyama hierarchical (default):** Shows dependency direction clearly. Aligns clusters with directory structure. 10K nodes in ~100–500ms in WASM. Preferred for understanding architecture.
2. **Fruchterman-Reingold force-directed (toggle):** Barnes-Hut quadtree O(N log N) for N > 500. Initial positions: spiral to avoid cold-start clustering. Convergence: Σ displacements < 0.1 × N. Good for exploration and circular dependency discovery.

**LOD strategy (via `get_visible_nodes`):**
- Zoom < 0.1: return directory cluster nodes only (~100–500 nodes)
- Zoom 0.1–0.5: return file nodes within visible clusters (~1K–5K nodes)
- Zoom > 0.5: return function nodes for expanded files in viewport (~100–1K nodes)

### Browser UI Shell

**Language:** TypeScript (no framework, vanilla DOM, bundled with esbuild)

**Rendering:** Sigma.js v3 (WebGL) — orchestrates between XrayEngine WASM (positions/edges) and WebGL display. Sigma receives `Float32Array` positions directly for GPU upload without JSON parsing overhead.

**UI Panels:**

```
┌────────────────────────────────────────────────────────────┐
│  [Xray]  Search: [________________]  [File|Fn]  [Export▾] │  ← top bar
├────────────────────────────────────────────────────────────┤
│                                                            │
│                     <canvas>                               │  ← main renderer
│                  (graph display)                           │
│                                                            │
├──────────────────────────────────┬─────────────────────────┤
│  Node Detail Panel               │  Graph Stats            │  ← bottom bar
│  Path: src/auth/index.ts         │  342 files  891 edges   │
│  Lang: TypeScript                │  8 clusters             │
│  Exports: [login, logout, ...]   │                         │
│  [Blast Radius ▶]                │                         │
└──────────────────────────────────┴─────────────────────────┘
```

**Interactions:**

| Interaction | Behavior |
|---|---|
| Click node | Select node, show detail panel |
| Double-click node | Zoom to node neighborhood |
| Click "Blast Radius" | Highlight transitive closure (dependents + dependencies) |
| Drag canvas | Pan |
| Scroll / pinch | Zoom |
| Search | Filter+highlight matching nodes, others dimmed |
| Export button | Opens export panel (Pro gate for AI context) |
| Mode toggle (File/Fn) | Switches graph between file-level and function-level |
| Layout toggle (Hierarchy/Force) | Switches between Sugiyama hierarchical (default) and force-directed |

**Visual encoding:**

| Element | Encoding |
|---|---|
| Node color | Language (blue=TS, orange=Python, green=Rust, teal=Go, red=Java) |
| Node size | LOC (log scale; min 8px, max 32px) |
| Node border | Entry points have dashed border |
| Edge color | Import=gray, Call=blue, Inheritance=purple, DynamicImport=dashed gray |
| Blast radius highlight | Selected=yellow, Dependents=red, Dependencies=green |
| Unrelated nodes | 30% opacity when something is selected |

---

## 7. AI Context Export Format

### JSON Format

```json
{
  "xray_export_version": 1,
  "exported_at": "2026-02-28T09:00:00Z",
  "root": "/path/to/repo",
  "query": {
    "from": "src/auth/index.ts",
    "depth": 2,
    "direction": "both"
  },
  "summary": {
    "node_count": 14,
    "edge_count": 23,
    "languages": ["typescript"],
    "entry_node": {
      "id": "a1b2c3d4e5f6a7b8",
      "path": "src/auth/index.ts",
      "exports": ["login", "logout", "refreshToken"]
    }
  },
  "nodes": [ /* same schema as §4.2, subset */ ],
  "edges": [ /* same schema as §4.3, subset */ ],
  "dependency_order": [ /* node ids in topological order */ ]
}
```

### Markdown Format

```markdown
# Xray Context Export

**Repository:** /path/to/repo
**Exported:** 2026-02-28T09:00:00Z
**Starting node:** src/auth/index.ts (depth 2, both directions)

## Architecture Summary

14 files, 23 dependencies across 1 language (TypeScript).

## Entry Node: src/auth/index.ts

- **Exports:** login, logout, refreshToken
- **Direct dependencies:** src/db/client.ts, src/utils/jwt.ts, src/config/index.ts
- **Direct dependents:** src/routes/auth.ts, src/middleware/guard.ts

## Dependency Tree

```
src/auth/index.ts
├── src/db/client.ts
│   └── src/db/connection.ts
├── src/utils/jwt.ts
└── src/config/index.ts
```

## File Index

| File | Language | Exports |
|---|---|---|
| src/auth/index.ts | TypeScript | login, logout, refreshToken |
| src/db/client.ts | TypeScript | query, transaction |
| ... | | |

## Dependency Matrix

| From | To | Kind |
|---|---|---|
| src/auth/index.ts | src/db/client.ts | Import |
| ... | | |
```

### Usage with LLMs

The export is designed to fit within a single LLM context window for typical module clusters (≤200 nodes with `--max-nodes 200`). Recommended usage:

```bash
# Generate context for a single module and pipe to an LLM
xray export . --from src/auth --depth 2 --format markdown | \
  llm "Explain the authentication flow and identify potential security concerns"
```

---

## 8. Performance Requirements and Benchmarks

### Targets

| Metric | Target | Stretch |
|---|---|---|
| 10K file scan (file-level) | < 5s | < 2s |
| 50K file scan (file-level) | < 15s | < 8s |
| 100K file scan (file-level) | < 30s | < 15s |
| Graph JSON serialization (100K nodes) | < 2s | < 0.5s |
| Browser: initial render (1K nodes) | < 1s after WASM load | < 500ms |
| Browser: interactive FPS (1K nodes) | 60fps | 60fps |
| Browser: interactive FPS (10K nodes) | 30fps | 60fps |
| WASM bundle size (raw) | < 2MB | < 1.5MB |
| WASM bundle size (gzip) | < 500KB | < 300KB |
| Cold start (binary launch → browser open) | < 3s | < 1.5s |

### Implementation Strategies

**Parsing parallelism:** Use `rayon` thread pool for parallel file parsing. Each file is parsed independently; results are merged into the graph.

**Incremental scanning:** `blake3` hashes file contents (~1GB/s with SIMD). Hash → parsed dependency mapping stored in `.xray/cache.db` (SQLite). Full serialized graph stored in `.xray/graph.bin` (bincode) for instant re-serve. Only files with changed hashes are re-parsed. Re-scan of unchanged 10K-file repo: <1 second.

**WASM compute optimization:**
- Sugiyama hierarchical layout as default: deterministic, fast (~100–500ms for 10K nodes in WASM).
- Barnes-Hut quadtree for force simulation O(N log N) instead of O(N²).
- `get_node_positions()` returns `Float32Array` — no JSON serialization, direct GPU upload via Sigma.js.
- LOD via `get_visible_nodes(viewport)`: renders only nodes visible at current zoom level.

**Sigma.js v3 rendering:**
- WebGL-accelerated: 50K nodes at 60fps with Sigma.js v3 (~50KB gzip).
- No OffscreenCanvas needed — Sigma handles GPU efficiently on main thread.

**Memory budget:**
- In-memory graph (file-level): ~500 bytes/node + ~100 bytes/edge. 100K nodes ≈ 50MB — acceptable.
- Function-level data: **NOT loaded into memory**. Lazy-loaded per file from `.xray/cache.db` on user expand.
- WASM heap: configure `--initial-memory 32MB --max-memory 128MB` (smaller than before; no rendering state).

### Benchmarking Harness

```bash
# Built-in benchmark mode (outputs stats to stderr)
xray scan /large/repo --stats 2>&1 | grep "parse_duration_ms"

# Dedicated benchmark crate (crates/bench, using criterion)
cargo bench --package xray-bench
```

Benchmark scenarios (checked into `crates/bench/fixtures/`):
- `small`: 500 files, mixed JS/TS
- `medium`: 10K files, mixed TS/Python
- `large`: 50K files, monorepo (Rust + TS + Python)
- `xlarge`: 100K files (generated synthetic repo)

---

## 9. Testing Strategy

### Test Pyramid

```
           ┌─────────────────┐
           │  E2E / Browser  │  (5–10 scenarios)
           │   Playwright    │
           ├─────────────────┤
           │  Integration    │  (50–100 tests)
           │  CLI + server   │
           ├─────────────────┤
           │    Unit Tests   │  (200+ tests)
           │  parsers/graph  │
           └─────────────────┘
```

### Unit Tests (`cargo test`)

**Coverage areas:**
1. **Language resolvers** — Each resolver has a fixture directory of source snippets covering:
   - Named imports, default imports, star imports
   - Dynamic imports
   - Re-exports
   - Circular dependency detection
   - Missing/unresolvable imports (should not panic)

2. **Graph builder** — Test with hand-crafted `FileAst` inputs:
   - Correct edge creation from import declarations
   - Node deduplication (same file referenced from multiple sources)
   - Topological sort for dependency ordering
   - Cycle detection (graphs are not required to be acyclic)

3. **Serializer** — Round-trip: construct graph → serialize to JSON → deserialize → compare.

4. **WASM renderer logic** — Pure Rust unit tests (no wasm target needed):
   - Hit testing (given node positions, does hit_test return correct ID?)
   - Blast radius computation (transitive closure correctness)
   - Search filtering

### Integration Tests

Located in `tests/integration/`. Each test:
1. Creates a temporary directory with source files.
2. Runs `xray scan` as a subprocess.
3. Asserts on stdout JSON (node count, edge count, specific edges).

**Scenarios:**
- Minimal JS project (3 files, 2 imports)
- Circular dependency in Python
- Rust workspace with multiple crates
- Repo with mixed languages
- Repo with `.gitignore` exclusions
- Empty directory (graceful exit)
- Single file with no imports

### E2E Tests (Playwright)

Located in `tests/e2e/`. Run against a local `xray view` instance.

**Scenarios:**
1. Load graph, verify nodes render.
2. Click node → detail panel shows correct info.
3. Click "Blast Radius" → highlights correct nodes.
4. Search → matching nodes highlighted, others dimmed.
5. Zoom/pan → graph stays within canvas.
6. Mode switch (file ↔ function) → graph updates.

### Test Fixtures

```
tests/fixtures/
  js-basic/          # 5 files, simple import chain
  py-circular/       # circular import between 2 Python files
  rust-workspace/    # cargo workspace with 3 crates
  mixed-large/       # 500 files, 3 languages (generated)
```

### CI Matrix

```yaml
# .github/workflows/ci.yml
strategy:
  matrix:
    os: [ubuntu-latest, macos-latest, windows-latest]
    rust: [stable, beta]
```

---

## 10. Error Handling

### Principles

1. **Never panic in library code.** All public functions return `Result<T, XrayError>`.
2. **Partial success is valid.** If 5 out of 1000 files fail to parse, report the failures and continue. Exit code 3.
3. **User-facing errors are actionable.** Every error message tells the user what went wrong and (where applicable) how to fix it.
4. **Structured errors for machine consumption.** JSON output includes an `errors` array.

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum XrayError {
    #[error("Failed to walk directory '{path}': {source}")]
    WalkDir { path: PathBuf, source: walkdir::Error },

    #[error("Failed to read file '{path}': {source}")]
    FileRead { path: PathBuf, source: std::io::Error },

    #[error("Parse error in '{path}' at line {line}: {message}")]
    ParseError { path: PathBuf, line: u32, message: String },

    #[error("Language '{language}' is not supported")]
    UnsupportedLanguage { language: String },

    #[error("Graph serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Server failed to bind on port {port}: {source}")]
    ServerBind { port: u16, source: std::io::Error },

    #[error("License required for '{feature}'. Run `xray license activate <key>`.")]
    LicenseRequired { feature: String },

    #[error("Invalid license key: {reason}")]
    InvalidLicense { reason: String },
}
```

### Error Reporting

**CLI output (stderr):**
```
error: Failed to parse 'src/legacy.ts' at line 42: unexpected token 'do'
warning: Skipping 'vendor/lodash.js' (unsupported extension)
...
Scan complete: 341 files parsed, 1 error, 2 warnings.
```

**JSON output with errors:**
```json
{
  "version": 1,
  "nodes": [ ... ],
  "edges": [ ... ],
  "errors": [
    {
      "kind": "ParseError",
      "path": "src/legacy.ts",
      "line": 42,
      "message": "unexpected token 'do'"
    }
  ]
}
```

### Graceful Degradation

| Situation | Behavior |
|---|---|
| File fails to parse | Skip file, log warning, include in `errors` array, continue |
| Unresolvable import | Create edge with `resolved: false`, continue |
| Port already in use | Try next port (up to +10), warn user |
| WASM fails to load | Show text-only fallback listing all nodes/edges |
| Browser cannot open | Print URL to stdout, proceed |
| License server unreachable | Fall back to cached license (offline validation) |

### Logging

```rust
// Use tracing crate with XRAY_LOG env var
// Default level: warn
// XRAY_LOG=debug xray . — verbose output for troubleshooting
```

Log levels:
- `error`: unrecoverable failures that change exit code
- `warn`: skipped files, unresolvable imports, port conflicts
- `info`: scan progress (every 10K files), server startup
- `debug`: per-file parse timing, edge resolution details
- `trace`: AST node-level detail (very verbose)

---

## Appendix A: Technology Stack

| Component | Crate/Library | Version | Notes |
|---|---|---|---|
| CLI framework | clap | 4.x | |
| Terminal progress | indicatif | 0.17.x | Progress bars during scan/parse |
| Directory walker | ignore | 0.4.x | Replaces walkdir; built-in .gitignore, parallel walking |
| Tree-sitter binding | tree-sitter | 0.26.x | Updated from 0.22 |
| Graph data structure | petgraph (StableGraph) | 0.6.x | StableGraph for stable indices across incremental updates |
| String interning | lasso | 0.7.x | Thread-safe (ThreadedRodeo); 30-50% memory savings on paths |
| Content hashing | blake3 | 1.x | ~1GB/s with SIMD; for incremental cache keys |
| Incremental cache | rusqlite | 0.31.x | .xray/cache.db — file_hash → parsed deps |
| Binary graph cache | bincode | 2.x | .xray/graph.bin — full graph for instant re-serve |
| JSON serialization | serde + serde_json | 1.x | |
| HTTP server | axum | 0.7.x | |
| Static asset embed | rust-embed | 8.x | Replaces include_bytes!; dev hot reload |
| Browser auto-open | webbrowser | 1.x | Cross-platform browser launch |
| WASM bindings | wasm-bindgen | 0.2.x | |
| Browser graph render | Sigma.js | v3 | ~50KB gzip; WebGL; 50K nodes @ 60fps |
| Error handling | thiserror | 1.x | |
| Parallel processing | rayon | 1.x | |
| File watching | notify | 6.x | |
| E2E testing | Playwright (Node) | 1.x | |
| Benchmarking | criterion | 0.5.x | |
| WASM size optimization | wasm-opt (-Oz) | (via wasm-pack) | Post-build optimization |

---

## Appendix B: File Structure

```
codebase-viz/
├── Cargo.toml               # workspace (3 crates: xray-core, xray-cli, xray-wasm)
├── crates/
│   ├── xray-core/           # rlib — shared by CLI + WASM targets
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── scanner/
│   │       │   ├── mod.rs
│   │       │   ├── walker.rs        # ignore crate integration
│   │       │   └── languages/
│   │       │       ├── typescript.rs
│   │       │       ├── python.rs
│   │       │       ├── rust.rs
│   │       │       ├── go.rs
│   │       │       └── java.rs
│   │       ├── graph/
│   │       │   ├── mod.rs
│   │       │   ├── builder.rs       # StableGraph construction
│   │       │   ├── types.rs         # Node, Edge, XrayGraph
│   │       │   ├── export.rs        # JSON + bincode + AI export
│   │       │   └── blast_radius.rs  # transitive closure
│   │       ├── cache/
│   │       │   ├── mod.rs
│   │       │   ├── db.rs            # SQLite cache.db (blake3 → deps)
│   │       │   └── graph_bin.rs     # bincode graph.bin serialization
│   │       └── layout/
│   │           ├── mod.rs
│   │           ├── sugiyama.rs      # hierarchical (default)
│   │           └── force.rs         # Fruchterman-Reingold + Barnes-Hut
│   ├── xray-cli/            # binary crate (native target)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── commands/
│   │       │   ├── view.rs
│   │       │   ├── scan.rs
│   │       │   ├── export.rs
│   │       │   └── license.rs
│   │       └── server/
│   │           ├── mod.rs
│   │           └── routes.rs        # axum routes, rust-embed assets
│   └── xray-wasm/           # cdylib (wasm32-unknown-unknown target)
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs               # XrayEngine wasm-bindgen wrapper
├── grammars/                # tree-sitter .scm query files
│   ├── typescript.scm
│   ├── python.scm
│   ├── rust.scm
│   ├── go.scm
│   └── java.scm
├── web/                     # browser UI shell (TypeScript + Sigma.js v3)
│   ├── src/
│   │   ├── main.ts          # entry point, Sigma.js init
│   │   ├── engine.ts        # XrayEngine WASM wrapper
│   │   ├── panels.ts        # detail panel, blast radius UI
│   │   └── styles.css
│   ├── index.html
│   └── package.json         # esbuild build, sigma dependency
├── tests/
│   ├── fixtures/
│   │   ├── js-basic/        # 5 files, simple import chain
│   │   ├── py-circular/     # circular import
│   │   ├── rust-workspace/  # cargo workspace, 3 crates
│   │   └── mixed-large/     # 500 files, 3 languages (generated)
│   ├── integration/
│   └── e2e/                 # Playwright tests
├── benches/                 # criterion benchmarks (workspace-level)
│   └── scan_bench.rs
└── SPEC.md                  # this file
```

---

*End of Specification*
