# Public Release Checklist

Use this checklist before making the repository public or cutting a release.

## Repository hygiene

- [ ] `git status --short` contains only intentional source/documentation changes.
- [ ] `git diff --cached --name-only` contains no secrets, local caches, build artifacts, or machine-specific config.
- [ ] `.xray/`, `target/`, `web/dist/`, `web/node_modules/`, logs, and local tool runtime files are not staged.
- [ ] `rg --hidden "(api[_-]?key|secret|token|password|private[_-]?key|BEGIN .*PRIVATE)"` has been reviewed.
- [ ] No personal paths, private URLs, internal credentials, or proprietary customer data appear in committed files.

## Project metadata

- [ ] `README.md` is current.
- [ ] `LICENSE` matches `Cargo.toml` metadata.
- [ ] `CONTRIBUTING.md` and `SECURITY.md` are present.
- [ ] GitHub settings enable private vulnerability reporting if available.

## Quality gates

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo test --all`
- [ ] `cd web && bun install && bun run typecheck && bun run build`

## Publishing

- [ ] Confirm whether crates should be published to crates.io or remain source-only.
- [ ] Confirm binary/package names do not conflict with existing projects.
- [ ] Tag the release only after all checks pass.
