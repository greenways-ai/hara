#!/bin/sh
# install.sh — install Hara runtimes from GitHub Release packages.
#
#   curl -fsSL https://www.hara-lang.org/install.sh | sh -- --rust --truffle
#
# Environment overrides:
#   HARA_VERSION          release tag to install (default: latest release)
#   HARA_INSTALL_DIR      install location (default: ~/.local/bin)
#   HARA_RELEASE_BASE_URL  base URL containing the release assets
#                         (default: https://github.com/hara-lang/hara/releases/download/$HARA_VERSION)
#   HARA_TARGET_TRIPLE    override platform detection (for testing)
#
# --rust installs hara for Linux x86_64, macOS arm64, and macOS x86_64.
# --truffle installs the hara-truffle native image.
# At least one runtime flag is required.
set -eu

REPO="hara-lang/hara"

info() { printf '%s\n' "$*"; }
die() { printf 'error: %s\n' "$*" >&2; exit 1; }

need() {
  command -v "$1" >/dev/null 2>&1 || die "required tool not found: $1"
}

INSTALL_RUST=false
INSTALL_TRUFFLE=false
for arg in "$@"; do
  case "$arg" in
    --rust) INSTALL_RUST=true ;;
    --truffle) INSTALL_TRUFFLE=true ;;
    -h|--help)
      cat <<'EOF'
Usage: install.sh [--rust] [--truffle]

Install one or both Hara runtimes from GitHub Releases.
EOF
      exit 0
      ;;
    *) die "unknown option: $arg (use --rust and/or --truffle)" ;;
  esac
done
[ "$INSTALL_RUST" = true ] || [ "$INSTALL_TRUFFLE" = true ] \
  || die "choose at least one runtime: --rust and/or --truffle"

# --- platform detection -----------------------------------------------------
detect_triple() {
  os=$(uname -s)
  arch=$(uname -m)
  case "$os" in
    Linux)
      case "$arch" in
        x86_64) printf 'x86_64-unknown-linux-gnu' ;;
        *) return 1 ;;
      esac
      ;;
    Darwin)
      case "$arch" in
        arm64) printf 'aarch64-apple-darwin' ;;
        x86_64) printf 'x86_64-apple-darwin' ;;
        *) return 1 ;;
      esac
      ;;
    *) return 1 ;;
  esac
}

if [ "$INSTALL_RUST" = true ] || [ "$INSTALL_TRUFFLE" = true ]; then
  if [ "${HARA_TARGET_TRIPLE:-}" ]; then
    TRIPLE=$HARA_TARGET_TRIPLE
  else
    TRIPLE=$(detect_triple) || TRIPLE=""
  fi
  case "$TRIPLE" in
    x86_64-unknown-linux-gnu|aarch64-apple-darwin|x86_64-apple-darwin) ;;
    *)
      die "native runtime is not supported on ${HARA_TARGET_TRIPLE:-$(uname -s)/$(uname -m)}.
Build from source instead: cargo build --release --manifest-path core/rust/Cargo.toml --bin hara"
      ;;
  esac
fi

# --- download helpers -------------------------------------------------------
if command -v curl >/dev/null 2>&1; then
  fetch() { curl -fsSL "$1"; }
  fetch_to() { curl -fsSL "$1" -o "$2"; }
elif command -v wget >/dev/null 2>&1; then
  fetch() { wget -qO- "$1"; }
  fetch_to() { wget -qO "$2" "$1"; }
else
  die "neither curl nor wget found; install one and retry"
fi

# --- version resolution -----------------------------------------------------
if [ "${HARA_VERSION:-}" ]; then
  VERSION=$HARA_VERSION
else
  info "resolving latest release..."
  # /releases/latest excludes prereleases; the list endpoint returns the
  # newest release first, including prereleases.
  VERSION=$(fetch "https://api.github.com/repos/$REPO/releases?per_page=1" \
    | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
    | head -n 1)
  [ -n "$VERSION" ] || die "could not resolve latest release; set HARA_VERSION explicitly"
