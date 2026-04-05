#!/usr/bin/env bash
set -euo pipefail
GREEN='\033[0;32m'; RED='\033[0;31m'; BLUE='\033[0;34m'; NC='\033[0m'
TIER="lite"; LOCATION="eastus"; RG="vibecody-rg"
while [[ $# -gt 0 ]]; do
  case "$1" in --tier) TIER="$2"; shift 2 ;; --location) LOCATION="$2"; shift 2 ;; --rg) RG="$2"; shift 2 ;;
    -h|--help) echo "Usage: $0 [--tier lite|pro|max] [--location REGION] [--rg RESOURCE_GROUP]"; exit 0 ;; *) shift ;; esac
done
command -v az &>/dev/null || { printf "${RED}[ERR]${NC} Azure CLI not found\n"; exit 1; }
az account show &>/dev/null || { printf "${RED}[ERR]${NC} Run: az login\n"; exit 1; }
printf "${BLUE}[INFO]${NC} Creating resource group %s...\n" "$RG"
az group create --name "$RG" --location "$LOCATION" -o none
printf "${BLUE}[INFO]${NC} Deploying VibeCody (tier=%s)...\n" "$TIER"
az deployment group create --resource-group "$RG" --template-file "$(dirname "$0")/main.bicep" --parameters tier="$TIER" -o none
URL=$(az containerapp show --name vibecody --resource-group "$RG" --query "properties.configuration.ingress.fqdn" -o tsv)
printf "\n${GREEN}[OK]${NC} VibeCody deployed at: https://%s\n" "$URL"
