#!/usr/bin/env bash
# makes bindings/c/include/anybox.h from the capi crate and copies medius.h next to it from the medius-capi
# anybox bundles. CI runs it and fails when the committed headers differ
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
cbindgen --quiet --config bindings/c/cbindgen.toml --crate anybox-capi --output bindings/c/include/anybox.h
medius_capi=$(cargo metadata --format-version 1 --locked |
  sed -n 's/.*"manifest_path":"\([^"]*medius-capi[\/]*Cargo.toml\)".*/\1/p' | head -n 1)
cp "$(dirname "${medius_capi//\\//}")/include/medius.h" bindings/c/include/medius.h
