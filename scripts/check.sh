#!/usr/bin/env bash
# the gates every commit and push has to pass, the same ones CI runs
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
if [ -f scripts/tests/git-identity.sh ]; then
  bash scripts/tests/git-identity.sh
fi
cargo fmt --all --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-features --locked
bash scripts/check-docs.sh
