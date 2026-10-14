#!/usr/bin/env bash
# fails on em or en dashes and on personal identifiers in any file git would commit.
# the identifiers live outside the repo as sha256 hashes of each lowercased word, one per line, in .git/info/identifiers or in
# the file ANYBOX_IDENTIFIERS points to, so the names themselves are never committed or even written down
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
  # the list holds sha256 hashes of the lowercased words, never the names, so a name can't be read off the disk.
  # every word in every file gets hashed and looked up, a hit prints only the file and the line
  IDS="$identifiers" perl -MDigest::SHA=sha256_hex -ne '
    BEGIN {
      open my $h, "<", $ENV{IDS} or die "check-docs: cannot read $ENV{IDS}\n";
      while (<$h>) { s/\s+//g; next unless $_; die "check-docs: $ENV{IDS} must only hold sha256 hashes, one per line\n" unless /^[0-9a-f]{64}$/; $bad{$_} = 1 }
    }
    for my $word (/[A-Za-z0-9]+/g) { if ($bad{sha256_hex(lc $word)}) { print "$ARGV:$.\n"; $found = 1; last } }
    close ARGV if eof;
    END { $? = $found ? 0 : 1 if $? == 0 }
  ' -- "${files[@]}" || rc=$?
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
  echo "check-docs: no identifier list at $identifiers. put the sha256 hash of each lowercased word to reject there, one per line, it never gets committed" >&2
  status=1
fi

exit $status
