#!/usr/bin/env bash
# Configure Apple Developer ID signing + notarization for the Release workflow.
#
# Why this exists: without these secrets every macOS artifact CI produces is
# ad-hoc signed with the hardened runtime enabled. macOS *kills* that
# combination on launch while the download-quarantine flag is set — the app
# bounces once and dies, or Finder claims it "is damaged". Users then have to
# run `xattr -dr com.apple.quarantine` by hand for every app, forever.
#
# Run this on a Mac that already has the Developer ID certificate in its
# keychain. Nothing here uploads anywhere except GitHub's encrypted-secret API,
# and the .p12 is written to a temp file that is deleted on exit.
#
#   ./scripts/setup-apple-signing.sh
#
# Re-running is safe: `gh secret set` overwrites.

set -euo pipefail

REPO="${REPO:-TuringWorks/vibecody}"

die() { printf '\n\033[31merror:\033[0m %s\n' "$*" >&2; exit 1; }
step() { printf '\n\033[1m── %s\033[0m\n' "$*"; }

# extract_identity <bundle.pem> <cn-substring> <out.p12> <password>
#
# Pulls one identity out of a multi-identity PEM bundle and rebuilds a .p12
# containing only it. Certificate and private key are paired by public modulus:
# export order is not guaranteed, and pairing by position produces a .p12 that
# imports cleanly and then cannot sign.
extract_identity() {
  local BUNDLE="$1" CN="$2" OUT="$3" PW="$4"
  local D; D="$(mktemp -d)"
  awk -v d="$D" 'BEGIN{c=0;k=0}
    /BEGIN CERTIFICATE/{inc=1;c++;f=sprintf("%s/cert_%03d.pem",d,c)}
    inc{print > f}
    /END CERTIFICATE/{inc=0}
    /BEGIN .*PRIVATE KEY/{ink=1;k++;g=sprintf("%s/key_%03d.pem",d,k)}
    ink{print > g}
    /END .*PRIVATE KEY/{ink=0}
    END{printf "%d %d\n",c,k > (d"/counts")}' "$BUNDLE"
  local NCERT NKEY; read -r NCERT NKEY < "$D/counts"

  local CHOSEN="" i=1
  while [ "$i" -le "$NCERT" ]; do
    local f; f="$(printf '%s/cert_%03d.pem' "$D" "$i")"
    if openssl x509 -in "$f" -noout -subject 2>/dev/null | grep -qF "$CN"; then CHOSEN="$f"; break; fi
    i=$((i+1))
  done
  [ -n "$CHOSEN" ] || { rm -rf "$D"; return 1; }

  local CMOD; CMOD="$(openssl x509 -in "$CHOSEN" -noout -modulus | openssl md5)"
  local KEYFILE="" j=1
  while [ "$j" -le "$NKEY" ]; do
    local g; g="$(printf '%s/key_%03d.pem' "$D" "$j")"
    if [ "$(openssl rsa -in "$g" -noout -modulus 2>/dev/null | openssl md5 || true)" = "$CMOD" ]; then
      KEYFILE="$g"; break
    fi
    j=$((j+1))
  done
  [ -n "$KEYFILE" ] || { rm -rf "$D"; return 2; }

  # Other certificates ride along as chain material (public data, no keys) so
  # the runner can build a full chain without Apple's intermediates installed.
  cat "$D"/cert_*.pem > "$D/chain.pem" 2>/dev/null || true
  openssl pkcs12 -export -inkey "$KEYFILE" -in "$CHOSEN" -certfile "$D/chain.pem" \
      -passout "pass:$PW" -out "$OUT" 2>/dev/null \
    || openssl pkcs12 -export -inkey "$KEYFILE" -in "$CHOSEN" \
      -passout "pass:$PW" -out "$OUT" 2>/dev/null \
    || { rm -rf "$D"; return 3; }
  rm -rf "$D"
}

