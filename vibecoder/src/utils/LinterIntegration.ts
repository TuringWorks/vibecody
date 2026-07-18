/**
 * LinterIntegration — run language-appropriate linters after agent writes a file.
 *
 * Currently supported:
 * - TypeScript/JavaScript: eslint (if installed)
 * - Rust: cargo check (not clippy, since clippy is slow)
 * - Python: flake8 (if installed)
 * - Go: go vet
 *
 * Delegates to the `run_linter` Tauri command which runs the linter process
 * and parses its JSON/text output into LintError records.
 */

import { invoke } from "@tauri-apps/api/core";

export interface LintError {
  line: number;
  col: number;
  severity: "error" | "warning";
  message: string;
  rule?: string;
}

export interface LintResult {
  filePath: string;
  errors: LintError[];
  warnings: LintError[];
  /** Raw output from the linter, for displaying to the user. */
  rawOutput: string;
  /** Whether the linter ran successfully (vs. not being installed). */
  linterAvailable: boolean;
}

/**
 * Determine linter from file extension.
 * Returns null if the file type has no supported linter.
 */
export function linterForFile(filePath: string): string | null {
  const ext = filePath.split(".").pop()?.toLowerCase() ?? "";
  switch (ext) {
    case "ts":
    case "tsx":
    case "js":
    case "jsx":
    case "mjs":
    case "cjs":
      return "eslint";
    case "rs":
      return "cargo-check";
    case "py":
      return "flake8";
    case "go":
      return "go-vet";
    default:
      return null;
  }
}

/**
 * Run the appropriate linter for `filePath`.
 * Returns a `LintResult` — never throws.
 */
export async function runLinter(filePath: string): Promise<LintResult> {
  const linter = linterForFile(filePath);
  if (!linter) {
    return { filePath, errors: [], warnings: [], rawOutput: "", linterAvailable: false };
  }

  try {
    const result = await invoke<{
      errors: LintError[];
      warnings: LintError[];
      raw_output: string;
      linter_available: boolean;
    }>("run_linter", { filePath, linter });

    return {
      filePath,
      errors: result.errors,
      warnings: result.warnings,
      rawOutput: result.raw_output,
      linterAvailable: result.linter_available,
    };
  } catch {
    return { filePath, errors: [], warnings: [], rawOutput: "", linterAvailable: false };
  }
}

/**
 * Format a LintResult as an agent context injection string.
 * Returns null if there are no errors/warnings.
 */
export function formatLintForAgent(result: LintResult): string | null {
  if (!result.linterAvailable) return null;
  const total = result.errors.length + result.warnings.length;
  if (total === 0) return null;

  const lines: string[] = [
    `[Linter] Found ${result.errors.length} error(s) and ${result.warnings.length} warning(s) in ${result.filePath.split("/").pop()}:`,
  ];

  for (const err of [...result.errors, ...result.warnings].slice(0, 10)) {
    const sev = err.severity === "error" ? "❌" : "⚠️";
    const rule = err.rule ? ` (${err.rule})` : "";
    lines.push(`  ${sev} Line ${err.line}:${err.col} — ${err.message}${rule}`);
  }

  if (total > 10) {
    lines.push(`  ...and ${total - 10} more.`);
  }

  return lines.join("\n");
}
