#!/bin/sh

set -e

export CARGO_TARGET_DIR="target"
export RUST_BACKTRACE="full"

cargo deny --workspace -L info check
cargo check
cargo clippy
cargo doc --workspace --no-deps
cargo test