# verify_p12 <p12> <password> -> prints "certs keys", fails if no private key
verify_p12() {
  local PEM; PEM="$(mktemp)"
  openssl pkcs12 -in "$1" -nodes -passin "pass:$2" -legacy -out "$PEM" 2>/dev/null \
    || openssl pkcs12 -in "$1" -nodes -passin "pass:$2" -out "$PEM" 2>/dev/null \
    || { rm -f "$PEM"; return 1; }
  grep -q 'BEGIN .*PRIVATE KEY' "$PEM" || { rm -f "$PEM"; return 2; }
  printf '%s %s' "$(grep -c 'BEGIN CERTIFICATE' "$PEM")" "$(grep -c 'BEGIN .*PRIVATE KEY' "$PEM")"
  rm -f "$PEM"
}

command -v gh >/dev/null || die "gh CLI not found — brew install gh"
gh auth status >/dev/null 2>&1 || die "gh is not authenticated — run: gh auth login"
[[ "$(uname -s)" == "Darwin" ]] || die "must run on macOS (needs the keychain)"

# ── 1. Pick the signing identity ──────────────────────────────────────────────
step "Developer ID Application identities in your keychain"
# Deliberately not `mapfile`: macOS ships bash 3.2, where it does not exist.
# There it would silently yield an empty array and this script would report
# "no identity found" on a machine that has one.
IDENTITIES=()
while IFS= read -r line; do
  [[ -n "$line" ]] && IDENTITIES+=("$line")
