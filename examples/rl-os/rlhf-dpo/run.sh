#!/usr/bin/env bash
# Slice 7c — Direct Preference Optimization on distilgpt2 with 6 toy
# preference pairs. Demonstrates the DPO loss + reference-model KL.
#
# Requires `--extra rlhf` for transformers + accelerate.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
SIDECAR_DIR="$REPO_ROOT/vibe-rl-py"
WORKSPACE="$SCRIPT_DIR/.workspace"
RUN_ID="dpo-001"
SUITE_ID="dpo-demo"

mkdir -p "$WORKSPACE/.vibecli/rl-artifacts/$RUN_ID"

echo "==> seeding $SUITE_ID preferences"
cd "$SIDECAR_DIR"
uv run python "$SCRIPT_DIR/seed_preferences.py" "$WORKSPACE" "$SUITE_ID"

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

exec uv run python -m vibe_rl train --run-id "$RUN_ID" --config "$RUN_CFG"
