# Xray — Product Roadmap

> **Interactive codebase architecture visualizer**
> Last updated: 2026-02-28

---

## Table of Contents

1. [Current Feature Inventory (M0-M4)](#current-feature-inventory)
2. [New Feature Ideas](#new-feature-ideas)
3. [Remaining TODO List](#remaining-todo-list)

---

## Current Feature Inventory

### M0 — Scaffold & Core Architecture

| Feature | Status | Details |
|---|---|---|
| **Cargo workspace** | Done | 3-crate workspace: xray-core, xray-cli, xray-wasm |
| **Tree-sitter parsing** | Done | Language-specific parsers with .scm grammar queries |
| **Graph data model** | Done | XrayGraph with Node, Edge, NodeKind, EdgeKind types |
| **petgraph-backed builder** | Done | GraphBuilder using StableGraph for dedup |
| **blake3 NodeId** | Done | Stable 16-char hex identifiers from path hashes |
| **File walker** | Done | ignore crate respects .gitignore, supports --exclude/--include/--depth |

### M1 — Parsing & Caching

| Feature | Status | Details |
|---|---|---|
| **5-language support** | Done | TypeScript/JS, Python, Rust, Go, Java |
| **Import detection** | Done | Static, Dynamic, Re-export edge kinds |
| **Export detection** | Done | Named, Default, Star exports |
| **Function/Class extraction** | Done | FunctionDecl and ClassDecl with line ranges, calls, async/export flags |
| **SQLite cache** | Done | CacheDb stores parsed imports and functions keyed by blake3 content hash |
| **Binary graph cache** | Done | graph_bin serializes full XrayGraph via bincode for instant re-serve |
| **Import resolution** | Done | resolve_imports() maps specifiers to known file paths |

### M2 — WASM Engine & Layout

| Feature | Status | Details |
|---|---|---|
| **WASM bindings** | Done | XrayEngine struct with wasm-bindgen, accepts graph JSON |
| **Sugiyama layout** | Done | Kahn longest-path layer assignment + barycenter crossing minimization |
| **Force-directed layout** | Done | Fruchterman-Reingold with per-tick API and ForceState |
| **LOD viewport filtering** | Done | get_visible_nodes() with zoom-level thresholds (500-node cap at low zoom) |
| **Blast radius (WASM)** | Done | Transitive downstream/upstream dependency computation |
| **Search (WASM)** | Done | Case-insensitive label/path matching |
| **GPU-ready positions** | Done | Float32Array-compatible Vec<f32> output for Sigma.js |

### M3 — Web UI & Interactive Visualization

| Feature | Status | Details |
|---|---|---|
| **Sigma.js v3 renderer** | Done | graphology directed graph + WebGL rendering |
| **Dark theme** | Done | Full dark palette with language-colored nodes |
| **Language-specific colors** | Done | TS=blue, Python=orange, Rust=green, Go=cyan, Java=red |
| **Node size by LOC** | Done | Logarithmic scaling (8-32px) |
| **Search bar** | Done | Real-time node filtering with dim/highlight |
| **Node detail panel** | Done | Shows path, language, exports on click |
| **Blast radius visualization** | Done | Highlight downstream (green) + upstream (red) + selected (yellow) |
| **Edge dimming** | Done | Non-relevant edges hidden during selection |
| **Layout toggle** | Done | Switch between Force Atlas 2 and hierarchical |
| **File/Function mode toggle** | Done | Switch between file-level and function-level views |
| **Function node expansion** | Done | Click file in Fn mode to lazy-load functions from /api/functions |
| **Function detail panel** | Done | Shows function name, kind, signature |
| **Call edges** | Done | Visual edges between function nodes |
| **Custom hover renderer** | Done | Dark-themed hover tooltip with shadow |
| **Fallback UI** | Done | Minimal inline HTML when web/dist not built |

### M4 — CLI, Export & Licensing

| Feature | Status | Details |
|---|---|---|
| **xray view** | Done | Scan + serve + auto-open browser (default command) |
| **xray scan** | Done | Parse-only, output JSON or DOT, with --pretty/--stats flags |
| **xray export** | Done | AI context export (Pro-gated) with subgraph extraction |
| **AI JSON export** | Done | Structured AiExportJson with dependency order, entry node, summary |
| **AI Markdown export** | Done | Human-readable with dependency tree, file index, dependency matrix |
| **Subgraph extraction** | Done | BFS from starting node with depth limit, direction, max-nodes cap |
| **Topological sort** | Done | Kahn algorithm for dependency ordering in exports |
| **xray license activate** | Done | Offline XRAY-XXXX-XXXX key validation, stored at ~/.xray/license.key |
| **xray license status** | Done | Show active/inactive with key redaction |
| **xray license deactivate** | Done | Remove stored key |
| **Pro gate (CLI)** | Done | is_pro() check before export command |
| **Pro gate (Web)** | Done | Modal prompt + localStorage key storage + 402 API response |
| **DEV_MODE bypass** | Done | const DEV_MODE = true flag to skip Pro gate in development |
| **Graphviz DOT output** | Done | to_dot() for external rendering |
| **Progress spinner** | Done | indicatif spinner during scan |
| **axum HTTP server** | Done | Routes: /health, /api/graph, /api/functions, static files |
| **CORS support** | Done | tower-http CORS middleware |
| **Auto port discovery** | Done | Binds to next available port if default (7000) is taken |
| **Embedded static files** | Done | rust-embed for bundled web assets |

---

## New Feature Ideas

### Free Tier Features

| Feature | Priority | Inspiration | Description |
|---|---|---|---|
| **Watch mode (hot reload)** | P1 | CodeSee | --watch flag already accepted but not implemented. Re-scan on file changes via notify crate, push updated graph to browser via WebSocket/SSE |
| **Circular dependency detection** | P1 | Dep Cruiser, Madge | Detect and highlight import cycles. Display cycle paths in detail panel. Add xray scan --cycles CLI flag |
| **Orphan file detection** | P2 | Dep Cruiser | Identify files with zero importers (dead code candidates). Highlight in UI and list via CLI |
| **Dependency rules/validation** | P2 | Dep Cruiser | Define allowed/forbidden dependency patterns (e.g. ui/ must not import db/). Run as CI check: xray lint --rules .xray/rules.yml |
| **Stability metrics** | P2 | Dep Cruiser | Compute instability index (fan-out / (fan-in + fan-out)) per module/directory. Surface in detail panel and as JSON output |
| **Directory grouping** | P2 | CodeSee | Group nodes by directory. Collapsible folder clusters. Show inter-module dependency counts |
| **Multi-repo/monorepo support** | P3 | CodeSee | Support scanning multiple roots or workspace packages. Show cross-package deps |
| **C/C++ language support** | P3 | — | Add #include parsing via tree-sitter-c and tree-sitter-cpp |
| **PHP/Ruby/Swift support** | P4 | — | Extend language coverage for broader adoption |

### Pro Tier Features

| Feature | Priority | Inspiration | Description |
|---|---|---|---|
| **AI codebase Q&A** | P1 | CodeSee, Greptile | Ask your codebase — use the graph + file contents as context for an LLM. Query: How does auth flow work? with graph-aware retrieval |
| **PR impact analysis** | P1 | CodeSee | Given a diff/PR, compute affected nodes and blast radius. GitHub Action/CI integration |
| **Architecture change detection** | P2 | CodeSee | Compare graph snapshots over time. Alert on new circular deps, coupling increases, architecture drift |
| **Custom graph annotations** | P2 | — | Tag nodes with custom labels (team ownership, domain, status). Persist in .xray/annotations.yml |
| **IDE extension (VS Code)** | P2 | Repomix, Cursor | Show dependency minimap in sidebar. Click-to-navigate from graph to source |
| **Team/CODEOWNERS overlay** | P3 | CodeSee | Color nodes by team ownership based on CODEOWNERS file |
| **Diff-aware export** | P3 | Repomix | Export only the subgraph affected by a git diff for focused AI review |
| **Shareable graph links** | P3 | CodeSee | Generate static HTML snapshot of current graph view for sharing |
| **Historical graph timeline** | P4 | CodeScene | Store graph snapshots at each commit. Visualize architecture evolution |

### AI & LLM Integration Features

| Feature | Priority | Inspiration | Description |
|---|---|---|---|
| **MCP server** | P1 | Repomix | Implement Model Context Protocol server so AI assistants can query the dependency graph and request subgraph exports directly |
| **Token-aware export** | P1 | Repomix | Count tokens per file in exports. Warn when exceeding LLM context limits. Auto-truncate to fit budget |
| **Tree-sitter compression** | P2 | Repomix | --compress mode that extracts only signatures, types, and key code elements (not full source) for smaller AI context |
| **AI summary generation** | P2 | CodeSee | Auto-generate natural language summaries of modules and their relationships using LLM |
| **llms.txt support** | P3 | — | Generate llms.txt / llms-full.txt files describing the codebase structure for AI crawlers |

---

## Remaining TODO List

### Must-Have for v1.0 Release

- [ ] **Implement watch mode** — --watch flag is accepted but does nothing. Wire up notify crate to re-scan on file changes and push updates to the browser (WebSocket or SSE)
- [ ] **License server** — Currently offline-only validation with simple format check. Implement a license validation server for production key management (key generation, expiry, seat limits)
- [ ] **Remove DEV_MODE flag** — const DEV_MODE = true in web/src/main.ts bypasses Pro gate. Must be false before release
- [ ] **Cargo test CI** — Ensure cargo test --workspace runs in CI for all crates
- [ ] **Web build CI** — Add cd web && bun install && bun run build to CI pipeline
- [ ] **Binary releases** — Set up cross-compilation and GitHub Releases for macOS, Linux, Windows binaries
- [ ] **README with usage examples** — Installation, quick start, CLI reference, screenshots
- [ ] **npm/Homebrew distribution** — Package for common install channels: brew install xray, npx xray

### Should-Have Before Launch

- [ ] **Error handling audit** — Several places use unwrap() or silent tracing::warn!(). Add user-facing error messages for common failures (permission denied, unsupported file, parse errors)
- [ ] **Performance benchmarks** — Benchmark scan time on large repos (10K+ files). Profile and optimize hot paths
- [ ] **WASM build integration** — Document and test the WASM build pipeline. Ensure wasm-pack build works with current deps
- [ ] **Graph diffing** — Compare two graph snapshots and highlight added/removed/changed nodes and edges
- [ ] **Export breadth** — Support function-level exports (currently file-level only in xray export)
- [ ] **Unresolved import reporting** — Surface resolved: false edges in the UI and CLI output as warnings

### Nice-to-Have Polish

- [ ] **Keyboard shortcuts** — Arrow keys to navigate nodes, Esc to deselect, / to focus search
- [ ] **Zoom-to-node** — Double-click or search result click zooms camera to the node
- [ ] **Edge labels** — Show import symbol names on edges when zoomed in
- [ ] **Minimap** — Small overview window showing full graph with viewport indicator
- [ ] **Graph statistics dashboard** — Language distribution chart, dependency depth histogram, coupling metrics
- [ ] **Node filtering by language** — Toggle visibility of nodes by language (checkboxes in sidebar)
- [ ] **Persist layout positions** — Save node positions between sessions so layout does not reset
- [ ] **Config file** — .xray/config.yml for default options (port, exclude patterns, default level)

---

## Competitive Landscape

| Tool | Type | Languages | Free? | Key Differentiator |
|---|---|---|---|---|
| **CodeSee** | SaaS platform | All major | Freemium (from $29/mo) | Auto-updating maps, PR integration, AI Q&A |
| **Dependency Cruiser** | CLI (npm) | JS/TS/Coffee | Yes (OSS) | Rule-based validation, stability metrics, CI integration |
| **Madge** | CLI (npm) | JS/TS/CSS | Yes (OSS) | Simple circular dep detection, multiple layout engines |
| **Repomix** | CLI (npm) | All | Yes (OSS) | AI context packing, token counting, MCP server |
| **Sourcetrail** | Desktop app | C/C++/Java/Python | Yes (OSS, archived) | Interactive code exploration, deep indexing |
| **CodeScene** | SaaS | All major | Paid | Behavioral analysis, hotspot detection, team coupling |
| **Xray (us)** | CLI + browser | TS/Py/Rust/Go/Java | Freemium | Rust-native speed, WASM engine, AI export, blast radius |

### Xray Competitive Advantages

1. **Rust-native performance** — Tree-sitter parsing + petgraph + blake3 hashing. Faster than Node.js-based tools on large repos
2. **Multi-language from day one** — 5 languages vs Dependency Cruiser JS-only or Madge JS-only
3. **WASM engine** — GPU-ready positions, LOD filtering, and graph computation in the browser without server round-trips
4. **AI-first export** — Purpose-built AI context format (JSON + Markdown) with dependency ordering, unlike Repomix flat file packing
5. **Blast radius analysis** — Visual upstream/downstream impact analysis, not available in most competitors

### Where Competitors Are Ahead

1. **CodeSee**: Auto-updating maps on every merge, PR integration, team collaboration
2. **Dependency Cruiser**: Rule-based architecture validation, stability metrics, deep CI integration
3. **Repomix**: MCP server integration, token counting, broader ecosystem (VSCode ext, browser ext, web app)
4. **CodeScene**: Behavioral analysis, hotspot detection, commit-level insights
