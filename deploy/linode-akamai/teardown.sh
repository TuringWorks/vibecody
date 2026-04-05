#!/usr/bin/env bash
set -euo pipefail
printf "Destroy Linode resources? [y/N] "; read -r c
[[ "$c" =~ ^[Yy] ]] || exit 0
cd "$(dirname "$0")" && terraform destroy -auto-approve -var="linode_token=$LINODE_TOKEN"
