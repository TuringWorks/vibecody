#!/usr/bin/env bash
set -euo pipefail
systemctl --user stop vibecody.service 2>/dev/null || true
systemctl --user disable vibecody.service 2>/dev/null || true
rm -f "$HOME/.config/systemd/user/vibecody.service"
rm -f "$HOME/.local/bin/vibecli"
echo "VibeCody uninstalled."
