#!/bin/bash
# Build PDF version of VibeCody whitepapers
#
# Strategy: Markdown → HTML (pandoc) → PDF (browser print)
# The HTML version is self-contained and can also be opened directly.
#
# Requirements: pandoc 3.x
#
# Usage:
#   cd docs/whitepapers
#   ./build-pdf.sh                     # Build all whitepapers
#   ./build-pdf.sh openclaw-comparison # Build specific whitepaper

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

build_html() {
    local src="$1"
    local name="${src%.md}"
    local html="${name}.html"
    local tmp="${name}-body.md"

    echo "Building ${html}..."

    # Strip YAML frontmatter
    awk 'BEGIN{skip=0} /^---$/{skip++; next} skip>=2{print}' "$src" > "$tmp"

    pandoc "$tmp" \
        -o "$html" \
        --standalone \
        --embed-resources \
        --metadata title="VibeCody vs OpenClaw & AI Agent Alternatives" \
        --toc \
        --toc-depth=2 \
        --columns=80 \
        --css="${SCRIPT_DIR}/whitepaper.css" \
        -f markdown+pipe_tables+strikeout 2>&1 || true

    rm -f "$tmp"

    if [ -f "$html" ]; then
        local size
        size=$(du -h "$html" | cut -f1)
        echo "  -> ${html} (${size})"
        echo ""
        echo "  To generate PDF:"
        echo "    Option A: Open ${html} in Chrome → Print → Save as PDF"
        echo "    Option B: npx puppeteer-pdf ${html} -o ${name}.pdf"
        echo "    Option C: brew install weasyprint && weasyprint ${html} ${name}.pdf"
    else
        echo "  -> FAILED"
        return 1
    fi
}

# Also try direct PDF if weasyprint or wkhtmltopdf are available
try_pdf() {
    local html="$1"
    local pdf="${html%.html}.pdf"

    if command -v weasyprint &>/dev/null; then
        echo "  Generating PDF via weasyprint..."
        weasyprint "$html" "$pdf" 2>&1
        [ -f "$pdf" ] && echo "  -> ${pdf} ($(du -h "$pdf" | cut -f1))"
    elif command -v wkhtmltopdf &>/dev/null; then
        echo "  Generating PDF via wkhtmltopdf..."
        wkhtmltopdf --enable-local-file-access "$html" "$pdf" 2>&1
        [ -f "$pdf" ] && echo "  -> ${pdf} ($(du -h "$pdf" | cut -f1))"
    fi
}

if [ $# -gt 0 ]; then
    build_html "${1}.md"
    try_pdf "${1}.html"
else
    for md in *-comparison.md; do
        [ -f "$md" ] || continue
        build_html "$md"
        try_pdf "${md%.md}.html"
    done
fi

echo ""
echo "Done."
