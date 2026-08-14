#!/usr/bin/env bash
set -euo pipefail

cargo fmt --all --check
cargo test --workspace --release --locked
cargo clippy --workspace --all-targets --release --locked -- -D warnings
git diff --check
