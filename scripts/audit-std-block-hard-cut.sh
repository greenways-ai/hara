#!/usr/bin/env bash
set -euo pipefail

legacy_namespace='std.lib.''block'
legacy_path='std/lib/''block'
failed=0

if git grep -n -F "$legacy_namespace" -- .; then
  echo "Legacy block namespace remains in tracked content." >&2
  failed=1
fi

if git grep -n -F "$legacy_path" -- .; then
  echo "Legacy block source path remains in tracked content." >&2
  failed=1
fi

if git ls-files | grep -F "$legacy_path"; then
  echo "Legacy block source path remains in the repository tree." >&2
  failed=1
fi

if [[ -e core/lib/src/std/lib/block.hal ]] || [[ -e core/lib/src/std/lib/block ]]; then
  echo "Removed portable std.lib.block source paths still exist." >&2
  failed=1
fi

if [[ -e core/rust/hal-src/std/lib/block.hal ]] || [[ -e core/rust/hal-src/std/lib/block ]]; then
  echo "Removed Rust HAL std.lib.block mirror paths still exist." >&2
  failed=1
fi

if [[ ! -f core/lib/src/std/block.hal ]] || [[ ! -d core/lib/src/std/block ]]; then
  echo "Canonical portable std.block sources are incomplete." >&2
  failed=1
fi

if [[ ! -f core/rust/hal-src/std/block.hal ]] || [[ ! -d core/rust/hal-src/std/block ]]; then
  echo "Canonical Rust HAL std.block mirror is incomplete." >&2
  failed=1
fi

if [[ -f core/lib/src/std/block.hal ]] && [[ -f core/rust/hal-src/std/block.hal ]] \
   && ! cmp -s core/lib/src/std/block.hal core/rust/hal-src/std/block.hal; then
  echo "Portable and Rust HAL std.block roots differ." >&2
  failed=1
fi

if [[ -d core/lib/src/std/block ]] && [[ -d core/rust/hal-src/std/block ]] \
   && ! diff -qr core/lib/src/std/block core/rust/hal-src/std/block; then
  echo "Portable and Rust HAL std.block namespace trees differ." >&2
  failed=1
fi

if [[ -e core/java/src/test/java/hara/truffle/StdLibBlockTest.java ]]; then
  echo "Legacy JVM std.lib.block test class remains." >&2
  failed=1
fi

exit "$failed"