fi

BASE_URL=${HARA_RELEASE_BASE_URL:-"https://github.com/$REPO/releases/download/$VERSION"}
INSTALL_DIR=${HARA_INSTALL_DIR:-"$HOME/.local/bin"}

# --- download + verify ------------------------------------------------------
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT INT TERM

fetch_to "$BASE_URL/SHA256SUMS" "$TMP/SHA256SUMS" \
  || die "download failed: $BASE_URL/SHA256SUMS"

if command -v sha256sum >/dev/null 2>&1; then
  verify() {
    CHECKSUM_ENTRY="$TMP/checksum-entry"
    grep " $1\$" "$TMP/SHA256SUMS" > "$CHECKSUM_ENTRY" || return 1
    (cd "$TMP" && sha256sum --check --status "$(basename "$CHECKSUM_ENTRY")")
  }
elif command -v shasum >/dev/null 2>&1; then
  verify() {
    CHECKSUM_ENTRY="$TMP/checksum-entry"
    grep " $1\$" "$TMP/SHA256SUMS" > "$CHECKSUM_ENTRY" || return 1
    (cd "$TMP" && shasum -a 256 --check --status "$(basename "$CHECKSUM_ENTRY")")
  }
else
  die "neither sha256sum nor shasum found; cannot verify checksum"
fi
mkdir -p "$INSTALL_DIR"

install_rust() {
  TARBALL="hara-rust-$VERSION-$TRIPLE.tar.gz"
  info "downloading $TARBALL ($TRIPLE)..."
  fetch_to "$BASE_URL/$TARBALL" "$TMP/$TARBALL" \
    || die "download failed: $BASE_URL/$TARBALL"
  verify "$TARBALL" || die "checksum mismatch for $TARBALL; aborting (file not installed)"
  tar -xzf "$TMP/$TARBALL" -C "$TMP" || die "failed to extract $TARBALL"
  [ -f "$TMP/hara" ] || die "archive did not contain a hara binary"
  DEST="$INSTALL_DIR/hara"
  [ ! -e "$DEST" ] || info "Existing installation found at $DEST, overwriting"
  cp "$TMP/hara" "$DEST"
  chmod 755 "$DEST"
  info "installed Rust runtime: $("$DEST" --version 2>/dev/null || echo "hara $VERSION")"
  info "location:  $DEST"
}

install_truffle() {
  TARBALL="hara-truffle-$VERSION-$TRIPLE.tar.gz"
  info "downloading $TARBALL ($TRIPLE)..."
  fetch_to "$BASE_URL/$TARBALL" "$TMP/$TARBALL" \
    || die "download failed: $BASE_URL/$TARBALL"
  verify "$TARBALL" || die "checksum mismatch for $TARBALL; aborting (file not installed)"
  tar -xzf "$TMP/$TARBALL" -C "$TMP" || die "failed to extract $TARBALL"
  [ -f "$TMP/hara-truffle" ] || die "archive did not contain a hara-truffle binary"
  DEST="$INSTALL_DIR/hara-truffle"
  [ ! -e "$DEST" ] || info "Existing installation found at $DEST, overwriting"
  cp "$TMP/hara-truffle" "$DEST"
  chmod 755 "$DEST"
  info "installed Truffle native image: $DEST"
}

[ "$INSTALL_RUST" = true ] && install_rust
[ "$INSTALL_TRUFFLE" = true ] && install_truffle

# --- PATH hint ---------------------------------------------------------------
case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *)
    info ""
    info "NOTE: $INSTALL_DIR is not on your PATH. Add it with:"
    info ""
    info "  export PATH=\"$INSTALL_DIR:\$PATH\""
    info ""
    info "(add that line to your ~/.profile, ~/.bashrc, or ~/.zshrc to make it permanent)"
    ;;
esac
