#!/usr/bin/env bash
set -euo pipefail
STACK="vibecody"; REGION="${AWS_DEFAULT_REGION:-us-east-1}"
while [[ $# -gt 0 ]]; do
  case "$1" in --stack) STACK="$2"; shift 2 ;; --region) REGION="$2"; shift 2 ;; *) shift ;; esac
done
printf "Delete stack '%s' in %s? [y/N] " "$STACK" "$REGION"; read -r c
[[ "$c" =~ ^[Yy] ]] || { echo "Aborted."; exit 0; }
aws cloudformation delete-stack --stack-name "$STACK" --region "$REGION"
aws cloudformation wait stack-delete-complete --stack-name "$STACK" --region "$REGION"
echo "Done."
