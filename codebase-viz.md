# Xray — Instant Codebase Architecture Visualizer

## Problem

Developers spend 80%+ of their time understanding code and only 5% writing it. Onboarding takes over a month for 70%+ of engineers — largely because no lightweight tool exists to produce an interactive, accurate map of how a codebase actually fits together.

Existing tools fall short in every direction: Sourcetrail was archived in 2021 and never supported Go, Rust, or TypeScript. Cloud-based options like CodeSee require uploading proprietary code to third-party servers, a non-starter for most teams. IDE plugins are language-specific, tightly coupled to one editor, and produce static views that go stale the moment someone commits. Documentation sites show what someone wrote, not what the code actually does.

The result: every new hire re-reads the same unfamiliar directories, senior engineers become human navigators, and AI agents lack the structural context they need to reason reliably about large repositories.

## Product

A Rust CLI that runs `xray .` in any repository and opens a browser with an interactive, real-time architecture map of the codebase.

**Core features:**
- **Multi-language parsing:** Tree-sitter grammars for JS/TS, Python, Go, Java, Rust, and C/C++ — parse once, render everywhere.
- **File-level and function-level graphs:** Dependency edges, call graphs, module clusters, blast radius overlays.
- **WASM browser renderer:** Ships as a single static binary; the interactive graph runs in-browser via WebAssembly with no server required.
- **Blast radius analysis:** Select any file or function, highlight every downstream caller and dependency.
- **AI context export:** Export a structured JSON or Markdown slice of the graph as context for LLM prompts or agent sessions.
- **Speed:** 100K files parsed in seconds on commodity hardware.

MVP scope: Rust CLI, tree-sitter for JS/TS + Python + Rust, file-level dependency extraction, WASM graph renderer with zoom/select/trace. Add Go/Java grammars and function-level resolution in subsequent iterations.

## Revenue Model

**Tiered CLI licensing:**
- Free: OSS CLI, basic architecture map, 3 languages — drives adoption and word-of-mouth.
- Pro $12/month or $99/year: all languages, function-level call graphs, AI context export, blast radius overlays.
- Team $29/month/seat: shared living architecture docs (auto-updated on push), team annotations, onboarding dashboards.

**Comparable pricing:** Sourcetrail (free but dead). CodeSee $29+/month cloud-only. Scitools Understand ~$70/month/seat. Xray undercuts on price and wins on privacy — code never leaves the developer's machine.

**Target: $8K MRR within 6 months** — ~270 Pro subscribers or ~90 Team seats.

## Competition

| Competitor | Strength | Weakness |
|---|---|---|
| Sourcetrail | ~14K GitHub stars, well-known brand | Archived Dec 2021, no Go/Rust/TS, 2019-era tech, community forks struggling |
| CodeSee | Auto-generated codebase maps, polished UX | Cloud-only, requires code upload, $29+/month SaaS |
| Scitools Understand | Large codebase support, code metrics | ~$70/month/seat, dated UI, enterprise sales cycle |
| CodeLayers | 3D viz on iOS/Vision Pro, AI agent integration | Apple-only, niche audience, 2026 early-stage |
| Codedocent | AI summaries for non-programmers | Early-stage, cloud-dependent, Feb 2026 Show HN |
| code-maat | Historical analysis from version control logs | Not real-time architecture visualization |

**Differentiation:** Single offline binary, multi-language tree-sitter parsing, WASM browser renderer — no cloud, no subscription lock-in for the free tier, and 100K-file performance no desktop tool currently matches.

## Target Audience

- **Primary:** Individual engineers and small teams (2-15 developers) onboarding to large or unfamiliar codebases. Senior developers navigating monorepos or legacy systems.
- **Secondary:** AI/LLM practitioners who need structured codebase context for agent sessions and prompt engineering.
- **Where to find them:** Hacker News (multiple codebase viz Show HN posts in Feb 2026 signal active demand), developer Twitter/X, r/rust, r/golang, r/programming, dev-focused newsletters (TLDR, Bytes, Console), OSS communities around tree-sitter.

## Effort Estimate

**Size: M (2-3 weeks for MVP)**

- Week 1: Rust CLI scaffolding, tree-sitter integration (JS/TS + Python + Rust), file-level dependency extraction, graph data structure.
- Week 2: WASM browser renderer, interactive graph UI (zoom, node select, path trace), blast radius highlighting.
- Week 3: Add Go and Java grammars, performance tuning for 100K+ file repos, landing page and Pro billing.

**Main technical challenges:**
- Grammar quality varies across tree-sitter community parsers — some languages need custom resolution logic.
- Browser graph layout for large codebases (1K+ nodes) requires careful rendering performance work.
- WASM binary size management — keeping the download small enough for fast cold starts.

## Signals

- [Anti-Bloat Software Movement](../signals/forums.md) — Developer backlash against cloud-dependent, heavyweight tooling; appetite for fast local binaries.
- [Terminal-Based Developer Productivity Tools](../signals/forums.md) — Strong HN traction for CLI-first tools that stay out of the way.
- [Privacy-First and Offline-First Tools](../signals/forums.md) — Teams unwilling to upload source code to third-party services; local processing is a hard requirement.
- [AI-Generated Code Quality Crisis](../signals/forums.md) — AI-generated code compounds the need to understand structure; agents need architectural context.
- [Vibe Coding Hangover](../signals/social.md) — Developers accumulating AI-generated code they can't navigate; architecture maps reduce rework.
- [AI Agent Debugging and Safety Layer](../signals/forums.md) — Agents operating without codebase structure context make unsafe changes; Xray's AI export feature addresses this directly.
