#!/usr/bin/env bash
set -euo pipefail
GREEN='\033[0;32m'; BLUE='\033[0;34m'; RED='\033[0;31m'; NC='\033[0m'
TIER="lite"; REGION="us-central1"; PROJECT=""
while [[ $# -gt 0 ]]; do
  case "$1" in --tier) TIER="$2"; shift 2 ;; --region) REGION="$2"; shift 2 ;; --project) PROJECT="$2"; shift 2 ;;
    -h|--help) echo "Usage: $0 --project PROJECT_ID [--tier lite|pro|max] [--region REGION]"; exit 0 ;; *) shift ;; esac
done
[[ -n "$PROJECT" ]] || { printf "${RED}[ERR]${NC} --project is required\n"; exit 1; }
command -v gcloud &>/dev/null || { printf "${RED}[ERR]${NC} gcloud not found\n"; exit 1; }
command -v terraform &>/dev/null || { printf "${RED}[ERR]${NC} terraform not found\n"; exit 1; }
gcloud auth print-access-token &>/dev/null || { printf "${RED}[ERR]${NC} Run: gcloud auth login\n"; exit 1; }
cd "$(dirname "$0")"
terraform init -input=false
terraform apply -auto-approve -var="project_id=$PROJECT" -var="tier=$TIER" -var="region=$REGION"
URL=$(terraform output -raw url)
printf "\n${GREEN}[OK]${NC} VibeCody deployed at: %s\n" "$URL"
