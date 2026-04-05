#!/usr/bin/env bash
set -euo pipefail
printf "Destroy Oracle Cloud resources? [y/N] "; read -r c
[[ "$c" =~ ^[Yy] ]] || { echo "Aborted."; exit 0; }
cd "$(dirname "$0")" && terraform destroy -auto-approve
echo "Done."
