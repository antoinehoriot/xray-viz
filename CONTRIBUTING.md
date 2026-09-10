# Contributing

Thanks for your interest in contributing to Xray.

## Ground rules

- Keep source scanning local-first and privacy-preserving.
- Do not commit secrets, private keys, tokens, machine-specific config, generated caches, or build artifacts.
- Prefer small pull requests with a clear description and tests when practical.
- Run formatting and tests before submitting.

## Local setup

```bash
git clone https://github.com/antoinehoriot/xray-viz.git
cd xray-viz
cargo build --all
cargo test --all
```

For web UI changes:

```bash
cd web
bun install
bun run typecheck
bun run build
```

## Quality checks

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

## Sensitive files

Before committing, check staged files carefully:

```bash
git diff --cached --name-only
git diff --cached
```

Never commit:

- `.env` files or credentials
- API keys, license keys, access tokens, private keys, certificates
- local `.xray/` caches (`cache.db`, `graph.bin`) unless a future workflow explicitly documents otherwise
- `target/`, `web/dist/`, `web/node_modules/`, coverage output, or logs
- machine-specific editor/tool configuration

If a secret is accidentally committed, rotate it immediately and disclose the incident to the maintainers.
