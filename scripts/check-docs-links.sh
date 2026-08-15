#!/usr/bin/env bash
#
# Every internal docs link must point at a page that exists.
#
# The site is a GitHub Pages *project* site, served under /vibecody/. A link
# written as `](/ghost-text/)` therefore resolves to the domain root —
# https://turingworks.github.io/ghost-text/ — which 404s, while the page itself
# sits perfectly happily at /vibecody/ghost-text/. The page renders, the link
# looks right in the source, and only clicking it reveals the problem. Fifteen
# links shipped that way across five files.
#
# Two checks, because the two failure modes are different:
#
#   1. A root-absolute link that forgets the /vibecody prefix.
#   2. A prefixed link whose target no page actually declares.
#
# Deliberately NOT checked: `*.md` links inside docs/design/** and other files
# with no `permalink:`. Those are not published pages — they are read on GitHub,
# where `./README.md` is exactly right. Flagging them would be noise, and a
# noisy check gets skipped.

set -euo pipefail

cd "$(dirname "$0")/.."

BASE="/vibecody"
fail=0

# ── 1. Root-absolute links missing the project prefix ────────────────────────
missing=$(grep -rnoE '\]\(/[A-Za-z0-9][A-Za-z0-9/_.-]*\)' docs/ 2>/dev/null \
  | grep -v "](${BASE}/" || true)

if [ -n "$missing" ]; then
  echo "Internal links missing the ${BASE} prefix (they resolve to the domain root and 404):"
  echo "$missing" | sed 's/^/  /'
  echo
  fail=1
fi

# ── 2. Prefixed links pointing at a page nobody declares ─────────────────────
# Collect declared permalinks, then every prefixed link, and diff them.
permalinks=$(grep -rhE '^permalink:' docs/ 2>/dev/null \
  | sed -E 's/^permalink:[[:space:]]*//; s#/+$##' | sort -u)

links=$(grep -rhoE "\]\(${BASE}/[A-Za-z0-9/_.-]*\)" docs/ 2>/dev/null \
  | sed -E "s#^\]\(${BASE}##; s#\)\$##; s#/+\$##" \
  | grep -vE '\.(svg|png|jpg|jpeg|gif|pdf|txt|json|yml)$' \
  | sort -u || true)

dangling=""
while IFS= read -r link; do
  [ -z "$link" ] && continue
  target="${link:-/}"
  if ! printf '%s\n' "$permalinks" | grep -qxF "$target"; then
    dangling+="  ${BASE}${link}"$'\n'
  fi
done <<< "$links"

if [ -n "$dangling" ]; then
  echo "Internal links whose target page does not exist:"
  printf '%s' "$dangling"
  echo
  fail=1
fi

if [ "$fail" -ne 0 ]; then
  echo "Fix the links above, or add the missing page."
  exit 1
fi

echo "docs links OK — $(printf '%s\n' "$links" | grep -c . ) internal links, all resolving."
