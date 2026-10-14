#!/usr/bin/env bash
# builds the C smoke test as C99, C11 and C++17 against the mock build and runs each. MSVC has no C99 mode, so
# Windows does C11, C17 and C++17. it needs cl (from a developer prompt) on Windows, gcc and g++ elsewhere
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
if [ "${OS:-}" = "Windows_NT" ]; then
  cl=$(command -v cl) || { echo "test-c: no cl on the path, run this from a developer prompt (vcvars64.bat)" >&2; exit 1; }
  # Git Bash puts its own link.exe ahead of MSVC's on the path, and cargo would pick that one
  export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER="$(dirname "$cl")/link.exe"
fi
cargo build -p anybox-capi --features mock --locked
out=target/c-smoke
mkdir -p "$out"
src=bindings/c/tests/smoke.c
layout=bindings/c/tests/layout.txt
if [ "${OS:-}" = "Windows_NT" ]; then
  # the DLL has to sit next to the exe for Windows to find it
  cp target/debug/anybox.dll "$out/"
  # cl takes its options with a dash too, which Git Bash leaves alone where it would rewrite a slash as a path
  for std in "-std:c11" "-std:c17" "-std:c++17 -TP"; do
    name=$(echo "$std" | tr -dc 'a-z0-9+' | tr '+' 'p')
    # shellcheck disable=SC2086
    # _CRT_SECURE_NO_WARNINGS keeps MSVC from calling plain fopen deprecated
    cl -nologo -W4 -WX $std -D_CRT_SECURE_NO_WARNINGS -DANYBOX_FEATURE_MOCK -Ibindings/c/include "$src" \
      -Fe"$out/smoke-$name.exe" -Fo"$out/smoke-$name.obj" -link target/debug/anybox.dll.lib
    "$out/smoke-$name.exe" "$layout"
  done
else
  for std in "gcc -std=c99" "gcc -std=c11" "g++ -std=c++17 -x c++"; do
    name=$(echo "$std" | tr -dc 'a-z0-9+' | tr '+' 'p')
    # shellcheck disable=SC2086
    $std -Wall -Wextra -Werror -DANYBOX_FEATURE_MOCK -Ibindings/c/include "$src" -x none \
      -Ltarget/debug -lanybox -Wl,-rpath,"$PWD/target/debug" -o "$out/smoke-$name"
    "$out/smoke-$name" "$layout"
  done
fi
