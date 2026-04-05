#!/usr/bin/env bash
set -euo pipefail
GREEN='\033[0;32m'; RED='\033[0;31m'; BLUE='\033[0;34m'; NC='\033[0m'
TIER="lite"
while [[ $# -gt 0 ]]; do
  case "$1" in --tier) TIER="$2"; shift 2 ;; -h|--help)
    echo "Usage: $0 [--tier lite|pro|max]"
    echo "Note: Oracle Cloud always-free tier supports up to 4 OCPU + 24 GB ARM — \$0/month!"
    exit 0 ;; *) shift ;; esac
done
command -v oci &>/dev/null || { printf "${RED}[ERR]${NC} OCI CLI not found. Install: https://docs.oracle.com/iaas/Content/API/SDKDocs/cliinstall.htm\n"; exit 1; }
command -v terraform &>/dev/null || { printf "${RED}[ERR]${NC} Terraform not found\n"; exit 1; }
cd "$(dirname "$0")"
terraform init -input=false
terraform apply -auto-approve -var="tier=$TIER"
printf "\n${GREEN}[OK]${NC} VibeCody deployed on Oracle Cloud (always-free eligible)!\n"
