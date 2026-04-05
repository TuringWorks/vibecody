#!/usr/bin/env bash
set -euo pipefail
printf "Destroy DigitalOcean resources? [y/N] "; read -r c
[[ "$c" =~ ^[Yy] ]] || { echo "Aborted."; exit 0; }
cd "$(dirname "$0")" && terraform destroy -auto-approve -var="do_token=$DIGITALOCEAN_TOKEN"
