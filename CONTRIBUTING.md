# Contributing

Read `README.md` and `docs/DEVELOPMENT.md` before changing code. Keep changes scoped to the owning crate or documented integration boundary.

Use Rust toolchain `1.85.1`. Before opening a pull request, run:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Do not commit generated build output, local runtime coordination state, credentials, or host-specific configuration. Explain behavior changes and verification in pull requests.
