#!/usr/bin/env bash
#
# tauri-build.sh — build a Tauri shell, signed when a certificate exists.
#
# Why this exists
# ---------------
# `tauri build` ad-hoc signs whenever APPLE_SIGNING_IDENTITY is unset, and the
# build log looks *identical* either way — both print `Signing with identity`.
# The only visible difference is one line about notarization, which reads as a
# warning about a separate step rather than "this bundle is not signed". So a
# local build silently produced artifacts Gatekeeper rejects on every machine
# but the one that built them, and nothing said so.
#
# The identity comes from codesign-macos.sh --print-identity rather than being
# resolved again here: it already refuses to guess between several certificates,
# which is the part worth having once.
#
#   ./scripts/tauri-build.sh vibedesk [-- extra tauri args]
#
# Unset is the only value meaning "don't sign". Tauri treats a
# present-but-empty Apple variable as an explicit request — an empty
# APPLE_SIGNING_IDENTITY becomes `codesign -s ""` ("no identity found") and
# empty notarization variables become an attempt that dies on "Team ID must be
# at least 3 characters". Both broke the v0.5.8 macOS release, one after the
# other; see .github/workflows/release.yml.
set -euo pipefail

APP_DIR="${1:?usage: tauri-build.sh <app-dir> [-- tauri args]}"
shift || true
[[ "${1:-}" == "--" ]] && shift || true

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if [[ "$(uname)" == "Darwin" ]]; then
  if [[ -z "${APPLE_SIGNING_IDENTITY:-}" ]]; then
    if ident="$("$repo_root/scripts/codesign-macos.sh" --print-identity 2>/dev/null)"; then
      export APPLE_SIGNING_IDENTITY="$ident"
    fi
  fi

  if [[ -n "${APPLE_SIGNING_IDENTITY:-}" ]]; then
    echo "signing as: $APPLE_SIGNING_IDENTITY"
  else
    # Say what the artifact will actually be. "no certificate found" is a fact
    # about this machine; "Gatekeeper will reject it" is what the reader needs.
    echo "warning: no Developer ID certificate — this build will be ad-hoc signed." >&2
    echo "         It runs here and is rejected on every other Mac." >&2
    echo "         Set APPLE_SIGNING_IDENTITY, or install a Developer ID Application cert." >&2
  fi

  # Notarization is a *separate* credential set, and Tauri only attempts it when
  # all three are present. Signed-but-unnotarized is still rejected by
  # Gatekeeper on a downloaded app — `spctl` reports it as
  # "source=Unnotarized Developer ID" — so this is worth saying explicitly
  # rather than leaving as the one-line warning Tauri prints.
  if [[ -n "${APPLE_SIGNING_IDENTITY:-}" ]] &&
     { [[ -z "${APPLE_ID:-}" ]] || [[ -z "${APPLE_PASSWORD:-}" ]] || [[ -z "${APPLE_TEAM_ID:-}" ]]; }; then
    echo "note: not notarizing — APPLE_ID / APPLE_PASSWORD / APPLE_TEAM_ID not all set." >&2
    echo "      A signed but unnotarized app is still blocked on a downloaded copy." >&2
    echo "      Run 'make notarize-macos' after this, or see docs/release.md." >&2
  fi
fi

cd "$APP_DIR"
exec npm run tauri:build -- "$@"
