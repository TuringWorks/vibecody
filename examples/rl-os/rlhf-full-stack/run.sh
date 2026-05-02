#!/usr/bin/env bash
# Slice 7c-extras+1 — full RLHF pipeline: REWARD_MODEL training, then
# PPO RLHF using that reward model.
#
# Requires `--extra rlhf`. Re-uses the rlhf-dpo seeding script.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
SIDECAR_DIR="$REPO_ROOT/vibe-rl-py"
WORKSPACE="$SCRIPT_DIR/.workspace"
SEED_SCRIPT="$SCRIPT_DIR/../rlhf-dpo/seed_preferences.py"
SUITE_ID="rlhf-demo"

RM_RUN_ID="rm-001"
PPO_RUN_ID="ppo-rlhf-001"

mkdir -p \
  "$WORKSPACE/.vibecli/rl-artifacts/$RM_RUN_ID" \
  "$WORKSPACE/.vibecli/rl-artifacts/$PPO_RUN_ID"

echo "==> seeding $SUITE_ID preferences"
cd "$SIDECAR_DIR"
uv run python "$SEED_SCRIPT" "$WORKSPACE" "$SUITE_ID"

# ---------- Stage 1: reward model ----------
RM_CFG="$WORKSPACE/.vibecli/rl-artifacts/$RM_RUN_ID/config.yaml"
{
  cat "$SCRIPT_DIR/rm-config.yaml"
  printf 'workspace_path: %s\nartifact_dir: %s\n' \
    "$WORKSPACE" \
    "$WORKSPACE/.vibecli/rl-artifacts/$RM_RUN_ID"
} > "$RM_CFG"

echo
echo "==========================================================="
echo "  Stage 1/2 — REWARD_MODEL training"
echo "==========================================================="
echo "==> run id: $RM_RUN_ID"
echo
uv run python -m vibe_rl train --run-id "$RM_RUN_ID" --config "$RM_CFG"

RM_CKPT="$WORKSPACE/.vibecli/rl-artifacts/$RM_RUN_ID/final.pt"
if [[ ! -f "$RM_CKPT" ]]; then
  echo "ERROR: reward model checkpoint not produced at $RM_CKPT" >&2
  exit 1
fi

# ---------- Stage 2: PPO RLHF ----------
PPO_CFG="$WORKSPACE/.vibecli/rl-artifacts/$PPO_RUN_ID/config.yaml"
{
  cat "$SCRIPT_DIR/ppo-config.yaml"
  printf 'workspace_path: %s\nartifact_dir: %s\nreward_model_id: %s\n' \
    "$WORKSPACE" \
    "$WORKSPACE/.vibecli/rl-artifacts/$PPO_RUN_ID" \
    "$RM_CKPT"
} > "$PPO_CFG"

echo
echo "==========================================================="
echo "  Stage 2/2 — PPO RLHF using the trained reward model"
echo "==========================================================="
echo "==> run id:    $PPO_RUN_ID"
echo "==> rm ckpt:   $RM_CKPT"
echo
exec uv run python -m vibe_rl train --run-id "$PPO_RUN_ID" --config "$PPO_CFG"
