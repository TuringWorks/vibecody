#!/usr/bin/env bash
set -euo pipefail
GREEN='\033[0;32m'; RED='\033[0;31m'; NC='\033[0m'
TIER="lite"
while [[ $# -gt 0 ]]; do case "$1" in --tier) TIER="$2"; shift 2 ;; -h|--help) echo "Usage: $0 [--tier lite|pro|max]"; exit 0 ;; *) shift ;; esac; done
command -v terraform &>/dev/null || { printf "${RED}[ERR]${NC} Terraform not found\n"; exit 1; }
[[ -n "${LINODE_TOKEN:-}" ]] || { printf "${RED}[ERR]${NC} Set LINODE_TOKEN\n"; exit 1; }
cd "$(dirname "$0")"
terraform init -input=false
terraform apply -auto-approve -var="linode_token=$LINODE_TOKEN" -var="tier=$TIER"
printf "\n${GREEN}[OK]${NC} VibeCody deployed: %s\n" "$(terraform output -raw url)"
