# Contributing to linex

Thank you for considering contributing to **linex**!

## Guidelines & Principles

- **Zero External Dependencies**: `Cargo.toml` must not have external runtime dependencies (`[dependencies]` remains empty). All OS interactions go through `src/sys.rs` using direct POSIX / Darwin C ABI.
- **Strict Codebase Budget**: Total line count across all `src/*.rs` must strictly remain $\le 1000$ lines.
- **Zero Allocation**: Hot telemetry formatting paths must remain zero-heap allocation, using stack buffers (`Out`).
- **Conventional Commits**: Commit messages must follow Conventional Commits 1.0.0.

## Quality Gates

Before opening a pull request, ensure all checks pass:

```bash
just check-all
```

This runs:
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo nextest run --all-targets`
- `cargo deny check`
- `gitleaks detect --no-banner`
- `cargo machete`
- `actionlint`
- `zizmor`

## Community & Questions

If you have questions or ideas, feel free to open an issue or discussion on GitHub.
