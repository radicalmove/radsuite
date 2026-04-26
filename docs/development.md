# RADsuite Development

## Local Checks

Run from the repository root:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Scope

This repository is the new Rust/Tauri implementation of RADsuite. The existing Python apps remain reference implementations only.
