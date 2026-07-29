#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
maximum_lines=700
failed=0

while IFS= read -r source; do
  relative="${source#"$root/"}"
  case "$relative" in
    src/core.rs|src/lib.rs|src/fiber.rs|src/bin/hara/repl.rs)
      # Compatibility facades while the Java port is split into modules.
      continue
      ;;
  esac

  lines="$(wc -l < "$source")"
  if (( lines > maximum_lines )); then
    echo "$relative has $lines lines; maximum is $maximum_lines" >&2
    failed=1
  fi
done < <(find "$root/src" -type f -name '*.rs' -print | sort)

if find "$root/src" -type f -name mod.rs -print -quit | grep -q .; then
  echo "Use module.rs plus module/*.rs; mod.rs files are not allowed" >&2
  failed=1
fi

required=(
  src/lang/data/vector.rs
  src/lang/protocol/iassoc.rs
  src/lang/protocol/ilookup.rs
  src/lang/protocol/ifind.rs
)

for relative in "${required[@]}"; do
  if [[ ! -f "$root/$relative" ]]; then
    echo "Missing required dedicated module: $relative" >&2
    failed=1
  fi
done

exit "$failed"
