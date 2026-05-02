#!/usr/bin/env bash
# Slice 7a — INT8 dynamic ONNX quantization of the cartpole-baseline checkpoint.
#
# Requires `--extra opt` for the optimization toolkit (onnx, onnxruntime).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
SIDECAR_DIR="$REPO_ROOT/vibe-rl-py"
WORKSPACE="$SCRIPT_DIR/.workspace"
RUN_ID="quantize-001"

CARTPOLE_DIR="$SCRIPT_DIR/../cartpole-baseline/.workspace"
SOURCE_CKPT="$CARTPOLE_DIR/.vibecli/rl-artifacts/cartpole-baseline-001/final.pt"

if [[ ! -f "$SOURCE_CKPT" ]]; then
  echo "ERROR: cartpole checkpoint not found at:"
  echo "  $SOURCE_CKPT"
  echo
  echo "Run cartpole-baseline first:"
  echo "  ../cartpole-baseline/run.sh"
  exit 2
fi

mkdir -p "$WORKSPACE/.vibecli/rl-artifacts/$RUN_ID"

RUN_CFG="$WORKSPACE/.vibecli/rl-artifacts/$RUN_ID/config.yaml"
{
  cat "$SCRIPT_DIR/config.yaml"
  printf 'workspace_path: %s\nartifact_dir: %s\nsource_checkpoint: %s\n' \
    "$WORKSPACE" \
    "$WORKSPACE/.vibecli/rl-artifacts/$RUN_ID" \
    "$SOURCE_CKPT"
} > "$RUN_CFG"

echo "==> repo root:   $REPO_ROOT"
echo "==> workspace:   $WORKSPACE"
echo "==> source ckpt: $SOURCE_CKPT"
echo "==> run id:      $RUN_ID"
echo

cd "$SIDECAR_DIR"
exec uv run python -m vibe_rl train --run-id "$RUN_ID" --config "$RUN_CFG"
