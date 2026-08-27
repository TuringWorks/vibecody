#!/usr/bin/env bash
#
# notarize-macos.sh — submit the signed macOS artifacts to Apple, then staple.
#
# Signing is not enough. A Developer ID signature with no notarization is still
# refused on any machine that did not build it: `spctl` reports
#
#     rejected
#     source=Unnotarized Developer ID
#
# Notarization is what turns that into `accepted`. Stapling then attaches the
# ticket to the artifact so it also works offline — an app that only passes
# while Apple is reachable is not shipped, it is on loan.
#
# Credentials
# -----------
# From a keychain profile, never from a file and never from the repo:
#
#     xcrun notarytool store-credentials vibecody \
#       --apple-id you@example.com \
#       --team-id  N7HV58M58W \
#       --password <app-specific-password>
#
# The password is an *app-specific* password from appleid.apple.com, not the
# Apple ID password. Override the profile name with NOTARY_PROFILE.
#
#   ./scripts/notarize-macos.sh                # notarize + staple what is built
#   ./scripts/notarize-macos.sh --verify-only  # report status, change nothing
set -euo pipefail

PROFILE="${NOTARY_PROFILE:-vibecody}"
VERIFY_ONLY=0
[[ "${1:-}" == "--verify-only" ]] && VERIFY_ONLY=1

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

command -v xcrun >/dev/null || { echo "error: xcrun not found (needs Xcode command line tools)" >&2; exit 1; }

# The .dmg is what people download and carries its own ticket; the .app inside
# it is stapled separately, because a user who drags the app out of the image
# keeps the app and not the image.
# `while read`, not `mapfile`: macOS ships bash 3.2, where mapfile does not
# exist, and this script runs on the machine that builds the release.
TARGETS=()
while IFS= read -r t; do
  [[ -n "$t" ]] && TARGETS+=("$t")
done < <(
  find target/release/bundle/dmg -maxdepth 1 -name '*.dmg' -not -name 'rw.*' 2>/dev/null || true
  find target/release/bundle/macos -maxdepth 1 -name '*.app' 2>/dev/null || true
)

if [[ ${#TARGETS[@]} -eq 0 ]]; then
  echo "nothing to notarize under target/release/bundle — build first." >&2
  exit 1
fi

ok=0; bad=0

assess() {
  local t="$1" out
  # `spctl` is the question a user's Mac asks at open time. `codesign --verify`
  # only confirms a signature exists, which was never the half that was failing.
  # A disk image is assessed as `open` against its primary signature; an app as
  # `exec`. Using the wrong one reports a pass that the user will not get.
  if [[ "$t" == *.dmg ]]; then
    out="$(spctl -a -vv -t open --context context:primary-signature "$t" 2>&1 || true)"
  else
    out="$(spctl -a -vv -t exec "$t" 2>&1 || true)"
  fi
  if grep -q "accepted" <<<"$out"; then
    echo "  ok    Gatekeeper accepts"
    return 0
  fi
  sed 's/^/        /' <<<"$out" | head -3
  return 1
}

for t in "${TARGETS[@]}"; do
  echo "$t"
  if [[ $VERIFY_ONLY -eq 0 ]]; then
    zip="$(mktemp -d)/$(basename "$t").zip"
    # ditto, not zip: it preserves the bundle's symlinks and extended
    # attributes, and a plain zip silently corrupts a .app on the way to Apple.
    ditto -c -k --keepParent "$t" "$zip"
    if ! xcrun notarytool submit "$zip" --keychain-profile "$PROFILE" --wait; then
      echo "  FAIL  submission rejected — 'xcrun notarytool log <id> --keychain-profile $PROFILE' has the reason" >&2
      bad=$((bad + 1)); echo; continue
    fi
    # Staple the *original*, not the zip: the zip was only the transport.
    xcrun stapler staple "$t"
  fi
  if assess "$t"; then ok=$((ok + 1)); else bad=$((bad + 1)); fi
  echo
done

echo "──────────────────────────────────────────"
echo "accepted: $ok    rejected: $bad"
[[ $bad -eq 0 ]] || { echo; echo "Do not publish these as notarized."; exit 1; }
