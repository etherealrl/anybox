#!/usr/bin/env bash
# the gates every commit and push has to pass, the same ones CI runs
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
if [ -f scripts/tests/git-identity.sh ]; then
  bash scripts/tests/git-identity.sh
fi
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
bash scripts/check-docs.sh
# the committed headers have to be what the capi crate makes, CI always has cbindgen for it
if command -v cbindgen >/dev/null; then
  bash scripts/gen-header.sh
  git diff --exit-code -- bindings/c/include
else
  echo "check: skipped the header check, cbindgen isn't installed"
fi
# Windows needs cl from a developer prompt, so outside one this skips and CI runs it in one on its own
if [ "${OS:-}" = "Windows_NT" ]; then
  compiler=cl
else
  compiler=gcc
fi
if command -v "$compiler" >/dev/null; then
  bash scripts/test-c.sh
else
  echo "check: skipped the C smoke test, no C compiler found"
fi
# the oldest Python the package takes and the newest one, uv fetches whichever is missing
if command -v uv >/dev/null; then
  for python in 3.11 3.14; do
    uv run --no-project --python "$python" --with pytest --with medius==3.4.3 pytest -q bindings/python/tests
  done
else
  echo "check: skipped the Python tests, uv isn't installed"
fi
