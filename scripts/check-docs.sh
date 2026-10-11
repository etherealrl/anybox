#!/usr/bin/env bash
# fails on em or en dashes and on personal identifiers in any file git would commit.
# the identifiers live outside the repo, one per line, in .git/info/identifiers or in
# the file ANYBOX_IDENTIFIERS points to, so the names themselves never get committed
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
export LC_ALL=C.UTF-8

files=()
while IFS= read -r -d '' f; do
  [ -f "$f" ] && files+=("$f")
done < <(git ls-files -z --cached --others --exclude-standard)

# the upstream skills are kept as unchanged copies, so their dashes stay
own=()
for f in "${files[@]}"; do
  case "$f" in
    .agents/skills/codebase-design/* | .agents/skills/rust-best-practices/*) ;;
    *) own+=("$f") ;;
  esac
done

status=0

# grep exits 2 on its own errors (a missing UTF-8 locale for -P, say), that has to fail too
rc=0
grep -nP '[\x{2013}\x{2014}]' -- "${own[@]}" || rc=$?
if [ "$rc" -eq 0 ]; then
  echo "check-docs: em or en dash found, use a comma, a full stop or a plain hyphen" >&2
  status=1
elif [ "$rc" -gt 1 ]; then
  echo "check-docs: the dash scan itself failed" >&2
  exit 2
fi

identifiers="${ANYBOX_IDENTIFIERS:-$(git rev-parse --git-path info/identifiers)}"
if [ -s "$identifiers" ]; then
  rc=0
  grep -niFf <(tr -d '\r' < "$identifiers" | grep -v '^[[:space:]]*$') -- "${files[@]}" || rc=$?
  if [ "$rc" -eq 0 ]; then
    echo "check-docs: a personal identifier is in the repo, take it out" >&2
    status=1
  elif [ "$rc" -gt 1 ]; then
    echo "check-docs: the identifier scan itself failed" >&2
    exit 2
  fi
# the list only exists on the maintainer's machine, so CI and a clone of the public repo (no .agents/) skip it
elif [ "${CI:-}" = "true" ] || [ ! -d .agents ]; then
  echo "check-docs: no identifier list here, only the dash scan ran" >&2
else
  echo "check-docs: no identifier list at $identifiers. put the names to reject there, one per line, it never gets committed" >&2
  status=1
fi

exit $status
