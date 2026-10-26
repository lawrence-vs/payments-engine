#!/bin/sh
set -eu
cd "$(dirname "$0")"
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -q
cargo audit
for f in src/*.rs; do
    head -n 1 "$f" | grep -q '^//!' || { echo "check.sh: $f has no one-line //! module doc"; exit 1; }
done
if git grep -n -i -E 'engineer to fill in|draft \(claude\)' -- '*.md'; then
    echo "check.sh: unfinished text above"; exit 1
fi
