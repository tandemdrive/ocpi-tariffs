#!/bin/sh

set -e

export CARGO_TARGET_DIR="target"
export RUST_BACKTRACE="full"

cargo deny --workspace --all-features -L info check
cargo check --workspace --all-features --verbose
cargo fmt --all --check
cargo clippy --workspace --all-features --all-targets
cargo doc --workspace --all-features --no-deps --document-private-items
cargo test --workspace --all-features --verbose
