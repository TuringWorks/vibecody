#!/usr/bin/env bash
set -euo pipefail
printf "Destroy VibeCody GCP resources? [y/N] "; read -r c
[[ "$c" =~ ^[Yy] ]] || { echo "Aborted."; exit 0; }
cd "$(dirname "$0")" && terraform destroy -auto-approve
echo "Done."
