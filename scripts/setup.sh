#!/usr/bin/env bash
#
# VibeCody Developer Setup Script
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/scripts/setup.sh | bash
#   — or —
#   ./scripts/setup.sh
#
# Installs all prerequisites for building VibeCody from source on macOS, Linux, and WSL.
# Safe to re-run — skips anything already installed.

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

info()  { echo -e "${BLUE}[info]${NC}  $*"; }
ok()    { echo -e "${GREEN}[ok]${NC}    $*"; }
warn()  { echo -e "${YELLOW}[warn]${NC}  $*"; }
fail()  { echo -e "${RED}[error]${NC} $*"; exit 1; }

# ── Detect OS ─────────────────────────────────────────────────────────────────

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Darwin) PLATFORM="macos" ;;
  Linux)  PLATFORM="linux" ;;
  *)      fail "Unsupported OS: $OS. Use macOS, Linux, or WSL." ;;
esac

info "Detected: $PLATFORM ($ARCH)"

# ── Check / Install: Rust ─────────────────────────────────────────────────────

if command -v rustc &>/dev/null; then
  RUST_VER=$(rustc --version | awk '{print $2}')
  ok "Rust $RUST_VER already installed"
else
  info "Installing Rust via rustup..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
  source "$HOME/.cargo/env"
  ok "Rust $(rustc --version | awk '{print $2}') installed"
fi

# Ensure stable toolchain is default
rustup default stable 2>/dev/null || true

# ── Check / Install: Node.js ──────────────────────────────────────────────────

NODE_MIN=18

if command -v node &>/dev/null; then
  NODE_VER=$(node -v | sed 's/v//' | cut -d. -f1)
  if [ "$NODE_VER" -ge "$NODE_MIN" ]; then
    ok "Node.js v$(node -v | sed 's/v//') already installed"
  else
    warn "Node.js v$(node -v | sed 's/v//') is below minimum v${NODE_MIN}"
    info "Please upgrade: https://nodejs.org/ or use nvm/fnm"
  fi
else
  info "Node.js not found. Installing..."
  if [ "$PLATFORM" = "macos" ]; then
    if command -v brew &>/dev/null; then
      brew install node
    else
      warn "Install Node.js v${NODE_MIN}+ from https://nodejs.org/ or install Homebrew first"
    fi
  else
    # Linux — try fnm (fast node manager)
    if command -v fnm &>/dev/null; then
      fnm install 20
      fnm use 20
    elif command -v nvm &>/dev/null; then
      nvm install 20
      nvm use 20
    else
      info "Installing Node.js 20 via NodeSource..."
      curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
      sudo apt-get install -y nodejs
    fi
  fi
  ok "Node.js $(node -v 2>/dev/null || echo 'installed — restart shell') ready"
fi

# ── Platform-specific system dependencies ──────────────────────────────────────

if [ "$PLATFORM" = "macos" ]; then
  # Xcode command line tools
  if xcode-select -p &>/dev/null; then
    ok "Xcode CLI tools already installed"
  else
    info "Installing Xcode command line tools..."
    xcode-select --install 2>/dev/null || warn "Xcode CLI tools install dialog should appear. Accept it and re-run this script."
  fi

elif [ "$PLATFORM" = "linux" ]; then
  # Tauri build dependencies (Debian/Ubuntu)
  if command -v apt-get &>/dev/null; then
    TAURI_DEPS=(
      libwebkit2gtk-4.1-dev
      libgtk-3-dev
      libayatana-appindicator3-dev
      librsvg2-dev
      patchelf
      build-essential
      curl
      wget
      file
      libssl-dev
      pkg-config
    )
    MISSING=()
    for dep in "${TAURI_DEPS[@]}"; do
      if ! dpkg -s "$dep" &>/dev/null; then
        MISSING+=("$dep")
      fi
    done
    if [ ${#MISSING[@]} -gt 0 ]; then
      info "Installing Tauri system dependencies: ${MISSING[*]}"
      sudo apt-get update -q
      sudo apt-get install -y "${MISSING[@]}"
      ok "System dependencies installed"
    else
      ok "All Tauri system dependencies already installed"
    fi

  elif command -v dnf &>/dev/null; then
    # Fedora/RHEL
    TAURI_DEPS=(webkit2gtk4.1-devel gtk3-devel libappindicator-gtk3-devel librsvg2-devel patchelf openssl-devel)
    info "Installing Tauri system dependencies (Fedora)..."
    sudo dnf install -y "${TAURI_DEPS[@]}" 2>/dev/null || warn "Some packages may need manual install"
    ok "System dependencies installed"

  elif command -v pacman &>/dev/null; then
    # Arch
    TAURI_DEPS=(webkit2gtk-4.1 gtk3 libappindicator-gtk3 librsvg patchelf openssl base-devel)
    info "Installing Tauri system dependencies (Arch)..."
    sudo pacman -S --needed --noconfirm "${TAURI_DEPS[@]}" 2>/dev/null || warn "Some packages may need manual install"
    ok "System dependencies installed"

  else
    warn "Unsupported package manager. Install Tauri prerequisites manually:"
    warn "  https://v2.tauri.app/start/prerequisites/#linux"
  fi
fi

# ── Install npm dependencies ──────────────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." 2>/dev/null && pwd || pwd)"

if [ -f "$REPO_ROOT/vibeui/package.json" ]; then
  info "Installing VibeUI frontend dependencies..."
  (cd "$REPO_ROOT/vibeui" && npm install --no-audit --no-fund)
  ok "VibeUI npm dependencies installed"
fi

if [ -f "$REPO_ROOT/vibeapp/package.json" ]; then
  info "Installing VibeCLI App frontend dependencies..."
  (cd "$REPO_ROOT/vibeapp" && npm install --no-audit --no-fund)
  ok "VibeCLI App npm dependencies installed"
fi

# ── Verify everything ─────────────────────────────────────────────────────────

echo ""
echo -e "${GREEN}================================${NC}"
echo -e "${GREEN}  VibeCody Setup Complete!${NC}"
echo -e "${GREEN}================================${NC}"
echo ""
echo "  Rust:    $(rustc --version 2>/dev/null || echo 'not found')"
echo "  Cargo:   $(cargo --version 2>/dev/null || echo 'not found')"
echo "  Node.js: $(node --version 2>/dev/null || echo 'not found')"
echo "  npm:     $(npm --version 2>/dev/null || echo 'not found')"
echo ""
echo "Next steps:"
echo ""
echo "  # Build VibeCLI"
echo "  make cli"
echo ""
echo "  # Run VibeUI in dev mode"
echo "  make ui"
echo ""
echo "  # Run all tests"
echo "  make test"
echo ""
echo "  # See all available commands"
echo "  make help"
echo ""
