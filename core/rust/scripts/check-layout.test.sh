#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
checker="$script_dir/check-layout.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
root="$tmp/rust"
baseline="$tmp/layout-baseline.txt"

mkdir -p \
  "$root/src/lang/data" \
  "$root/src/lang/protocol" \
  "$root/src/legacy" \
  "$root/src/hash"
touch \
  "$root/src/lang/data/vector.rs" \
  "$root/src/lang/protocol/iassoc.rs" \
  "$root/src/lang/protocol/ilookup.rs" \
  "$root/src/lang/protocol/ifind.rs"

write_lines() {
  local path="$1"
  local count="$2"
  awk -v count="$count" 'BEGIN { for (i = 1; i <= count; i++) print "// line " i }' > "$path"
}

run_check() {
  HARA_LAYOUT_ROOT="$root" \
  HARA_LAYOUT_BASELINE="$baseline" \
  HARA_LAYOUT_MAX_LINES=5 \
    bash "$checker"
}

write_lines "$root/src/legacy/large.rs" 7
write_lines "$root/src/hash/mod.rs" 1
cat > "$baseline" <<'BASELINE'
lines src/legacy/large.rs 7
modrs src/hash/mod.rs
BASELINE

run_check

printf '// growth\n' >> "$root/src/legacy/large.rs"
if run_check >"$tmp/growth.out" 2>&1; then
  echo "expected legacy growth to fail" >&2
  exit 1
fi
grep -Fq 'src/legacy/large.rs grew to 8 lines; recorded legacy maximum is 7' "$tmp/growth.out"
write_lines "$root/src/legacy/large.rs" 7

write_lines "$root/src/legacy/large.rs" 5
if run_check >"$tmp/resolved.out" 2>&1; then
  echo "expected resolved legacy debt to leave the baseline" >&2
  exit 1
fi
grep -Fq 'Resolved line baseline entry: src/legacy/large.rs now has 5 lines; remove it from the baseline' "$tmp/resolved.out"
write_lines "$root/src/legacy/large.rs" 7

write_lines "$root/src/new-large.rs" 6
if run_check >"$tmp/new.out" 2>&1; then
  echo "expected a new oversized module to fail" >&2
  exit 1
fi
grep -Fq 'src/new-large.rs has 6 lines; maximum is 5' "$tmp/new.out"
rm "$root/src/new-large.rs"

mkdir -p "$root/src/other"
write_lines "$root/src/other/mod.rs" 1
if run_check >"$tmp/modrs.out" 2>&1; then
  echo "expected an unrecorded mod.rs to fail" >&2
  exit 1
fi
grep -Fq 'unexpected legacy path: src/other/mod.rs' "$tmp/modrs.out"
rm -rf "$root/src/other"

cat >> "$baseline" <<'BASELINE'
lines src/removed.rs 9
BASELINE
if run_check >"$tmp/stale.out" 2>&1; then
  echo "expected a stale baseline entry to fail" >&2
  exit 1
fi
grep -Fq 'Stale line baseline entry: src/removed.rs' "$tmp/stale.out"

printf 'Rust layout baseline tests passed.\n'
