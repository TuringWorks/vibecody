#!/usr/bin/env sh
# VibeCLI installer — downloads the latest release binary for your platform.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/vibecody/vibecody/main/install.sh | sh
#
# Or with a specific version:
#   VERSION=v0.2.0 sh install.sh
#
# Override install directory:
#   INSTALL_DIR=/usr/local/bin sh install.sh

set -e

REPO="vibecody/vibecody"
BINARY="vibecli"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

# ── Detect OS and architecture ────────────────────────────────────────────────

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Darwin)
    case "$ARCH" in
      arm64|aarch64) ARCHIVE="vibecli-aarch64-apple-darwin.tar.gz" ;;
      x86_64)        ARCHIVE="vibecli-x86_64-apple-darwin.tar.gz"  ;;
      *) echo "Unsupported macOS architecture: $ARCH" >&2; exit 1 ;;
    esac
    ;;
  Linux)
    case "$ARCH" in
      x86_64|amd64) ARCHIVE="vibecli-x86_64-linux.tar.gz"   ;;
      aarch64|arm64) ARCHIVE="vibecli-aarch64-linux.tar.gz"  ;;
      *) echo "Unsupported Linux architecture: $ARCH" >&2; exit 1 ;;
    esac
    ;;
  *)
    echo "Unsupported OS: $OS" >&2
    echo "For Windows, download the .zip from GitHub Releases manually." >&2
    exit 1
    ;;
esac

# ── Resolve version ────────────────────────────────────────────────────────────

if [ -z "$VERSION" ]; then
  echo "Fetching latest release version..."
  VERSION="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": "\(.*\)".*/\1/')"
  if [ -z "$VERSION" ]; then
    echo "Could not determine latest version. Set VERSION=vX.Y.Z manually." >&2
    exit 1
  fi
fi

echo "Installing vibecli ${VERSION} for ${OS}/${ARCH}..."

# ── Download & extract ─────────────────────────────────────────────────────────

TMP="$(mktemp -d)"
URL="https://github.com/${REPO}/releases/download/${VERSION}/${ARCHIVE}"

echo "Downloading ${URL}..."
curl -fsSL --progress-bar "$URL" -o "${TMP}/${ARCHIVE}"

echo "Extracting..."
tar -xzf "${TMP}/${ARCHIVE}" -C "${TMP}"

# ── Install ────────────────────────────────────────────────────────────────────

mkdir -p "$INSTALL_DIR"
install -m 755 "${TMP}/${BINARY}" "${INSTALL_DIR}/${BINARY}"
rm -rf "$TMP"

echo ""
echo "  vibecli installed to ${INSTALL_DIR}/${BINARY}"

# Hint if INSTALL_DIR is not on PATH
case ":${PATH}:" in
  *":${INSTALL_DIR}:"*) ;;
  *)
    echo ""
    echo "  Add '${INSTALL_DIR}' to your PATH:"
    echo "    export PATH=\"${INSTALL_DIR}:\$PATH\""
    ;;
esac

echo ""
echo "  Run: vibecli --help"
echo "  Docs: https://vibecody.github.io/vibecody/"
echo ""
