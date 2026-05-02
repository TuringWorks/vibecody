#!/usr/bin/env bash
# Slice 7b — Multi-Agent PPO on PettingZoo MPE simple_spread_v3.
#
# Requires `--extra marl` for the multi-agent toolkit (PettingZoo, mpe2).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
SIDECAR_DIR="$REPO_ROOT/vibe-rl-py"
WORKSPACE="$SCRIPT_DIR/.workspace"
RUN_ID="mappo-001"

mkdir -p "$WORKSPACE/.vibecli/rl-artifacts/$RUN_ID"

RUN_CFG="$WORKSPACE/.vibecli/rl-artifacts/$RUN_ID/config.yaml"
{
  cat "$SCRIPT_DIR/config.yaml"
  printf 'workspace_path: %s\nartifact_dir: %s\n' \
    "$WORKSPACE" \
    "$WORKSPACE/.vibecli/rl-artifacts/$RUN_ID"
} > "$RUN_CFG"

echo "==> repo root:   $REPO_ROOT"
echo "==> workspace:   $WORKSPACE"
echo "==> run id:      $RUN_ID"
echo

cd "$SIDECAR_DIR"
exec uv run python -m vibe_rl train --run-id "$RUN_ID" --config "$RUN_CFG"
