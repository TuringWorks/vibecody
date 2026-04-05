#!/usr/bin/env bash
set -euo pipefail
PLIST="$HOME/Library/LaunchAgents/com.vibecody.vibecli.plist"
launchctl unload "$PLIST" 2>/dev/null || true
rm -f "$PLIST"
rm -f "$HOME/.local/bin/vibecli"
echo "VibeCody uninstalled."
