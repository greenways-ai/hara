#!/bin/sh
# install.sh — install the hara native CLI from prebuilt GitHub releases.
#
#   curl -fsSL https://raw.githubusercontent.com/hoebat/hara.lang/main/scripts/install.sh | sh
#
# Environment overrides:
#   HARA_VERSION          release tag to install (default: latest release)
#   HARA_INSTALL_DIR      install location (default: ~/.local/bin)
#   HARA_RELEASE_BASE_URL  base URL containing the release assets
#                         (default: https://github.com/hoebat/hara.lang/releases/download/$HARA_VERSION)
#   HARA_TARGET_TRIPLE    override platform detection (for testing)
#
# Platforms: Linux x86_64, macOS arm64, macOS x86_64. Anything else: build
# from source — cargo build --release --manifest-path rust/Cargo.toml --bin hara
set -eu

REPO="hoebat/hara.lang"

info() { printf '%s\n' "$*"; }
die() { printf 'error: %s\n' "$*" >&2; exit 1; }

need() {
  command -v "$1" >/dev/null 2>&1 || die "required tool not found: $1"
}

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

if [ "${HARA_TARGET_TRIPLE:-}" ]; then
  TRIPLE=$HARA_TARGET_TRIPLE
else
  TRIPLE=$(detect_triple) || TRIPLE=""
fi
case "$TRIPLE" in
  x86_64-unknown-linux-gnu|aarch64-apple-darwin|x86_64-apple-darwin) ;;
  *)
    die "platform not supported yet (${HARA_TARGET_TRIPLE:-$(uname -s)/$(uname -m)}).
Build from source instead: cargo build --release --manifest-path rust/Cargo.toml --bin hara"
    ;;
esac

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
TARBALL="hara-$VERSION-$TRIPLE.tar.gz"

INSTALL_DIR=${HARA_INSTALL_DIR:-"$HOME/.local/bin"}

# --- download + verify ------------------------------------------------------
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT INT TERM

info "downloading $TARBALL ($TRIPLE)..."
fetch_to "$BASE_URL/$TARBALL" "$TMP/$TARBALL" \
  || die "download failed: $BASE_URL/$TARBALL"
fetch_to "$BASE_URL/SHA256SUMS" "$TMP/SHA256SUMS" \
  || die "download failed: $BASE_URL/SHA256SUMS"

if command -v sha256sum >/dev/null 2>&1; then
  verify() { (cd "$TMP" && sha256sum --check --status SHA256SUMS); }
elif command -v shasum >/dev/null 2>&1; then
  verify() { (cd "$TMP" && shasum -a 256 --check --status SHA256SUMS); }
else
  die "neither sha256sum nor shasum found; cannot verify checksum"
fi
if ! verify; then
  die "checksum mismatch for $TARBALL; aborting (file not installed)"
fi

# --- install ----------------------------------------------------------------
tar -xzf "$TMP/$TARBALL" -C "$TMP" || die "failed to extract $TARBALL"
[ -f "$TMP/hara" ] || die "archive did not contain a hara binary"

mkdir -p "$INSTALL_DIR"
DEST="$INSTALL_DIR/hara"
if [ -e "$DEST" ]; then
  info "Existing installation found at $DEST, overwriting"
fi
cp "$TMP/hara" "$DEST"
chmod 755 "$DEST"

info "installed: $("$DEST" --version 2>/dev/null || echo "hara $VERSION")"
info "location:  $DEST"

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
