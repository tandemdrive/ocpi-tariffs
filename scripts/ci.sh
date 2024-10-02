#!/bin/sh

set -e

start=$(date -Iseconds -u)
host_name=$(hostname)
echo "Starting build at: ${start} on ${host_name}"

export CARGO_TARGET_DIR="${BUILD_OUTPUT}"
export RUST_BACKTRACE="full"
export PATH="/var/whack/.cargo/bin:$PATH"

cargo deny --workspace -L info check
cargo check
cargo clippy
cargo doc --workspace --no-deps
cargo test
