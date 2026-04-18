#!/usr/bin/env node
/**
 * audit-design-system.mjs — scan every panel in vibeui/src/components/ for
 * violations of the design-system rules documented in vibeui/design-system/
 * and required by AGENTS.md.
 *
 * Run from vibeui/:  node scripts/audit-design-system.mjs
 * Or:                npm run audit:design
 *
 * The script is intentionally pessimistic — it will surface false positives
 * for legitimate uses (e.g. hex colors inside Monaco theme blobs, drop-zone
 * dashed borders that span style boundaries). Use the report as a punch list,
 * not a gate.
 */
import { readFileSync, readdirSync, writeFileSync, statSync } from "node:fs";
import { join, basename } from "node:path";

const COMPONENTS_DIR = new URL("../src/components/", import.meta.url).pathname;
const REPORT_PATH = new URL("../design-system-audit.md", import.meta.url).pathname;

// ── Rules ────────────────────────────────────────────────────────────────────

/** A rule returns one violation object per occurrence (or [] for clean files). */
const RULES = [
  {
    id: "hex-color",
    severity: "high",
    label: "Hard-coded hex color in inline style",
    test: (src) => {
      // Match `#abc` or `#abcdef` (or 8-digit) inside style={…} blocks only.
      // We look at the file globally — false positives possible, but the false-
      // negative cost (a missed hex) is higher.
      const styleBlocks = src.match(/style\s*=\s*\{[\s\S]*?\}/g) ?? [];
      let count = 0;
      for (const block of styleBlocks) {
        // Skip CSS-var fallbacks like var(--btn-primary-fg, #fff)
        // and HTML numeric entities like &#9650;
        const stripped = block
          .replace(/var\([^)]*\)/g, "")
          .replace(/&#[0-9]+;/g, "");
        const matches = stripped.match(/#[0-9a-fA-F]{3,8}\b/g) ?? [];
        count += matches.length;
      }
      return count;
    },
  },
  {
    id: "div-onclick",
    severity: "med",
    label: "<div onClick> instead of <button> (a11y)",
    test: (src) => {
      // Match the FULL <div ...> opening tag (handle JSX braces / strings).
      let count = 0;
      let i = 0;
      while (i < src.length) {
        const tagStart = src.indexOf("<div", i);
        if (tagStart < 0) break;
        // Find tag end at brace depth 0
        let depth = 0;
        let inStr = null;
        let end = -1;
        for (let j = tagStart + 4; j < src.length; j++) {
          const c = src[j];
          if (inStr) {
            if (c === "\\") { j++; continue; }
            if (c === inStr) inStr = null;
            continue;
          }
          if (c === '"' || c === "'" || c === "`") { inStr = c; continue; }
          if (c === "{") depth++;
          else if (c === "}") depth--;
          else if (c === ">" && depth === 0) { end = j; break; }
        }
        if (end < 0) break;
        const tag = src.slice(tagStart, end + 1);
        i = end + 1;
        if (!/\bonClick=/.test(tag)) continue;
        if (/\brole=/.test(tag) && /\btabIndex/.test(tag)) continue;
        count++;
      }
      return count;
    },
  },
  {
    id: "height-100",
    severity: "med",
    label: "height: 100% (rule 1: use flex: 1, minHeight: 0)",
    test: (src) => {
      const re = /height:\s*['"]?100%/g;
      return (src.match(re) ?? []).length;
    },
  },
  {
    id: "no-panel-container",
    severity: "high",
    label: "Missing panel-container root class",
    test: (src) => {
      // Skip composites and helper sub-components — they're allowed not to be panels.
      if (!/export\s+(default\s+)?(function|const)\s+\w*Panel\b/.test(src)) return 0;
      return src.includes("panel-container") ? 0 : 1;
    },
  },
  {
    id: "loading-text-no-class",
    severity: "low",
    label: "'Loading…' text not wrapped in .panel-loading",
    test: (src) => {
      // Heuristic: the strings `Loading…` / `Loading...` appear but the panel-loading class is never used.
      const hasLoadingText = /Loading[…\.]/.test(src);
      const hasLoadingClass = /className\s*=\s*["'][^"']*\bpanel-loading\b/.test(src);
      return hasLoadingText && !hasLoadingClass ? 1 : 0;
    },
  },
  {
    id: "empty-text-no-class",
    severity: "low",
    label: "Empty/no-items text not wrapped in .panel-empty",
    test: (src) => {
      const hasEmptyText = /No\s+(items|results|data|annotations|tokens|imports|sessions|tasks|jobs|messages)\b/i.test(src);
      const hasEmptyClass = /className\s*=\s*["'][^"']*\bpanel-empty\b/.test(src);
      return hasEmptyText && !hasEmptyClass ? 1 : 0;
    },
  },
  {
    id: "inline-button-style",
    severity: "med",
    label: "<button> with inline style instead of panel-btn class",
    test: (src) => {
      const buttonTags = src.match(/<button\b[^>]*>/g) ?? [];
      let count = 0;
      for (const tag of buttonTags) {
        const hasInlineStyle = /\sstyle\s*=\s*\{/.test(tag);
        // Accept className="..." | className='...' | className={`...`} | className={"..."}
        const hasPanelBtn = /className\s*=\s*(?:["'`{][^"'`}]*\bpanel-btn\b|\{`[^`]*\bpanel-btn\b)/.test(tag);
        if (hasInlineStyle && !hasPanelBtn) count++;
      }
      return count;
    },
  },
  {
    id: "non-grid-spacing",
    severity: "low",
    label: "Non-4px-grid spacing literal in style (e.g. padding: 13px)",
    test: (src) => {
      // Look for px values inside style blocks that aren't on the 4px grid.
      const styleBlocks = src.match(/style\s*=\s*\{[\s\S]*?\}/g) ?? [];
      let count = 0;
      const allowed = new Set([0, 1, 2, 3, 4, 8, 12, 16, 20, 24, 28, 32, 40, 48, 56, 64]);
      for (const block of styleBlocks) {
        // Only look at padding / margin / gap properties.
        const propRe = /(?:padding|margin|gap)(?:Top|Right|Bottom|Left|Block|Inline)?\s*:\s*['"]?([^,'"\}\)]+)['"]?/g;
        let m;
        while ((m = propRe.exec(block)) !== null) {
          const value = m[1];
          // Extract integer px values — skip CSS vars/calc/auto/percentages.
          const pxMatches = value.match(/(\d+)px/g) ?? [];
          for (const pxStr of pxMatches) {
            const n = parseInt(pxStr, 10);
            if (!allowed.has(n)) count++;
          }
        }
      }
      return count;
    },
  },
  {
    id: "raw-tab-bar",
    severity: "low",
    label: "Custom tab styling instead of .panel-tab-bar / .panel-tab",
    test: (src) => {
      // Heuristic: file talks about tabs (a tabStyle helper / `setActiveTab`) but never uses panel-tab-bar.
      const looksTabby = /(tabStyle|setActiveTab|activeTab|tab\s*===?)/.test(src);
      const usesClass = /className\s*=\s*["'][^"']*\bpanel-tab/.test(src);
      return looksTabby && !usesClass ? 1 : 0;
    },
  },
  {
    id: "localstorage-credential",
    severity: "high",
    label: "localStorage usage that may persist credentials",
    test: (src) => {
      const re = /localStorage\.(setItem|getItem|removeItem)\s*\(\s*["']([^"']+)["']/g;
      let count = 0, m;
      while ((m = re.exec(src)) !== null) {
        const key = m[2].toLowerCase();
        if (/(token|key|secret|password|auth|cred|api[_-]?key)/.test(key)) count++;
      }
      return count;
    },
  },
];

const SEVERITY_RANK = { high: 3, med: 2, low: 1 };

// ── Scan ─────────────────────────────────────────────────────────────────────

const files = readdirSync(COMPONENTS_DIR)
  .filter((f) => f.endsWith("Panel.tsx"))
  .map((f) => join(COMPONENTS_DIR, f))
  .filter((p) => statSync(p).isFile())
  .sort();

const results = [];
for (const path of files) {
  const src = readFileSync(path, "utf8");
  const violations = {};
  let total = 0;
  for (const rule of RULES) {
    const n = rule.test(src);
    if (n > 0) {
      violations[rule.id] = n;
      total += n * SEVERITY_RANK[rule.severity];
    }
  }
  results.push({ name: basename(path), path, violations, score: total });
}

// ── Report ───────────────────────────────────────────────────────────────────

const totalsByRule = {};
for (const r of RULES) totalsByRule[r.id] = 0;
let cleanCount = 0;
for (const r of results) {
  if (r.score === 0) cleanCount++;
  for (const [id, n] of Object.entries(r.violations)) totalsByRule[id] += n;
}

const lines = [];
const today = new Date().toISOString().slice(0, 10);
lines.push("# VibeUI Design-System Audit");
lines.push("");
lines.push(`_Generated ${today} by \`vibeui/scripts/audit-design-system.mjs\`._`);
lines.push("");
lines.push(`Scanned **${results.length}** panels in \`vibeui/src/components/*Panel.tsx\`.`);
lines.push("");
lines.push(`- ✅ Clean (zero violations): **${cleanCount}**`);
lines.push(`- ⚠ With violations: **${results.length - cleanCount}**`);
lines.push("");
lines.push("## Violations by rule");
lines.push("");
lines.push("| Severity | Rule | Total occurrences | Affected files |");
lines.push("|---|---|---:|---:|");
for (const rule of RULES) {
  const total = totalsByRule[rule.id];
  const affected = results.filter((r) => r.violations[rule.id]).length;
  if (total === 0) continue;
  const sev = rule.severity === "high" ? "🔴 high" : rule.severity === "med" ? "🟠 med" : "🟡 low";
  lines.push(`| ${sev} | ${rule.label} | ${total} | ${affected} |`);
}
lines.push("");

lines.push("## Worst offenders (top 30 by weighted score)");
lines.push("");
lines.push("Score = sum of (occurrences × severity-weight). High=3, Med=2, Low=1.");
lines.push("");
lines.push("| Score | Panel | High | Med | Low |");
lines.push("|---:|---|---:|---:|---:|");
const ranked = [...results].filter((r) => r.score > 0).sort((a, b) => b.score - a.score).slice(0, 30);
for (const r of ranked) {
  let h = 0, m = 0, l = 0;
  for (const rule of RULES) {
    const n = r.violations[rule.id] ?? 0;
    if (rule.severity === "high") h += n;
    else if (rule.severity === "med") m += n;
    else l += n;
  }
  lines.push(`| ${r.score} | \`${r.name}\` | ${h} | ${m} | ${l} |`);
}
lines.push("");

lines.push("## Per-panel detail (panels with violations only)");
lines.push("");
const dirty = results.filter((r) => r.score > 0).sort((a, b) => b.score - a.score);
for (const r of dirty) {
  lines.push(`### \`${r.name}\` — score ${r.score}`);
  lines.push("");
  lines.push("| Rule | Occurrences |");
  lines.push("|---|---:|");
  for (const rule of RULES) {
    const n = r.violations[rule.id];
    if (!n) continue;
    lines.push(`| ${rule.label} | ${n} |`);
  }
  lines.push("");
}

writeFileSync(REPORT_PATH, lines.join("\n"));

// ── Console summary ──────────────────────────────────────────────────────────

const fmt = (n) => String(n).padStart(5);
console.log("");
console.log(`Scanned ${results.length} panels`);
console.log(`  ✅ Clean:           ${cleanCount}`);
console.log(`  ⚠  With violations: ${results.length - cleanCount}`);
console.log("");
console.log("By rule:");
for (const rule of RULES) {
  const total = totalsByRule[rule.id];
  if (total === 0) continue;
  const affected = results.filter((r) => r.violations[rule.id]).length;
  console.log(`  [${rule.severity.padEnd(4)}] ${fmt(total)} occurrences in ${fmt(affected)} files — ${rule.label}`);
}
console.log("");
console.log(`Full report: ${REPORT_PATH}`);
console.log("");
console.log("Top 10 offenders:");
for (const r of ranked.slice(0, 10)) {
  console.log(`  ${fmt(r.score)}  ${r.name}`);
}
