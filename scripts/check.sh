#!/usr/bin/env bash
# the gates every commit and push has to pass, the same ones CI runs
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
cargo fmt --all --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-features --locked
bash scripts/check-docs.sh
