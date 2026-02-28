# Xray dev tooling — just recipes
# Install: cargo install just
# Usage:   just <recipe>

default:
    @just --list

# ── Build ──────────────────────────────────────────────────────────────────────

# Build all Rust crates (native)
build:
    cargo build --all

# Build in release mode
build-release:
    cargo build --all --release

# Build WASM crate with wasm-pack (web target)
build-wasm:
    wasm-pack build crates/xray-wasm --target web

# Build WASM in release mode with size optimization
build-wasm-release:
    wasm-pack build crates/xray-wasm --target web --release

# Build the web UI (requires npm/bun)
build-web:
    cd web && bun install && bun run build

# ── Development ────────────────────────────────────────────────────────────────

# Watch Rust files and rebuild on change
watch:
    cargo watch -x "build --all"

# Watch + run tests
watch-test:
    cargo watch -x "test --all"

# Watch web UI (TypeScript)
watch-web:
    cd web && bun run dev

# ── Quality Gates ──────────────────────────────────────────────────────────────

# Run all tests
test:
    cargo test --all

# Run tests with output
test-verbose:
    cargo test --all -- --nocapture

# Lint with clippy
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# Format check
fmt-check:
    cargo fmt --all -- --check

# Format (write)
fmt:
    cargo fmt --all

# Typecheck web UI
typecheck-web:
    cd web && bun run typecheck

# Run all quality gates
check: fmt-check lint test typecheck-web

# ── Run ────────────────────────────────────────────────────────────────────────

# Scan current directory and print graph JSON
scan path=".":
    cargo run --bin xray -- scan {{path}} --pretty --stats

# View current directory in browser (M2)
view path=".":
    cargo run --bin xray -- view {{path}}

# ── WASM Dev ───────────────────────────────────────────────────────────────────

# Rebuild WASM + copy to web/public for hot-reload
wasm-dev:
    wasm-pack build crates/xray-wasm --target web --dev
    cp crates/xray-wasm/pkg/xray_wasm* web/public/ 2>/dev/null || true

# ── Clean ──────────────────────────────────────────────────────────────────────

clean:
    cargo clean
    rm -rf crates/xray-wasm/pkg web/dist

# ── CI ─────────────────────────────────────────────────────────────────────────

# Run full CI suite locally (matches .github/workflows/ci.yml)
ci: check build-wasm
    @echo "✓ All CI checks passed"
