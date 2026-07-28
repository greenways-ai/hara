#!/bin/sh
# Functional tests for scripts/install.sh.
#
# Builds a fake "release" (tarball + SHA256SUMS) from the locally compiled
# release binary, serves it over file://, and exercises the installer.
#
# Prereq: cargo build --release --manifest-path rust/Cargo.toml --bin hara
# Usage:  sh scripts/test-install.sh
set -eu

REPO_ROOT=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
INSTALL_SH="$REPO_ROOT/scripts/install.sh"
WORK="$REPO_ROOT/.tmp/install-test"
case "$(uname -s)/$(uname -m)" in
  Linux/x86_64) TRIPLE="x86_64-unknown-linux-gnu" ;;
  Darwin/arm64) TRIPLE="aarch64-apple-darwin" ;;
  Darwin/x86_64) TRIPLE="x86_64-apple-darwin" ;;
  *) echo "unsupported test platform: $(uname -s)/$(uname -m)" >&2; exit 1 ;;
esac
VERSION="v9.9.9"
TARBALL="hara-rust-$VERSION-$TRIPLE.tar.gz"
TRUFFLE_TARBALL="hara-truffle-$VERSION-$TRIPLE.tar.gz"
BINARY="$REPO_ROOT/rust/target/release/hara"

pass=0
fail=0

ok() { pass=$((pass + 1)); printf 'ok %s - %s\n' "$pass" "$1"; }
not_ok() { fail=$((fail + 1)); printf 'not ok - %s\n' "$1"; }
check() { # check <description> <command...>
  desc=$1; shift
  if "$@" >/dev/null 2>&1; then ok "$desc"; else not_ok "$desc"; fi
}

if [ ! -x "$BINARY" ]; then
  echo "missing release binary at $BINARY" >&2
  echo "run: cargo build --release --manifest-path rust/Cargo.toml --bin hara" >&2
  exit 1
fi

# --- fake release fixture ---------------------------------------------------
rm -rf "$WORK"
mkdir -p "$WORK/release"
tar -czf "$WORK/release/$TARBALL" -C "$(dirname "$BINARY")" hara
mkdir -p "$WORK/truffle"
cp "$BINARY" "$WORK/truffle/hara-truffle"
tar -czf "$WORK/release/$TRUFFLE_TARBALL" -C "$WORK/truffle" hara-truffle
(cd "$WORK/release" && sha256sum "$TARBALL" "$TRUFFLE_TARBALL" > SHA256SUMS)

run_installer() { # run_installer <extra-env...>; stdout+stderr captured
  env -i PATH="$TEST_PATH" HOME="$TEST_HOME" \
    HARA_VERSION="$VERSION" \
    HARA_TARGET_TRIPLE="$TRIPLE" \
    HARA_RELEASE_BASE_URL="file://$WORK/release" \
    "$@" sh "$INSTALL_SH" --rust 2>&1
}

run_truffle_installer() { # run_truffle_installer <extra-env...>; stdout+stderr captured
  env -i PATH="$TEST_PATH" HOME="$TEST_HOME" \
    HARA_VERSION="$VERSION" \
    HARA_TARGET_TRIPLE="$TRIPLE" \
    HARA_RELEASE_BASE_URL="file://$WORK/release" \
    "$@" sh "$INSTALL_SH" --truffle 2>&1
}

TEST_HOME="$WORK/home"
mkdir -p "$TEST_HOME"
# Deliberately exclude any install dir from PATH so the hint is exercised.
TEST_PATH="/usr/bin:/bin"

# --- 1. happy path ----------------------------------------------------------
OUT=$({ run_installer HARA_INSTALL_DIR="$WORK/bin"; } || echo "EXIT:$?")
check "installer exits 0 on happy path" test -x "$WORK/bin/hara"
RESULT=$("$WORK/bin/hara" eval '(+ 19 23)' 2>/dev/null || true)
check "installed binary evaluates (+ 19 23) => 42" test "$RESULT" = "42"
case "$OUT" in
  *"export PATH="*) ok "prints PATH hint when install dir not on PATH" ;;
  *) not_ok "prints PATH hint when install dir not on PATH" ;;
esac
case "$OUT" in
  *EXIT:*) not_ok "installer exit status was 0" ;;
  *) ok "installer exit status was 0" ;;
esac

# --- 2. native-image install ------------------------------------------------
OUT=$({ run_truffle_installer HARA_INSTALL_DIR="$WORK/truffle-bin"; } || echo "EXIT:$?")
check "Truffle installer installs hara-truffle" test -x "$WORK/truffle-bin/hara-truffle"
RESULT=$("$WORK/truffle-bin/hara-truffle" eval '(+ 19 23)' 2>/dev/null || true)
check "installed hara-truffle evaluates (+ 19 23) => 42" test "$RESULT" = "42"
case "$OUT" in
  *EXIT:*) not_ok "Truffle installer exit status was 0" ;;
  *) ok "Truffle installer exit status was 0" ;;
esac

# --- 3. default install dir under HOME --------------------------------------
OUT=$({ run_installer; } || echo "EXIT:$?")
check "default install dir is \$HOME/.local/bin" test -x "$TEST_HOME/.local/bin/hara"

# --- 4. checksum mismatch aborts --------------------------------------------
cp "$WORK/release/SHA256SUMS" "$WORK/release/SHA256SUMS.good"
sed 's/^./X/' "$WORK/release/SHA256SUMS" > "$WORK/release/SHA256SUMS.bad"
mv "$WORK/release/SHA256SUMS.bad" "$WORK/release/SHA256SUMS"
OUT=$({ run_installer HARA_INSTALL_DIR="$WORK/bin-badsum"; } && echo "EXIT:0" || echo "EXIT:$?")
mv "$WORK/release/SHA256SUMS.good" "$WORK/release/SHA256SUMS"
case "$OUT" in
  *checksum*|*Checksum*|*CHECKSUM*) ok "checksum mismatch reported" ;;
  *) not_ok "checksum mismatch reported" ;;
esac
case "$OUT" in
  *EXIT:0*) not_ok "checksum mismatch aborts with nonzero exit" ;;
  *) ok "checksum mismatch aborts with nonzero exit" ;;
esac
check "checksum mismatch leaves no binary behind" test ! -e "$WORK/bin-badsum/hara"

# --- 5. unsupported platform ------------------------------------------------
OUT=$({ run_installer HARA_INSTALL_DIR="$WORK/bin-win" HARA_TARGET_TRIPLE="x86_64-pc-windows-msvc"; } \
      && echo "EXIT:0" || echo "EXIT:$?")
case "$OUT" in
  *"not supported"*) ok "unsupported platform reported" ;;
  *) not_ok "unsupported platform reported" ;;
esac
case "$OUT" in
  *EXIT:0*) not_ok "unsupported platform exits nonzero" ;;
  *) ok "unsupported platform exits nonzero" ;;
esac
check "unsupported platform installs nothing" test ! -e "$WORK/bin-win/hara"

# --- 6. overwrite existing install ------------------------------------------
OUT=$({ run_installer HARA_INSTALL_DIR="$WORK/bin"; } || echo "EXIT:$?")
case "$OUT" in
  *[Ee]xisting*|*verwrit*) ok "reinstall over existing binary is reported" ;;
  *) not_ok "reinstall over existing binary is reported" ;;
esac
check "reinstall keeps a working binary" test -x "$WORK/bin/hara"

# --- summary ----------------------------------------------------------------
echo "---"
echo "$((pass + fail)) tests, $pass passed, $fail failed"
[ "$fail" -eq 0 ]
