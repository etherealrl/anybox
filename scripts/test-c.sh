#!/usr/bin/env bash
# builds the C smoke test as C99, C11 and C++17 against the mock build and runs each. MSVC has no C99 mode, so
# Windows does C11, C17 and C++17. then the C++ wrapper's tests (with the mock and without it), its fake, its
# conformance runner, which needs the expected.json the capi crate's tests record, its Medius tests over medius'
# own mock, and the Medius suite's runner, which needs the medius-expected.json they record too, and builds the C and
# C++ examples in docs/bindings.md and the hardware programs. it needs gcc and g++, or cl on Windows, which check.sh
# loads through scripts/msvc.sh
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
if [ "${OS:-}" = "Windows_NT" ]; then
  cl=$(command -v cl) || { echo "test-c: no cl on the path, run it through check.sh, which loads it, or from a developer prompt" >&2; exit 1; }
  # Git Bash puts its own link.exe ahead of MSVC's on the path, and cargo would pick that one
  export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER="$(dirname "$cl")/link.exe"
fi
cargo build -p anybox-capi --features mock --locked
# wherever cargo builds to, like the Python and C# tests take it
lib="${CARGO_TARGET_DIR:-target}/debug"
out=target/c-smoke
mkdir -p "$out"
src=bindings/c/tests/smoke.c
layout=bindings/c/tests/layout.txt
# a docs example without its own includes is a fragment, so it goes in a function that gets the device
rm -rf "$out/docs"
mkdir -p "$out/docs"
awk -v dir="$out/docs" '/^```(c|cpp)$/ {n++; file = dir "/example" n "." substr($0, 4); next}
  /^```/ {file = ""; next} file != "" {print > file}' docs/bindings.md
for example in "$out"/docs/example*; do
  if ! grep -q '^#include' "$example"; then
    {
      echo '#include "anybox.hpp"'
      echo 'void example(const anybox::Device& device) {'
      cat "$example"
      echo '}'
    } > "$example.full"
    mv "$example.full" "$example"
  fi
done
if [ "${OS:-}" = "Windows_NT" ]; then
  # the DLL has to sit next to the exe for Windows to find it
  cp "$lib/anybox.dll" "$out/"
  # cl takes its options with a dash too, which Git Bash leaves alone where it would rewrite a slash as a path
  for std in "-std:c11" "-std:c17" "-std:c++17 -TP -permissive-"; do
    name=$(echo "$std" | tr -dc 'a-z0-9+' | tr '+' 'p')
    # shellcheck disable=SC2086
    # _CRT_SECURE_NO_WARNINGS keeps MSVC from calling plain fopen deprecated
    cl -nologo -W4 -WX $std -D_CRT_SECURE_NO_WARNINGS -DANYBOX_FEATURE_MOCK -Ibindings/c/include "$src" \
      -Fe"$out/smoke-$name.exe" -Fo"$out/smoke-$name.obj" -link "$lib/anybox.dll.lib"
    "$out/smoke-$name.exe" "$layout"
  done
  cpp() {
    local name="$1"
    shift
    mkdir -p "$out/$name"
    # -permissive- turns off what only MSVC allows, otherwise code like that passes here and breaks the Linux build
    # shellcheck disable=SC2068
    cl -nologo -W4 -WX -permissive- -std:c++17 -EHsc -Ibindings/c/include $@ -Fe"$out/$name.exe" -Fo"$out/$name/" \
      -link "$lib/anybox.dll.lib"
  }
  cpp cpp-tests -DANYBOX_FEATURE_MOCK bindings/c/tests/cpp/tests.cpp bindings/c/tests/cpp/second.cpp
  cpp cpp-tests-no-mock bindings/c/tests/cpp/tests.cpp bindings/c/tests/cpp/second.cpp
  cpp cpp-fake -DANYBOX_FEATURE_MOCK bindings/c/tests/cpp/fake.cpp
  cpp cpp-conformance -DANYBOX_FEATURE_MOCK bindings/c/tests/cpp/conformance.cpp
  cpp cpp-medius -DANYBOX_FEATURE_MOCK -DMEDIUS_FEATURE_MOCK bindings/c/tests/cpp/medius.cpp bindings/c/tests/cpp/second.cpp
  cpp cpp-medius-suite -DANYBOX_FEATURE_MOCK -DMEDIUS_FEATURE_MOCK bindings/c/tests/cpp/medius_suite.cpp
  for program in cpp-tests cpp-tests-no-mock cpp-fake cpp-conformance cpp-medius cpp-medius-suite; do
    "$out/$program.exe"
  done
  # the C example's function is static and nothing calls it on its own (C4505)
  for example in "$out"/docs/example*; do
    case "$example" in
    *.c) cl -nologo -W4 -WX -wd4505 -std:c11 -Ibindings/c/include -c "$example" -Fo"$example.obj" ;;
    *) cl -nologo -W4 -WX -permissive- -std:c++17 -EHsc -Ibindings/c/include -c "$example" -Fo"$example.obj" ;;
    esac
  done
  # the hardware programs only run on a box, so here they just have to build, with the flags the hardware run uses
  cl -nologo -W4 -WX -Ibindings/c/include -c bindings/c/tests/hardware.c -Fo"$out/hardware-c.obj"
  cl -nologo -W4 -WX -permissive- -std:c++17 -EHsc -Ibindings/c/include -c bindings/c/tests/hardware.cpp \
    -Fo"$out/hardware-cpp.obj"
else
  for std in "gcc -std=c99" "gcc -std=c11" "g++ -std=c++17 -x c++"; do
    name=$(echo "$std" | tr -dc 'a-z0-9+' | tr '+' 'p')
    # shellcheck disable=SC2086
    $std -Wall -Wextra -Werror -DANYBOX_FEATURE_MOCK -Ibindings/c/include "$src" -x none \
      -L"$lib" -lanybox -Wl,-rpath,"$(cd "$lib" && pwd)" -o "$out/smoke-$name"
    "$out/smoke-$name" "$layout"
  done
  cpp() {
    local name="$1"
    shift
    # shellcheck disable=SC2068
    g++ -std=c++17 -Wall -Wextra -Werror -pthread -Ibindings/c/include $@ -L"$lib" -lanybox \
      -Wl,-rpath,"$(cd "$lib" && pwd)" -o "$out/$name"
  }
  cpp cpp-tests -DANYBOX_FEATURE_MOCK bindings/c/tests/cpp/tests.cpp bindings/c/tests/cpp/second.cpp
  cpp cpp-tests-no-mock bindings/c/tests/cpp/tests.cpp bindings/c/tests/cpp/second.cpp
  cpp cpp-fake -DANYBOX_FEATURE_MOCK bindings/c/tests/cpp/fake.cpp
  cpp cpp-conformance -DANYBOX_FEATURE_MOCK bindings/c/tests/cpp/conformance.cpp
  cpp cpp-medius -DANYBOX_FEATURE_MOCK -DMEDIUS_FEATURE_MOCK bindings/c/tests/cpp/medius.cpp bindings/c/tests/cpp/second.cpp
  cpp cpp-medius-suite -DANYBOX_FEATURE_MOCK -DMEDIUS_FEATURE_MOCK bindings/c/tests/cpp/medius_suite.cpp
  for program in cpp-tests cpp-tests-no-mock cpp-fake cpp-conformance cpp-medius cpp-medius-suite; do
    "$out/$program"
  done
  for example in "$out"/docs/example*; do
    case "$example" in
    # the C example's function is static and nothing calls it on its own
    *.c) gcc -std=c11 -Wall -Wextra -Werror -Wno-unused-function -Ibindings/c/include -c "$example" -o "$example.o" ;;
    *) g++ -std=c++17 -Wall -Wextra -Werror -Ibindings/c/include -c "$example" -o "$example.o" ;;
    esac
  done
  gcc -Wall -Wextra -Werror -Ibindings/c/include -c bindings/c/tests/hardware.c -o "$out/hardware-c.o"
  g++ -std=c++17 -Wall -Wextra -Werror -pthread -Ibindings/c/include -c bindings/c/tests/hardware.cpp -o "$out/hardware-cpp.o"
fi
