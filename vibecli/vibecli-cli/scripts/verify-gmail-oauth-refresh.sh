#!/usr/bin/env bash
# verify-gmail-oauth-refresh.sh
#
# Local verifier for the Gmail OAuth refresh flow shipped in commit 06715180
# (feat(email): OAuth refresh-token flow for Gmail + Outlook).
#
# Designed to run once ~2 weeks after the feature ships (e.g. 2026-05-17), to
# answer three questions:
#   1. Did the in-process refresh actually fire and persist a new token?
#   2. Are the refresh credentials configured at all?  (If not, the flow is
#      dormant and (1) being negative tells us nothing.)
#   3. Is the answer good news or worth investigating?
#
# It does NOT decrypt any token — the ProfileStore values are encrypted with
# ChaCha20-Poly1305 and the script has no need for the plaintext.  It only
# reads the row's `updated_at` column (epoch ms) and compares it against the
# date the feature shipped.
#
# Usage:
#   bash scripts/verify-gmail-oauth-refresh.sh                  # default
#   VIBECLI_PROFILE_DB=/path/to/profile_settings.db bash ...    # alternate DB
#   VERIFIER_SHIPPED_AFTER_MS=1778572800000 bash ...            # alternate cutoff
#
# Exit codes:
#   0  refresh credentials configured AND (refresh fired) → green light
#   1  refresh credentials configured but no refresh observed yet
#   2  refresh credentials not configured (flow is dormant — informational)
#   3  ProfileStore not found or unreadable
#   4  sqlite3 binary missing

set -uo pipefail

DB="${VIBECLI_PROFILE_DB:-$HOME/.vibecli/profile_settings.db}"
# 2026-05-03T00:00:00Z = the day the OAuth refresh shipped on origin/main.
# Override via VERIFIER_SHIPPED_AFTER_MS if the feature shipped on a different
# date for you (forks, downstream rollouts).
SHIPPED_AFTER_MS_DEFAULT=$(date -j -u -f "%Y-%m-%dT%H:%M:%SZ" "2026-05-03T00:00:00Z" "+%s" 2>/dev/null || echo "")
if [ -z "${SHIPPED_AFTER_MS_DEFAULT}" ]; then
  # Fallback for GNU date (Linux).
  SHIPPED_AFTER_MS_DEFAULT=$(date -u -d "2026-05-03T00:00:00Z" "+%s" 2>/dev/null || echo "1778572800")
fi
SHIPPED_AFTER_MS="${VERIFIER_SHIPPED_AFTER_MS:-$((SHIPPED_AFTER_MS_DEFAULT * 1000))}"

ACCESS_KEY="integration.email.gmail_access_token"
REFRESH_KEY="integration.email.gmail_refresh_token"
CLIENT_ID_KEY="integration.email.gmail_oauth_client_id"
CLIENT_SECRET_KEY="integration.email.gmail_oauth_client_secret"

bold()    { printf '\033[1m%s\033[0m\n' "$*"; }
green()   { printf '\033[32m%s\033[0m\n' "$*"; }
yellow()  { printf '\033[33m%s\033[0m\n' "$*"; }
red()     { printf '\033[31m%s\033[0m\n' "$*"; }
neutral() { printf '%s\n' "$*"; }

bold "Gmail OAuth refresh — local verifier"
neutral "  ProfileStore: $DB"
neutral "  Cutoff       : $(date -u -r $((SHIPPED_AFTER_MS / 1000)) "+%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || echo "$SHIPPED_AFTER_MS ms")"
echo

if ! command -v sqlite3 >/dev/null 2>&1; then
  red "✗ sqlite3 binary not found in PATH — cannot inspect ProfileStore."
  exit 4
fi

if [ ! -r "$DB" ]; then
  red "✗ ProfileStore not found or unreadable at $DB"
  neutral "  vibecli either has not run on this account, or the DB is at a custom"
  neutral "  path. Set VIBECLI_PROFILE_DB to point at it and re-run."
  exit 3
fi

# Helper: print the updated_at (epoch ms) for a given provider, or empty.
row_updated_at() {
  local provider="$1"
  sqlite3 "$DB" "SELECT updated_at FROM api_keys WHERE profile_id='default' AND provider='$provider' LIMIT 1;" 2>/dev/null
}

