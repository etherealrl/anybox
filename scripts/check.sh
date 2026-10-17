#!/usr/bin/env bash
# the gates every commit and push has to pass, the same ones CI runs
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
if [ "${OS:-}" = "Windows_NT" ]; then
  # cargo can't link anything on Windows without Visual Studio, so this one stops instead of skipping like the
  # other missing tools
  source scripts/msvc.sh
fi
if [ -f scripts/tests/git-identity.sh ]; then
  bash scripts/tests/git-identity.sh
fi
cargo fmt --all --check
# the plain calls in every binding have to be what bindings/gen makes from its table now
cargo run --quiet -p anybox-gen --locked -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
# docs.rs builds the docs with every feature, and a broken link only shows there otherwise. only the crate, since
# the capi's library has the same name
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features --locked -p anybox-rs --quiet
bash scripts/check-docs.sh
# the committed headers have to be what the capi crate makes. cbindgen's output changes between versions, so it's
# the one CI installs or nothing
cbindgen_version=$(sed -n 's/.*cargo install cbindgen --version \([0-9.]*\).*/\1/p' .github/workflows/ci.yml)
if [ "$(cbindgen --version 2>/dev/null)" = "cbindgen $cbindgen_version" ]; then
  bash scripts/gen-header.sh
  git diff --exit-code -- bindings/c/include
else
  echo "check: skipped the header check, it needs cbindgen $cbindgen_version"
fi
if [ "${OS:-}" = "Windows_NT" ]; then
  (msvc && bash scripts/test-c.sh)
elif command -v gcc >/dev/null; then
  bash scripts/test-c.sh
else
  echo "check: skipped the C smoke test, no C compiler found"
fi
# the oldest Python the package takes and the newest one, uv fetches whichever is missing. the medius package is
# the one the medius extra pins, which a test holds to the medius inside the library
if command -v uv >/dev/null; then
  medius=$(sed -n 's/^medius = \["\(medius==[0-9.]*\)"\]$/\1/p' bindings/python/pyproject.toml)
  for python in 3.11 3.14; do
    uv run --no-project --python "$python" --with pytest --with "$medius" pytest -q bindings/python/tests
  done
  # a pinned ruff, so a new one's new rules can't fail a commit that didn't touch Python
  uvx ruff@0.17.1 format --check bindings/python
  uvx ruff@0.17.1 check bindings/python
else
  echo "check: skipped the Python tests, uv isn't installed"
fi
# the C# tests, the global.json next to them turns on the test runner they need
if command -v dotnet >/dev/null; then
  (cd bindings/csharp && dotnet test --project tests/Anybox.Tests)
  # the C# examples in the docs have to build, each one as a file-based app on the package, with no warnings
  rm -rf target/docs-csharp
  mkdir -p target/docs-csharp
  project="$(pwd -W 2>/dev/null || pwd)/bindings/csharp/src/Anybox/Anybox.csproj"
  for doc in docs/bindings.md bindings/csharp/README.md; do
    awk -v dir=target/docs-csharp -v name="$(basename "$(dirname "$doc")")" -v project="$project" \
      '/^```csharp$/ {n++; file = dir "/" name n ".cs"; print "#:project " project > file
        print "#:property TreatWarningsAsErrors=true" > file; next}
       /^```/ {file = ""; next} file != "" {print > file}' "$doc"
  done
  for example in target/docs-csharp/*.cs; do
    dotnet build --nologo -v q "$example"
  done
  # the hardware program only runs on a box, so here it just has to build
  (cd bindings/csharp && dotnet build tests/Hardware --nologo -v q -p:TreatWarningsAsErrors=true)
else
  echo "check: skipped the C# tests, dotnet isn't installed"
fi
# the C# and C++ sides of medius.h and the Rust layout test are generated, so they have to be what the generator
# makes now, and the generator has to refuse a medius.h its tables don't fit
if command -v dotnet >/dev/null; then
  dotnet run bindings/csharp/tools/gen-medius.cs
  git diff --exit-code -- bindings/csharp/src/Anybox/Medius/Generated.cs bindings/csharp/src/Anybox/Medius/Records.cs bindings/c/tests/medius_layout.rs \
    bindings/c/include/anybox_medius.hpp bindings/c/src/medius_suite/generated.rs \
    bindings/c/tests/cpp/medius_suite.hpp bindings/csharp/tests/Anybox.Tests/Medius/SuiteTables.cs
  bash scripts/test-gen-medius.sh
fi
