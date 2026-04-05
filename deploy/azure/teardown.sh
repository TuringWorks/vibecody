#!/usr/bin/env bash
set -euo pipefail
RG="vibecody-rg"
while [[ $# -gt 0 ]]; do case "$1" in --rg) RG="$2"; shift 2 ;; *) shift ;; esac; done
printf "Delete resource group '%s'? [y/N] " "$RG"; read -r c
[[ "$c" =~ ^[Yy] ]] || { echo "Aborted."; exit 0; }
az group delete --name "$RG" --yes --no-wait
echo "Deletion initiated."