# Helper: 0 if a row exists for the provider, else 1.
row_exists() {
  local provider="$1"
  local n
  n=$(sqlite3 "$DB" "SELECT COUNT(*) FROM api_keys WHERE profile_id='default' AND provider='$provider';" 2>/dev/null || echo 0)
  [ "${n:-0}" -gt 0 ]
}

format_ts_ms() {
  local ms="$1"
  if [ -z "$ms" ]; then printf "(none)"; return; fi
  date -u -r $((ms / 1000)) "+%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || printf "%s ms" "$ms"
}

# ── (2) Is the refresh flow even configured? ─────────────────────────────────
configured=0
if row_exists "$REFRESH_KEY" && row_exists "$CLIENT_ID_KEY" && row_exists "$CLIENT_SECRET_KEY"; then
  configured=1
  green  "✓ Refresh credentials configured (refresh_token + client_id + client_secret all present)."
else
  yellow "○ Refresh credentials NOT fully configured:"
  for k in "$REFRESH_KEY" "$CLIENT_ID_KEY" "$CLIENT_SECRET_KEY"; do
    if row_exists "$k"; then
      neutral "    ✓ $k"
    else
      neutral "    ✗ $k missing"
    fi
  done
  neutral "  Without all three, EmailClient::can_refresh() returns false and the"
  neutral "  refresh flow is dormant — Gmail will keep returning 401 every hour."
  neutral "  Configure under Settings → Integrations → Email."
fi

# ── (1) Did refresh fire? ────────────────────────────────────────────────────
access_updated_ms=$(row_updated_at "$ACCESS_KEY")
echo
if [ -z "$access_updated_ms" ]; then
  yellow "○ No gmail_access_token row in ProfileStore — Gmail integration not connected."
  refresh_fired=0
elif [ "$access_updated_ms" -gt "$SHIPPED_AFTER_MS" ]; then
  green "✓ gmail_access_token last persisted at $(format_ts_ms "$access_updated_ms")"
  neutral "  (later than the shipping cutoff $(format_ts_ms "$SHIPPED_AFTER_MS"))"
  neutral "  Either you re-pasted the token by hand, OR refresh_access_token() ran"
  neutral "  and persisted a fresh one. The two are indistinguishable from this"
  neutral "  signal alone — but if you have not touched Settings since the feature"
  neutral "  shipped, this IS the refresh flow."
  refresh_fired=1
else
  yellow "○ gmail_access_token last persisted at $(format_ts_ms "$access_updated_ms")"
  neutral "  (BEFORE the shipping cutoff $(format_ts_ms "$SHIPPED_AFTER_MS"))"
  neutral "  Either the Gmail integration has not been used since the refresh"
  neutral "  flow shipped, or refresh has never fired."
  refresh_fired=0
fi

# ── (3) Verdict ──────────────────────────────────────────────────────────────
echo
bold "Verdict"
if [ "$configured" -eq 1 ] && [ "$refresh_fired" -eq 1 ]; then
  green "  PASS — refresh is configured AND a fresh access token has been persisted."
  neutral "  No action needed. The 401-every-hour trap is closed."
  exit 0
elif [ "$configured" -eq 1 ] && [ "$refresh_fired" -eq 0 ]; then
  yellow "  INVESTIGATE — refresh creds are set, but the access-token row has not"
  neutral "  been updated since the feature shipped. Possible causes:"
  neutral "    • Gmail integration genuinely unused for ~2 weeks (idle account)."
  neutral "    • Daemon never hit a 401, so the refresh path was never invoked"
  neutral "      (unusual — Gmail tokens expire after ~60 minutes of issuance)."
  neutral "    • A bug in get_json/post_json/patch_json's 401-retry loop."
  neutral "  Try /email inbox in a vibecli session and re-run this script."
  exit 1
elif [ "$configured" -eq 0 ] && [ "$refresh_fired" -eq 1 ]; then
  yellow "  PARTIAL — the access token has rotated, but refresh creds are missing."
  neutral "  This means you re-pasted the token manually. The flow is still dormant."
  neutral "  Add the refresh-token + client-id + client-secret in Settings → Integrations → Email."
  exit 2
else
  yellow "  DORMANT — refresh creds are missing AND no rotation observed."
  neutral "  Add refresh credentials to enable automatic refresh — without them"
  neutral "  Gmail will keep failing every ~60 minutes with HTTP 401."
  exit 2
fi