done < <(
  security find-identity -v -p codesigning 2>/dev/null \
    | grep 'Developer ID Application' \
    | sed -E 's/^[[:space:]]*[0-9]+\) [0-9A-F]+ "(.*)"$/\1/'
)
(( ${#IDENTITIES[@]} )) || die \
  "no 'Developer ID Application' identity found.
   Create one at https://developer.apple.com/account/resources/certificates
   then download it and double-click to install into your keychain."

if (( ${#IDENTITIES[@]} == 1 )); then
  IDENTITY="${IDENTITIES[0]}"
  echo "   using: $IDENTITY"
else
  for i in "${!IDENTITIES[@]}"; do echo "   $((i+1))) ${IDENTITIES[$i]}"; done
  read -rp "   pick [1-${#IDENTITIES[@]}]: " n
  IDENTITY="${IDENTITIES[$((n-1))]}"
fi

# "Developer ID Application: Name (TEAMID)" — the Team ID is the parenthesised tail.
TEAM_ID="$(sed -E 's/.*\(([A-Z0-9]+)\)$/\1/' <<<"$IDENTITY")"
[[ "$TEAM_ID" =~ ^[A-Z0-9]{10}$ ]] || die "could not parse a 10-character Team ID from: $IDENTITY"
echo "   team id: $TEAM_ID"

# ── 2. Build a .p12 per certificate type ──────────────────────────────────────
# Developer ID and Apple Distribution are NOT interchangeable:
#   Developer ID Application -> macOS outside the App Store (VibeCoder, vibecli…)
#   Apple Distribution       -> iOS and watchOS
# Signing the mobile targets with a Developer ID certificate fails; they get
# separate secrets so each job imports exactly the identity it can use.
step "Certificate export"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
P12="$TMP/developer-id.p12"
P12_PASSWORD="$(openssl rand -base64 24)"
DIST_P12="$TMP/distribution.p12"
DIST_P12_PASSWORD="$(openssl rand -base64 24)"
DIST_IDENTITY=""

ALL_P12="$TMP/all.p12"
ALL_PASSWORD="$(openssl rand -base64 24)"
BUNDLE="$TMP/bundle.pem"
echo "   macOS will prompt for your login-keychain password."
echo "   Choose \"Allow\" (or \"Always Allow\") when it asks."

if security export -t identities -f pkcs12 -P "$ALL_PASSWORD" -o "$ALL_P12" 2>"$TMP/export.err"; then
  openssl pkcs12 -in "$ALL_P12" -nodes -passin "pass:$ALL_PASSWORD" -legacy -out "$BUNDLE" 2>/dev/null \
    || openssl pkcs12 -in "$ALL_P12" -nodes -passin "pass:$ALL_PASSWORD" -out "$BUNDLE" 2>/dev/null \
    || die "could not read back the exported .p12"
  rm -f "$ALL_P12"
  echo "   exported $(grep -c 'BEGIN .*PRIVATE KEY' "$BUNDLE") identity/identities"

  extract_identity "$BUNDLE" "${IDENTITY%% (*}" "$P12" "$P12_PASSWORD" \
    || die "could not isolate '$IDENTITY' from the export"
  echo "   built macOS .p12 (Developer ID)"

  # Apple Distribution is optional: only needed if iOS/watchOS get signed.
  DIST_IDENTITY="$(security find-identity -v -p codesigning 2>/dev/null \
    | grep 'Apple Distribution' \
    | sed -E 's/^[[:space:]]*[0-9]+\) [0-9A-F]+ "(.*)"$/\1/' | head -1 || true)"
  if [ -n "$DIST_IDENTITY" ]; then
    if extract_identity "$BUNDLE" "${DIST_IDENTITY%% (*}" "$DIST_P12" "$DIST_P12_PASSWORD"; then
      echo "   built mobile .p12 (Apple Distribution): $DIST_IDENTITY"
    else
      echo "   \033[33mnote:\033[0m found '$DIST_IDENTITY' but could not isolate it — iOS/watchOS signing will be skipped"
      DIST_IDENTITY=""
    fi
  else
    echo "   no Apple Distribution identity in the keychain — iOS/watchOS cannot be signed"
  fi
  rm -f "$BUNDLE"
else
  echo "   \033[33msecurity export failed:\033[0m $(tr -d '\n' < "$TMP/export.err" | head -c 200)"
  cat <<EOF

   Fall back to a manual export. If Keychain Access greys out the .p12 option:
     - select the \033[1mMy Certificates\033[0m category, not "Certificates"
     - expand the certificate so its private key is selected with it
     - if still greyed out the key is non-extractable and must be reissued

EOF
  read -rp "   Path to a .p12 you exported: " USER_P12
  USER_P12="${USER_P12/#\~/$HOME}"
  USER_P12="$(printf '%s' "$USER_P12" | sed -E "s/^['\"]//; s/['\"]\$//; s/\\\\ / /g")"
  [ -f "$USER_P12" ] || die "no such file: $USER_P12"
  read -rsp "   Password you set on that .p12: " P12_PASSWORD; echo
  cp "$USER_P12" "$P12"
fi

# Verify before uploading: a wrong password or a certificate-without-key
# imports cleanly in CI and then fails at `codesign`, 40 minutes in.
COUNTS="$(verify_p12 "$P12" "$P12_PASSWORD")" \
  || die "the macOS .p12 is unusable (wrong password, or no private key) — nothing uploaded."
echo "   verified macOS .p12: $COUNTS (certs keys)"
if [ -n "$DIST_IDENTITY" ]; then
  DCOUNTS="$(verify_p12 "$DIST_P12" "$DIST_P12_PASSWORD")" \
    || die "the distribution .p12 is unusable — nothing uploaded."
  echo "   verified mobile .p12: $DCOUNTS (certs keys)"
fi

# ── 3. Apple ID + app-specific password (for notarization) ────────────────────
step "Notarization credentials"
echo "   Signing alone is not enough: a Developer ID app that is NOT notarized"
echo "   still trips Gatekeeper on download. Notarization needs your Apple ID"
echo "   and an app-specific password (NOT your account password)."
echo "   Create one at https://appleid.apple.com → Sign-In and Security →"
echo "   App-Specific Passwords."
echo
read -rp "   Apple ID email: " APPLE_ID_EMAIL
[[ -n "$APPLE_ID_EMAIL" ]] || die "Apple ID is required for notarization"
read -rsp "   App-specific password (xxxx-xxxx-xxxx-xxxx): " APP_PASSWORD; echo
[[ "$APP_PASSWORD" =~ ^[a-z]{4}-[a-z]{4}-[a-z]{4}-[a-z]{4}$ ]] \
  || echo "   note: that does not look like the usual xxxx-xxxx-xxxx-xxxx shape — continuing anyway"

# Fail here rather than at the end of a release build.
step "Validating the credentials against Apple's notary service"
if xcrun notarytool history --apple-id "$APPLE_ID_EMAIL" --password "$APP_PASSWORD" \
     --team-id "$TEAM_ID" >/dev/null 2>&1; then
  echo "   accepted by notarytool"
else
  die "Apple rejected these credentials.
   Check the Apple ID, the app-specific password, and that the Apple ID is a
   member of team $TEAM_ID. Nothing has been uploaded."
fi

# ── 4. Push the secrets ───────────────────────────────────────────────────────
step "Setting repository secrets on $REPO"
base64 -i "$P12" | gh secret set APPLE_CERT_P12_BASE64      --repo "$REPO"
printf '%s' "$P12_PASSWORD"    | gh secret set APPLE_CERT_P12_PASSWORD    --repo "$REPO"
printf '%s' "$IDENTITY"        | gh secret set APPLE_SIGNING_IDENTITY     --repo "$REPO"
printf '%s' "$TEAM_ID"         | gh secret set APPLE_TEAM_ID              --repo "$REPO"
printf '%s' "$APPLE_ID_EMAIL"  | gh secret set APPLE_ID                   --repo "$REPO"
printf '%s' "$APP_PASSWORD"    | gh secret set APPLE_APP_SPECIFIC_PASSWORD --repo "$REPO"

if [ -n "$DIST_IDENTITY" ]; then
  # Separate from the Developer ID secret on purpose: iOS/watchOS jobs import
  # this one, macOS jobs import the other. A job that imports the wrong type
  # fails at signing with an unhelpful "no matching identity" error.
  base64 -i "$DIST_P12" | gh secret set APPLE_DIST_CERT_P12_BASE64 --repo "$REPO"
  printf '%s' "$DIST_P12_PASSWORD" | gh secret set APPLE_DIST_CERT_P12_PASSWORD --repo "$REPO"
  printf '%s' "$DIST_IDENTITY"     | gh secret set APPLE_DIST_SIGNING_IDENTITY  --repo "$REPO"
fi

# ── 5. iOS + watchOS provisioning profiles (optional) ─────────────────────────
# These need a *provisioning profile* each, on top of the Apple Distribution
# certificate uploaded above. Profiles are per-App-ID and expire annually, which
# is why they are a separate, skippable phase rather than part of the main flow.
step "iOS / watchOS signing (optional)"

if [ -z "$DIST_IDENTITY" ]; then
  echo "   No Apple Distribution certificate — iOS and watchOS stay unsigned."
  echo "   Create one at https://developer.apple.com/account/resources/certificates"
  echo "   (type: Apple Distribution), install it, then re-run this script."
  IOS_STATE="unsigned"; WATCH_STATE="unsigned"
else
  cat <<'EOF'
   Signing these needs a distribution provisioning profile per App ID:

     iOS      dev.vibecody.vibecodyMobile
     watchOS  com.turingworks.vibecody.watch
              com.turingworks.vibecody.watch.complication

   Create the App IDs and profiles at
   https://developer.apple.com/account/resources/profiles
   (type: App Store or Ad Hoc distribution, cert: Apple Distribution)

   Leave a path blank to skip that platform; it keeps shipping unsigned.
EOF

  ask_profile() {  # $1=label $2=expected-bundle-id -> echoes the path, or empty
    local path
    read -rp "   $1 .mobileprovision (blank = skip): " path
    path="${path/#\~/$HOME}"
    path="$(printf '%s' "$path" | sed -E "s/^['\"]//; s/['\"]$//; s/\\\\ / /g")"
    [ -n "$path" ] || return 0
    [ -f "$path" ] || die "no such file: $path"
    local plist; plist="$(security cms -D -i "$path" 2>/dev/null || true)"
    [ -n "$plist" ] || die "cannot parse that file as a provisioning profile"
    local team expiry appid
    team="$(printf '%s' "$plist" | plutil -extract TeamIdentifier.0 raw - 2>/dev/null || true)"
    expiry="$(printf '%s' "$plist" | plutil -extract ExpirationDate raw - 2>/dev/null || echo unknown)"
    appid="$(printf '%s' "$plist" | plutil -extract Entitlements.application-identifier raw - 2>/dev/null || echo '?')"
    [ "$team" = "$TEAM_ID" ] || die "that profile is team $team, but signing is team $TEAM_ID"
    case "$appid" in
      *"$2") : ;;
      *) echo "   \033[33mwarning:\033[0m profile app id is '$appid', expected to end with '$2'" ;;
    esac
    echo "   team $team, app id $appid, expires $expiry" >&2
    printf '%s' "$path"
  }

  IOS_PROFILE="$(ask_profile 'iOS    ' 'dev.vibecody.vibecodyMobile')"
  if [ -n "$IOS_PROFILE" ]; then
    base64 -i "$IOS_PROFILE" | gh secret set APPLE_IOS_PROFILE_BASE64 --repo "$REPO"
    IOS_STATE="signed"
  else
    IOS_STATE="unsigned"
  fi

  WATCH_PROFILE="$(ask_profile 'watchOS' 'com.turingworks.vibecody.watch')"
  if [ -n "$WATCH_PROFILE" ]; then
    base64 -i "$WATCH_PROFILE" | gh secret set APPLE_PROVISIONING_PROFILE_BASE64 --repo "$REPO"
    WATCH_STATE="signed"
  else
    WATCH_STATE="unsigned"
  fi

  if [ "$WATCH_STATE" = "signed" ]; then
    echo
    echo "   TestFlight upload (optional) needs an App Store Connect API key."
    echo "   https://appstoreconnect.apple.com/access/integrations/api"
    read -rp "   Path to AuthKey_XXXXXX.p8 (blank = skip): " ASC_KEY_PATH
    ASC_KEY_PATH="${ASC_KEY_PATH/#\~/$HOME}"
    ASC_KEY_PATH="$(printf '%s' "$ASC_KEY_PATH" | sed -E "s/^['\"]//; s/['\"]$//; s/\\\\ / /g")"
    if [ -n "$ASC_KEY_PATH" ]; then
      [ -f "$ASC_KEY_PATH" ] || die "no such file: $ASC_KEY_PATH"
      grep -q 'BEGIN PRIVATE KEY' "$ASC_KEY_PATH" || die "that is not a .p8 private key"
      ASC_KEY_ID_GUESS="$(basename "$ASC_KEY_PATH" | sed -E 's/^AuthKey_(.*)\.p8$/\1/')"
      read -rp "   Key ID [$ASC_KEY_ID_GUESS]: " ASC_KEY_ID
      ASC_KEY_ID="${ASC_KEY_ID:-$ASC_KEY_ID_GUESS}"
      read -rp "   Issuer ID (uuid): " ASC_ISSUER_ID
      [ -n "$ASC_ISSUER_ID" ] || die "issuer id is required alongside the key"
      base64 -i "$ASC_KEY_PATH" | gh secret set APPLE_ASC_KEY_BASE64 --repo "$REPO"
      printf '%s' "$ASC_KEY_ID"    | gh secret set APPLE_ASC_KEY_ID    --repo "$REPO"
      printf '%s' "$ASC_ISSUER_ID" | gh secret set APPLE_ASC_ISSUER_ID --repo "$REPO"
      echo "   TestFlight upload configured"
    fi
  fi
fi

step "Done"
cat <<EOF
   Set: APPLE_CERT_P12_BASE64, APPLE_CERT_P12_PASSWORD, APPLE_SIGNING_IDENTITY,
        APPLE_TEAM_ID, APPLE_ID, APPLE_APP_SPECIFIC_PASSWORD

   APPLE_KEYCHAIN_PASSWORD is deliberately not set — the workflow generates a
   throwaway one per job.

   What these secrets cover:
     VibeCoder    signed + notarized
     VibeAIChat   signed + notarized
     VibeDesk     signed + notarized
     vibecli      signed + notarized (ticket is fetched online; a bare
                  binary cannot carry a stapled ticket)
     watchOS      $WATCH_STATE
     iOS          $IOS_STATE
     Android      unsigned by design (sideload)

   The next release fails rather than publishing an ad-hoc bundle: each app job
   now verifies its .app after building.

   Verify any downloaded artifact:

     for a in VibeCoder VibeAIChat VibeDesk; do
       codesign -dv --verbose=2 "/Applications/\$a.app" 2>&1 | grep -E 'Authority|Signature'
       spctl -a -vvv -t exec "/Applications/\$a.app"
     done

   Expect "Authority=Developer ID Application: ..." and "accepted".
   "Signature=adhoc" or "rejected" means the cert was not picked up — check the
   job log for "APPLE_CERT_P12_BASE64 not set".
EOF
