# Xray

Xray is a fast, local-first codebase architecture visualizer. It scans a repository with tree-sitter, builds dependency graphs, and serves an interactive browser UI from a Rust CLI.

## Highlights

- **Offline by default** — source code is scanned locally and is not sent to external services.
- **Multi-language scanning** — TypeScript/JavaScript, Python, Rust, Go, and Java support are included.
- **Interactive graph UI** — explore file-level and function-level relationships in the browser.
- **Architecture analysis** — inspect cycles, orphan files, directory groups, annotations, and blast radius.
- **AI context export** — export graph slices as structured JSON or Markdown for LLM/agent context.

> Status: early-stage project. APIs, CLI flags, and output formats may change before v1.0.

## Installation

### From source

```bash
git clone https://github.com/antoinehoriot/xray-viz.git
cd xray-viz
cargo install --path crates/xray-cli
```

### Development build

```bash
cargo build --all
```

To build the full web UI bundle:

```bash
cd web
bun install
bun run build
```

If the web bundle has not been built, `xray view` serves a minimal fallback UI.

## Usage

Open an interactive graph for the current repository:

```bash
xray view .
```

Scan and print graph JSON:

```bash
xray scan . --pretty
```

Show cycle and orphan reports:

```bash
xray scan . --cycles --orphans
```

Write graph output to a file:

```bash
xray scan . --output graph.json
```

Watch for changes while serving the UI:

```bash
xray view . --watch
```

Export AI/LLM context from a starting node:

```bash
xray export . --from src/main.rs --depth 2 --format markdown
```

## Repository layout

```text
crates/xray-core/   Core scanner, cache, graph, and layout logic
crates/xray-cli/    Command-line interface and embedded HTTP server
crates/xray-wasm/   WebAssembly bindings for browser graph operations
web/                TypeScript/Sigma.js browser UI
grammars/           Tree-sitter queries
tests/fixtures/     Language parser fixtures
docs/               Roadmap and planning docs
```

## Development

Run Rust quality checks:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

Run web checks:

```bash
cd web
bun install
bun run typecheck
bun run build
```

## Privacy and generated files

Xray writes scan caches and annotations under `.xray/` in the repository being scanned. These files are local working data and should not be committed unless you intentionally want to share annotations.

Before opening a pull request, verify that no local secrets, caches, build artifacts, or machine-specific config are staged:

```bash
git status --short
git diff --cached --name-only
```

## Contributing

Contributions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md) before opening an issue or pull request.

## Security

Please report security issues privately using the process in [SECURITY.md](SECURITY.md).

## License

This project is licensed under the [MIT License](LICENSE).
