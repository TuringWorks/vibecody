import js from "@eslint/js";
import tseslint from "typescript-eslint";
import reactHooks from "eslint-plugin-react-hooks";
import noUnsanitized from "eslint-plugin-no-unsanitized";

export default tseslint.config(
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    plugins: {
      "react-hooks": reactHooks,
      "no-unsanitized": noUnsanitized,
    },
    rules: {
      // Classic react-hooks rules only — the v7 plugin's recommended config
      // includes experimental React Compiler rules that flag valid patterns
      // in standard React 18 codebases (refs during render, setState-in-effect, etc.)
      "react-hooks/rules-of-hooks": "error",
      "react-hooks/exhaustive-deps": "warn",
      // Allow unused vars prefixed with _ (common React pattern)
      "@typescript-eslint/no-unused-vars": [
        "warn",
        { argsIgnorePattern: "^_", varsIgnorePattern: "^_", caughtErrorsIgnorePattern: "^_" },
      ],
      // Allow explicit any in specific cases (Tauri interop, third-party libs)
      "@typescript-eslint/no-explicit-any": "warn",
      // Prefer const assertions
      "prefer-const": "error",
      // No console.log in production (warn, not error — allow console.error/warn)
      "no-console": ["warn", { allow: ["warn", "error"] }],
      // DREAD #10 — block raw HTML injection sinks. dangerouslySetInnerHTML
      // is an XSS pivot to the full 1,045-cmd Tauri surface (see threat
      // model §6 B2). The rule treats DOMPurify.sanitize() output as safe;
      // any other path requires an explicit eslint-disable comment naming
      // the safety argument.
      "no-unsanitized/method": "error",
      "no-unsanitized/property": "error",
    },
  },
  {
    // Tests legitimately build DOM nodes from literal HTML fixtures via
    // `c.innerHTML = "<button>…</button>"`. They run in jsdom only and never
    // touch user-controlled input — the no-unsanitized rule would force
    // every fixture to be replaced with verbose `createElement` chains.
    ignores: ["dist/", "node_modules/", "src-tauri/", "src/**/__tests__/**"],
  },
);
