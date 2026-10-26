#!/bin/sh
set -eu
cd "$(dirname "$0")"
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo doc --no-deps -q
cargo audit
