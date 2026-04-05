#!/usr/bin/env bash
set -euo pipefail
GREEN='\033[0;32m'; BLUE='\033[0;34m'; YELLOW='\033[1;33m'; NC='\033[0m'
ALWAYS_ON=false
while [[ $# -gt 0 ]]; do
  case "$1" in --always-on) ALWAYS_ON=true; shift ;; -h|--help)
    echo "Usage: $0 [--always-on]"; exit 0 ;; *) shift ;; esac
done

printf "${BLUE}VibeCody macOS Setup${NC}\n\n"

# Detect hardware
ARCH=$(uname -m)
if [[ "$ARCH" == "arm64" ]]; then
  printf "${GREEN}[OK]${NC} Apple Silicon detected (Metal GPU acceleration available)\n"
else
  printf "${GREEN}[OK]${NC} Intel Mac detected\n"
fi
RAM=$(sysctl -n hw.memsize | awk '{printf "%.0f", $1/1073741824}')
printf "${GREEN}[OK]${NC} ${RAM} GB RAM\n"

# Install vibecli
printf "${BLUE}[INFO]${NC} Installing VibeCLI...\n"
curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/install.sh | sh
printf "${GREEN}[OK]${NC}   VibeCLI installed\n"

# Ollama
if ! command -v ollama &>/dev/null; then
  printf "${YELLOW}[?]${NC}    Install Ollama? [Y/n] "
  read -r ans
  if [[ ! "$ans" =~ ^[Nn] ]]; then
    if command -v brew &>/dev/null; then
      brew install ollama
    else
      curl -fsSL https://ollama.com/install.sh | sh
    fi
    printf "${GREEN}[OK]${NC}   Ollama installed\n"
  fi
fi

# Model recommendation
if (( RAM >= 32 )); then MODEL="codellama:13b"
elif (( RAM >= 16 )); then MODEL="codellama:7b"
else MODEL="qwen3-coder:480b-cloud"; fi
printf "${BLUE}[INFO]${NC} Recommended model: ${MODEL}\n"

# Always-on (launchd)
if $ALWAYS_ON; then
  PLIST_DIR="$HOME/Library/LaunchAgents"
  mkdir -p "$PLIST_DIR"
  cp "$(dirname "$0")/com.vibecody.vibecli.plist" "$PLIST_DIR/"
  # Update binary path in plist
  sed -i '' "s|VIBECLI_BIN|$HOME/.local/bin/vibecli|g" "$PLIST_DIR/com.vibecody.vibecli.plist"
  launchctl load "$PLIST_DIR/com.vibecody.vibecli.plist"
  printf "${GREEN}[OK]${NC}   Always-on service loaded at http://localhost:7878\n"

  # Mac Mini headless note
  if system_profiler SPHardwareDataType 2>/dev/null | grep -q "Mac mini\|Mac Studio"; then
    printf "${BLUE}[TIP]${NC}  Mac Mini/Studio detected — ideal as a headless VibeCody server!\n"
    echo "         Access remotely via: vibecli --serve --tailscale"
  fi
fi

printf "\n${GREEN}Setup complete!${NC}\n"
echo "  Run: vibecli"
echo "  Docs: https://vibecody.github.io/vibecody/guides/macos/"
