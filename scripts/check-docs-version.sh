#!/usr/bin/env bash
#
# Fail when user-facing docs advertise a release other than the current one.
#
# Version strings rot silently. The published site said "What's new in 0.5.5"
# three releases after 0.5.5 shipped, and the VibeMobile / watchOS / Wear OS
# download tables pointed at v0.5.7 assets after v0.5.8 was tagged. Nothing
# caught it: a stale version renders perfectly, and the version-bump checklist
# named only release.md, CHANGELOG.md and RELEASE.md.
#
# Both checks below are deliberately narrow. Docs are *supposed* to name old
# versions in three cases, and none of them are flagged:
#   - minimum-version claims  — "vibecli --version returns 0.5.1+"
#   - historical notes        — "introduced in v0.5.5", "raised from 12.0 in v0.5.5"
#   - the changelog           — every past release, by definition
#
# Usage: scripts/check-docs-version.sh
set -euo pipefail

cd "$(dirname "$0")/.."

VERSION="$(sed -n '/^\[workspace\.package\]/,/^\[/p' Cargo.toml \
  | sed -n 's/^version = "\(.*\)"/\1/p' | head -1)"

if [[ -z "$VERSION" ]]; then
  echo "check-docs-version: could not read version from [workspace.package] in Cargo.toml" >&2
  exit 1
fi

echo "Current version: $VERSION"
failed=0

# ── 1. Release-asset links must point at the current tag ────────────────────
# A link to a previous tag still resolves, which is what makes this rot
# invisible: users download a working-but-old build and never report it.
#
# CHANGELOG.md and release.md are excluded wholesale — both are archives that
# keep per-release sections on purpose. release.md's *current* section is
# checked separately below.
stale_links="$(grep -rn "releases/download/v" docs --include='*.md' \
  | grep -v "^docs/CHANGELOG.md:" \
  | grep -v "^docs/release.md:" \
  | grep -v "releases/download/v${VERSION}/" || true)"

if [[ -n "$stale_links" ]]; then
  echo
  echo "FAIL: download links do not point at v${VERSION}:" >&2
  echo "$stale_links" >&2
  failed=1
fi

# ── 1b. release.md's "Latest" section, up to the next release heading ───────
latest_section="$(awk '/^## v.* — Latest$/{inside=1; next} /^## /{inside=0} inside' docs/release.md)"

if [[ -z "$latest_section" ]]; then
  echo "FAIL: docs/release.md has no '## v<version> — Latest' section" >&2
  failed=1
else
  stale_latest="$(printf '%s\n' "$latest_section" \
    | grep -n "releases/download/v" \
    | grep -v "releases/download/v${VERSION}/" || true)"
  if [[ -n "$stale_latest" ]]; then
    echo
    echo "FAIL: docs/release.md 'Latest' section links to a tag other than v${VERSION}:" >&2
    echo "$stale_latest" >&2
    failed=1
  fi
fi

# ── 2. Pages that state the current release must state *this* one ───────────
# Each entry is "<file>|<pattern with VERSION substituted>|<what it is>".
# Add a row here whenever a page starts advertising the latest version.
checks=(
  "docs/release.md|## v${VERSION} — Latest|release-notes heading"
  "docs/CHANGELOG.md|## [${VERSION}]|changelog entry"
  "docs/index.md|v${VERSION}|release-notes nav row"
  "docs/vibecoder.md|### What's new in ${VERSION}|VibeCoder what's-new heading"
  "docs/vibecli.md|### What's new in ${VERSION}|VibeCLI what's-new heading"
  "docs/vibemobile.md|## What's new in ${VERSION}|VibeMobile what's-new heading"
  "docs/quickstart.md|VibeCLI v${VERSION}|sample startup banner"
  "docs/api-reference.md|\"version\": \"${VERSION}\"|/health sample response"
  "docs/connectivity.md|\"daemon_version\": \"${VERSION}\"|beacon sample payload"
)

for entry in "${checks[@]}"; do
  IFS='|' read -r file pattern label <<<"$entry"
  if [[ ! -f "$file" ]]; then
    echo "FAIL: $file is missing (expected to carry the $label)" >&2
    failed=1
  elif ! grep -qF -- "$pattern" "$file"; then
    echo "FAIL: $file — $label should read '$pattern'" >&2
    failed=1
  fi
done

if [[ $failed -ne 0 ]]; then
  echo
  echo "Docs still advertise an older release. Update them, or update this" >&2
  echo "script if a page legitimately stopped tracking the current version." >&2
  exit 1
fi

echo "All checked docs reference v${VERSION}."
