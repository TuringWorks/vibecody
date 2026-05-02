#!/usr/bin/env bash
# Slice 2 — PPO baseline on CartPole-v1.
#
# Self-contained: locates the repo from its own path and points the
# sidecar at .workspace/ inside this example directory.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
SIDECAR_DIR="$REPO_ROOT/vibe-rl-py"
WORKSPACE="$SCRIPT_DIR/.workspace"
RUN_ID="cartpole-baseline-001"

mkdir -p "$WORKSPACE/.vibecli/rl-artifacts/$RUN_ID"

# Materialize the per-run config the sidecar consumes — this matches
# what the daemon's executor would write at run-start time.
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
echo "==> config:      $RUN_CFG"
echo

cd "$SIDECAR_DIR"
exec uv run python -m vibe_rl train --run-id "$RUN_ID" --config "$RUN_CFG"
