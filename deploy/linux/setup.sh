#!/usr/bin/env bash
set -euo pipefail
GREEN='\033[0;32m'; RED='\033[0;31m'; BLUE='\033[0;34m'; YELLOW='\033[1;33m'; NC='\033[0m'
TIER="lite"; ALWAYS_ON=false
while [[ $# -gt 0 ]]; do
  case "$1" in --tier) TIER="$2"; shift 2 ;; --always-on) ALWAYS_ON=true; shift ;; -h|--help)
    echo "Usage: $0 [--tier lite|pro|max] [--always-on]"; exit 0 ;; *) shift ;; esac
done

printf "${BLUE}VibeCody Linux Setup${NC}\n\n"

# Detect distro and install deps
if command -v apt-get &>/dev/null; then
  printf "${BLUE}[INFO]${NC} Detected Debian/Ubuntu\n"
  sudo apt-get update -qq && sudo apt-get install -y -qq curl pkg-config libssl-dev
elif command -v dnf &>/dev/null; then
  printf "${BLUE}[INFO]${NC} Detected Fedora/RHEL\n"
  sudo dnf install -y -q curl openssl-devel
elif command -v pacman &>/dev/null; then
  printf "${BLUE}[INFO]${NC} Detected Arch\n"
  sudo pacman -Sy --noconfirm curl openssl
fi

# Install vibecli
printf "${BLUE}[INFO]${NC} Installing VibeCLI...\n"
curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/install.sh | sh
printf "${GREEN}[OK]${NC}   VibeCLI installed\n"

# Ollama
if ! command -v ollama &>/dev/null; then
  printf "${YELLOW}[?]${NC}    Install Ollama for local models? [Y/n] "
  read -r ans
  if [[ ! "$ans" =~ ^[Nn] ]]; then
    curl -fsSL https://ollama.com/install.sh | sh
    printf "${GREEN}[OK]${NC}   Ollama installed\n"
  fi
fi

# Always-on service
if $ALWAYS_ON; then
  SVCDIR="$HOME/.config/systemd/user"
  mkdir -p "$SVCDIR"
  cp "$(dirname "$0")/vibecody.service" "$SVCDIR/"
  systemctl --user daemon-reload
  systemctl --user enable --now vibecody.service
  printf "${GREEN}[OK]${NC}   Always-on service enabled at http://localhost:7878\n"
fi

printf "\n${GREEN}Setup complete!${NC}\n"
echo "  Run: vibecli"
echo "  Docs: https://vibecody.github.io/vibecody/guides/linux/"
