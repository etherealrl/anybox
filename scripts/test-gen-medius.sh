#!/usr/bin/env bash
# feeds gen-medius a medius.h with one thing changed at a time and checks it refuses each one with the right reason,
# so a medius bump can't slip past the tables. it reads the changed header with --header and writes nothing (--dry)
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
header=bindings/c/include/medius.h
failed=0

# the unchanged header has to pass, otherwise every case below would fail for the wrong reason
dotnet run bindings/csharp/tools/gen-medius.cs -- --dry --header "$header" >/dev/null

refuses() {
  local what="$1" expected="$2" script="$3"
  sed -e "$script" "$header" > "$work/medius.h"
  if cmp -s "$header" "$work/medius.h"; then
    echo "test-gen-medius: the change for \"$what\" didn't change medius.h, fix its sed script" >&2
    failed=1
    return
  fi
  local out
  if out="$(dotnet run bindings/csharp/tools/gen-medius.cs -- --dry --header "$work/medius.h" 2>&1)"; then
    echo "test-gen-medius: $what went through" >&2
    failed=1
  elif ! grep -qF "$expected" <<< "$out"; then
    echo "test-gen-medius: $what was refused without \"$expected\":" >&2
    echo "$out" >&2
    failed=1
  else
    echo "ok: $what"
  fi
}

refuses "a new function" "medius_device_brand_new is new in medius.h" \
  's/^MediusStatus medius_device_reset(struct MediusDevice \*dev);$/&\nMediusStatus medius_device_brand_new(struct MediusDevice *dev);/'
refuses "a function gone" "medius_device_reapply has an entry in Tables.Functions but medius.h doesn't have it" \
  '/^MediusStatus medius_device_reapply(struct MediusDevice \*dev);$/d'
refuses "a changed function written by hand" "medius_device_set_bearing changed in medius.h" \
  's/^\(MediusStatus medius_device_set_bearing(struct MediusDevice \*dev,\)$/\1 uint8_t extra,/'
refuses "a generated function with a new byte" "medius_device_set_spread's extra is a byte" \
  's/^MediusStatus medius_device_set_spread(struct MediusDevice \*dev, uint16_t percent);$/MediusStatus medius_device_set_spread(struct MediusDevice *dev, uint16_t percent, uint8_t extra);/'
refuses "a generated function that grew a pointer" "medius_device_wheel's count (uint32_t*) fits no shape" \
  's/^MediusStatus medius_device_wheel(struct MediusDevice \*dev, int16_t delta);$/MediusStatus medius_device_wheel(struct MediusDevice *dev, int16_t delta, uint32_t *count);/'
refuses "a new byte in a generated struct" "MediusRate.newflag is a byte" \
  's/^    uint8_t change_driven;$/&\n    uint8_t newflag;/'
refuses "a changed struct written by hand" "MediusUsage changed in medius.h, so check Usage" \
  's/^    uint16_t id;$/    uint32_t id;/'
refuses "a new struct" "MediusBrandNew is new in medius.h" \
  's/^typedef struct MediusStamped {$/typedef struct MediusBrandNew {\n    uint16_t value;\n} MediusBrandNew;\n\n&/'
refuses "a new member of an enum written against by hand" "MediusBlanket's members changed" \
  's/^    MEDIUS_BLANKET_MEDIA = 4,$/&\n    MEDIUS_BLANKET_PAN = 5,/'
refuses "a new enum" "MediusBrandNew is new in medius.h, give it an entry in Tables.Enums" \
  's/^\/\/ What paces injected motion\.$/enum MediusBrandNew\n#if defined(__cplusplus) || __STDC_VERSION__ >= 202311L\n  : uint8_t\n#endif \/\/ defined(__cplusplus) || __STDC_VERSION__ >= 202311L\n {\n    MEDIUS_BRAND_NEW_ONE = 0,\n};\n\n&/'
refuses "a generated function that's only there with the mock now" "medius_device_reset is only there with the mock" \
  's/^MediusStatus medius_device_reset(struct MediusDevice \*dev);$/#if defined(MEDIUS_FEATURE_MOCK)\n&\n#endif/'
refuses "a function written by hand that's only there with the mock now" "medius_device_find is only there with the mock now" \
  's/^MediusStatus medius_device_find(struct MediusDevice \*\*out);$/#if defined(MEDIUS_FEATURE_MOCK)\n&\n#endif/'
refuses "a presence flag gone" "MediusBoxInfo.device's role if(has_device) names has_device" \
  '/^    uint8_t has_device;$/d'
refuses "a count too narrow for its array" "MediusRewriteRule.payload_len is a uint8_t, which can't count the 512 payload can take" \
  's/^    uint16_t payload_len;$/    uint8_t payload_len;/'
refuses "a new field in a record kept in Rust's order" "MediusClipPacketTrigger's Order in Tables.Records has to name every one of its fields" \
  's/^    uint8_t selector_len;$/&\n    uint16_t window;/'

exit "$failed"
