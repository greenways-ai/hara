#!/usr/bin/env bash
set -euo pipefail

legacy_namespace='std.lib.''block'
legacy_path='std/lib/''block'
portable_legacy_root="core/lib/src/${legacy_path}"
rust_legacy_root="core/rust/hal-src/${legacy_path}"
legacy_jvm_test='core/java/src/test/java/hara/truffle/StdLib''BlockTest.java'
failed=0

if git grep -n -F "$legacy_namespace" -- .; then
  echo "A retired block namespace remains in tracked content." >&2
  failed=1
fi

if git grep -n -F "$legacy_path" -- .; then
  echo "A retired block source path remains in tracked content." >&2
  failed=1
fi

if git ls-files | grep -F "$legacy_path"; then
  echo "A retired block source path remains in the repository tree." >&2
  failed=1
fi

if [[ -e "${portable_legacy_root}.hal" ]] || [[ -e "$portable_legacy_root" ]]; then
  echo "Removed portable block source paths still exist." >&2
  failed=1
fi

if [[ -e "${rust_legacy_root}.hal" ]] || [[ -e "$rust_legacy_root" ]]; then
  echo "Removed Rust HAL block mirror paths still exist." >&2
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

if ! cmp -s core/lib/src/std/block.hal core/rust/hal-src/std/block.hal; then
  echo "Portable and Rust HAL std.block roots differ." >&2
  failed=1
fi

if ! diff -qr core/lib/src/std/block core/rust/hal-src/std/block; then
  echo "Portable and Rust HAL std.block namespace trees differ." >&2
  failed=1
fi

if [[ -e "$legacy_jvm_test" ]]; then
  echo "A retired JVM block test class remains." >&2
  failed=1
fi

exit "$failed"
