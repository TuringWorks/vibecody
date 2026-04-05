#!/usr/bin/env bash
set -euo pipefail
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; BLUE='\033[0;34m'; NC='\033[0m'
info() { printf "${BLUE}[INFO]${NC}  %s\n" "$*"; }
ok()   { printf "${GREEN}[OK]${NC}    %s\n" "$*"; }
err()  { printf "${RED}[ERR]${NC}   %s\n" "$*" >&2; }

TIER="lite"; STACK="vibecody"; REGION="${AWS_DEFAULT_REGION:-us-east-1}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --tier)   TIER="$2"; shift 2 ;;
    --region) REGION="$2"; shift 2 ;;
    --stack)  STACK="$2"; shift 2 ;;
    -h|--help) echo "Usage: $0 [--tier lite|pro|max] [--region REGION] [--stack NAME]"; exit 0 ;;
    *) err "Unknown: $1"; exit 1 ;;
  esac
done

info "Checking prerequisites..."
command -v aws &>/dev/null || { err "AWS CLI not found. Install: https://aws.amazon.com/cli/"; exit 1; }
aws sts get-caller-identity &>/dev/null || { err "Not authenticated. Run: aws configure"; exit 1; }
ok "AWS CLI authenticated ($(aws sts get-caller-identity --query Account --output text))"

info "Deploying VibeCody (tier=$TIER, region=$REGION)..."
aws cloudformation deploy \
  --stack-name "$STACK" \
  --template-file "$SCRIPT_DIR/cloudformation.yaml" \
  --parameter-overrides "Tier=$TIER" \
  --capabilities CAPABILITY_IAM \
  --region "$REGION" \
  --no-fail-on-empty-changeset

URL=$(aws cloudformation describe-stacks --stack-name "$STACK" --region "$REGION" \
  --query "Stacks[0].Outputs[?OutputKey=='ServiceURL'].OutputValue" --output text)

echo ""
ok "VibeCody deployed!"
echo "  URL:    $URL"
echo "  Health: $URL/health"
echo "  Tier:   $TIER"
echo "  Teardown: ./teardown.sh --stack $STACK --region $REGION"
