#!/usr/bin/env bash
set -euo pipefail
GREEN='\033[0;32m'; RED='\033[0;31m'; BLUE='\033[0;34m'; YELLOW='\033[1;33m'; NC='\033[0m'
HEADLESS=false; TUNNEL=""
while [[ $# -gt 0 ]]; do
  case "$1" in --headless) HEADLESS=true; shift ;; --cloudflare) TUNNEL="cloudflare"; shift ;;
    --tailscale) TUNNEL="tailscale"; shift ;; -h|--help)
    echo "Usage: $0 [--headless] [--cloudflare|--tailscale]"; exit 0 ;; *) shift ;; esac
done

printf "${BLUE}VibeCody Raspberry Pi Setup${NC}\n\n"

# Detect Pi model
PI_MODEL="Unknown"
if [[ -f /proc/device-tree/model ]]; then
  PI_MODEL=$(tr -d '\0' < /proc/device-tree/model)
fi
printf "${GREEN}[OK]${NC} Model: ${PI_MODEL}\n"

# Detect RAM
RAM_KB=$(grep MemTotal /proc/meminfo | awk '{print $2}')
RAM_GB=$(echo "$RAM_KB" | awk '{printf "%.1f", $1/1048576}')
RAM_MB=$((RAM_KB / 1024))
printf "${GREEN}[OK]${NC} RAM: ${RAM_GB} GB\n"

# Recommend model based on RAM
if (( RAM_MB < 2048 )); then
  MODEL="tinyllama:1.1b"; CONFIG="config-pi3.toml"
  printf "${YELLOW}[WARN]${NC} Low RAM — recommending TinyLlama (1.1B) or use a cloud provider\n"
elif (( RAM_MB < 6144 )); then
  MODEL="phi:2.7b"; CONFIG="config-pi4.toml"
elif (( RAM_MB < 10240 )); then
  MODEL="mistral:7b"; CONFIG="config-pi4.toml"
else
  MODEL="codellama:7b"; CONFIG="config-pi5.toml"
fi
printf "${BLUE}[INFO]${NC} Recommended model: ${MODEL}\n"

# Add swap if low RAM
if (( RAM_MB < 4096 )); then
  if [[ ! -f /swapfile ]]; then
    printf "${BLUE}[INFO]${NC} Creating 2GB swap for model loading...\n"
    sudo fallocate -l 2G /swapfile
    sudo chmod 600 /swapfile
    sudo mkswap /swapfile
    sudo swapon /swapfile
    echo '/swapfile none swap sw 0 0' | sudo tee -a /etc/fstab > /dev/null
    printf "${GREEN}[OK]${NC}   Swap enabled\n"
  fi
fi

# Install deps
sudo apt-get update -qq && sudo apt-get install -y -qq curl

# Install vibecli
printf "${BLUE}[INFO]${NC} Installing VibeCLI (aarch64)...\n"
curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/install.sh | sh
printf "${GREEN}[OK]${NC}   VibeCLI installed\n"

# Install Ollama
if ! command -v ollama &>/dev/null; then
  printf "${BLUE}[INFO]${NC} Installing Ollama...\n"
  curl -fsSL https://ollama.com/install.sh | sh
  printf "${GREEN}[OK]${NC}   Ollama installed\n"
fi

# Copy optimized config
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
mkdir -p "$HOME/.vibecli"
if [[ -f "$SCRIPT_DIR/$CONFIG" ]]; then
  cp "$SCRIPT_DIR/$CONFIG" "$HOME/.vibecli/config.toml"
  printf "${GREEN}[OK]${NC}   Optimized config installed ($CONFIG)\n"
fi

# Pull recommended model
printf "${YELLOW}[?]${NC}    Pull $MODEL now? [Y/n] "
read -r ans
if [[ ! "$ans" =~ ^[Nn] ]]; then
  ollama pull "$MODEL"
fi

# Install systemd service
printf "${BLUE}[INFO]${NC} Installing systemd service...\n"
sudo cp "$SCRIPT_DIR/vibecody-pi.service" /etc/systemd/system/vibecody.service
sudo sed -i "s|%VIBECLI_BIN%|$HOME/.local/bin/vibecli|g" /etc/systemd/system/vibecody.service
sudo sed -i "s|%USER%|$(whoami)|g" /etc/systemd/system/vibecody.service
sudo systemctl daemon-reload
sudo systemctl enable --now vibecody.service
printf "${GREEN}[OK]${NC}   Service enabled at http://localhost:7878\n"

# Remote access tunnel
if [[ "$TUNNEL" == "cloudflare" ]]; then
  printf "${BLUE}[INFO]${NC} Setting up Cloudflare Tunnel...\n"
  curl -fsSL https://pkg.cloudflare.com/cloudflared-linux-arm64.deb -o /tmp/cloudflared.deb
  sudo dpkg -i /tmp/cloudflared.deb
  echo "Run: cloudflared tunnel login && cloudflared tunnel create vibecody"
elif [[ "$TUNNEL" == "tailscale" ]]; then
  printf "${BLUE}[INFO]${NC} Setting up Tailscale...\n"
  curl -fsSL https://tailscale.com/install.sh | sh
  echo "Run: sudo tailscale up && tailscale serve --bg 7878"
fi

printf "\n${GREEN}Raspberry Pi setup complete!${NC}\n"
echo "  VibeCody: http://localhost:7878"
echo "  Model: $MODEL"
echo "  Service: sudo systemctl status vibecody"
echo "  Docs: https://vibecody.github.io/vibecody/guides/raspberry-pi/"
