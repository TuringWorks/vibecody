#!/usr/bin/env bash
#
# codesign-macos.sh — sign VibeCody's macOS release artifacts with a Developer
# ID Application certificate, then *verify* every signature.
#
# Why this exists
# ---------------
# `tauri build` signs the three `.app` bundles when APPLE_SIGNING_IDENTITY is
# set, but nothing signs the standalone binaries that ship in the tarballs
# (`vibecli`, `vibe-indexer`). Those were going out ad-hoc, so Gatekeeper on a
# fresh machine treats them as unidentified and `spctl` rejects them.
#
# Every step is verified rather than assumed. `codesign --sign` can report
# success and still leave a binary Gatekeeper refuses (wrong identity, missing
# hardened runtime, a nested unsigned dylib), so each target is re-read with
# `codesign --verify --strict` and the authority line is checked to be a
# Developer ID — not "adhoc". A script that prints "signed" without confirming
# it is worse than one that does nothing, because the release then ships
# believing it is signed.
#
# Usage
# -----
#   ./scripts/codesign-macos.sh                  # sign whatever is built
#   ./scripts/codesign-macos.sh --verify-only    # check, change nothing
#   APPLE_SIGNING_IDENTITY="Developer ID Application: Acme (TEAMID)" \
#     ./scripts/codesign-macos.sh
#
# Notarization is a separate step and is NOT done here — it needs an Apple ID
# and an app-specific password. See docs/release.md § Code signing.

set -euo pipefail

VERIFY_ONLY=0
[[ "${1:-}" == "--verify-only" ]] && VERIFY_ONLY=1

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

# ── Identity ──────────────────────────────────────────────────────────────────
#
# Prefer an explicit identity; otherwise auto-detect, but only when the choice
# is unambiguous. Picking the first of several certificates silently would mean
# shipping artifacts signed by whichever one happened to sort first.
resolve_identity() {
  if [[ -n "${APPLE_SIGNING_IDENTITY:-}" ]]; then
    printf '%s' "$APPLE_SIGNING_IDENTITY"
    return
  fi
  local found
  found="$(security find-identity -v -p codesigning 2>/dev/null \
    | grep 'Developer ID Application' \
    | sed -E 's/.*"(.*)".*/\1/')"
  local count
  count="$(printf '%s' "$found" | grep -c . || true)"
  if [[ "$count" -eq 0 ]]; then
    echo "error: no 'Developer ID Application' certificate in the keychain." >&2
    echo "       Install one from your Apple Developer account, or set" >&2
    echo "       APPLE_SIGNING_IDENTITY to name it explicitly." >&2
    exit 1
  fi
  if [[ "$count" -gt 1 ]]; then
    echo "error: $count Developer ID certificates found; set APPLE_SIGNING_IDENTITY" >&2
    echo "       to choose one:" >&2
    printf '         %s\n' $found >&2
    exit 1
  fi
  printf '%s' "$found"
}

IDENTITY="$(resolve_identity)"
echo "Identity: $IDENTITY"
echo

# ── Targets ───────────────────────────────────────────────────────────────────
#
# Only what actually exists is signed; a target that was not built is reported
# as skipped rather than failing the run, so signing a partial build is useful.
TARGETS=(
  "target/release/vibecli"
  "target/release/vibe-indexer"
  "vibecoder/src-tauri/target/release/bundle/macos/VibeCoder.app"
  "vibeaichat/src-tauri/target/release/bundle/macos/VibeAIChat.app"
  "vibedesk/src-tauri/target/release/bundle/macos/VibeDesk.app"
  "target/release/bundle/macos/VibeCoder.app"
  "target/release/bundle/macos/VibeAIChat.app"
  "target/release/bundle/macos/VibeDesk.app"
)

signed=0
skipped=0
failed=0

sign_one() {
  local target="$1"
  # `--options runtime` enables the hardened runtime, which notarization
  # requires; `--timestamp` embeds a secure timestamp, without which the
  # signature stops validating once the certificate expires.
  local args=(--force --sign "$IDENTITY" --options runtime --timestamp)

  # An .app carries entitlements; a bare binary does not need them, and
  # attaching app entitlements to a CLI is a way to get mysterious rejections.
  if [[ "$target" == *.app ]]; then
    local ent
    ent="$(entitlements_for "$target")"
    if [[ -n "$ent" ]]; then
      args+=(--entitlements "$ent")
    fi
    # Nested code (frameworks, helpers) must be signed before the outer bundle.
    args+=(--deep)
  fi

  codesign "${args[@]}" "$target"
}

entitlements_for() {
  case "$1" in
    *VibeCoder.app)  echo "vibecoder/src-tauri/macos/entitlements.plist" ;;
    *VibeAIChat.app) echo "vibeaichat/src-tauri/macos/entitlements.plist" ;;
    *VibeDesk.app)   echo "vibedesk/src-tauri/macos/entitlements.plist" ;;
    *)               echo "" ;;
  esac
}

# Confirm the signature is real, is ours, and is not ad-hoc.
verify_one() {
  local target="$1"

  # Capture, then match. Piping into `grep -q` looks equivalent but is not:
  # grep exits at the first match and closes the pipe, codesign takes SIGPIPE,
  # and `set -o pipefail` reports the whole pipeline as failed — so a perfectly
  # valid signature came back as "does not verify". Same trap as piping cargo
  # into `tail`.
  local verify
  verify="$(codesign --verify --strict --verbose=2 "$target" 2>&1)" || {
    echo "  FAIL  codesign --verify exited non-zero"
    printf '        %s\n' "$verify"
    return 1
  }
  if [[ "$verify" != *"valid on disk"* ]]; then
    echo "  FAIL  signature does not verify"
    printf '        %s\n' "$verify"
    return 1
  fi

  local info authority
  info="$(codesign -dv --verbose=4 "$target" 2>&1)"
  if grep -q 'Signature=adhoc' <<<"$info"; then
    echo "  FAIL  still ad-hoc signed"
    return 1
  fi
  authority="$(grep -m1 '^Authority=' <<<"$info" | sed 's/^Authority=//')"
  if [[ "$authority" != Developer\ ID\ Application:* ]]; then
    echo "  FAIL  unexpected authority: ${authority:-<none>}"
    return 1
  fi
  # A missing hardened runtime is accepted by codesign but rejected by
  # notarization, which is a slow and confusing place to discover it.
  if ! grep -q 'flags=.*runtime' <<<"$info"; then
    echo "  WARN  hardened runtime not enabled — notarization would reject this"
  fi
  echo "  ok    $authority"
  return 0
}

for target in "${TARGETS[@]}"; do
  if [[ ! -e "$target" ]]; then
    continue
  fi
  echo "$target"
  if [[ "$VERIFY_ONLY" -eq 0 ]]; then
    if ! sign_one "$target"; then
      echo "  FAIL  codesign --sign failed"
      failed=$((failed + 1))
      continue
    fi
  fi
  if verify_one "$target"; then
    signed=$((signed + 1))
  else
    failed=$((failed + 1))
  fi
  echo
done

# Anything in the list that was not built at all.
for target in "${TARGETS[@]}"; do
  [[ -e "$target" ]] || skipped=$((skipped + 1))
done

echo "──────────────────────────────────────────"
echo "verified: $signed    not built: $skipped    failed: $failed"

if [[ "$failed" -gt 0 ]]; then
  echo
  echo "Signing did not complete. Do not publish these artifacts as signed."
  exit 1
fi
if [[ "$signed" -eq 0 ]]; then
  echo
  echo "Nothing was signed — no release artifacts found. Build first:"
  echo "  make build-cli && make build-apps"
  exit 1
fi
